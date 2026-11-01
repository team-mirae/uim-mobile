//! JA kana-kanji conversion (SPEC §6, §8 M6), ported from jis-moonshot's converter.
//!
//! A word lattice over the hiragana, searched for the cheapest paths: each word's cost plus the
//! connection cost between neighbouring words' parts of speech (a bigram over Mozc's POS ids).
//! The data is `data/kanji.bin`, built from Mozc's open-source dictionary by
//! `tools/kanji/build_kanji.py` (format there). Platform-independent and allocation-bounded:
//! input longer than [`MAX_INPUT`] kana isn't converted, and the lattice keeps a fixed beam per
//! position, so a keystroke never costs more than a few milliseconds.

/// The conversion data didn't parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadModel;

/// Longest reading looked up as one word, in kana (the data has none longer).
const MAX_WORD: usize = 8;
/// Longest input converted, in kana: a staged segment longer than this stays kana.
pub const MAX_INPUT: usize = 40;
/// A kana no word covers, kept as typed: about as costly as a rare word.
const UNKNOWN_COST: i32 = 10_000;
/// Paths kept at each position, and per right-hand part of speech there.
const BEAM: usize = 48;
const PER_POS: usize = 2;
/// Predecessors tried per word: the cheapest few, so the second-best path differs early too.
const PREDECESSORS: usize = 2;
/// The connection matrix's cost quantum (build_kanji.py's STEP).
const STEP: i32 = 64;
/// The beginning / end of sentence part of speech.
const BOS: u16 = 0;

#[derive(Clone, Copy, Debug)]
struct Entry {
    left: u16,
    right: u16,
    cost: u16,
    /// [`SAME`], [`KATAKANA`] or an offset into `surfaces`.
    surface: u32,
}

const SAME: u32 = u32::MAX;
const KATAKANA: u32 = u32::MAX - 1;

/// The conversion data, parsed once and shared by every engine (behind an `Arc`).
pub struct Kanji {
    ids: usize,
    unknown: u16,
    /// `ids` × `ids` connection costs ÷ [`STEP`]: row = the left word's right id, column = the
    /// right word's left id.
    matrix: Vec<u8>,
    /// Readings, sorted by bytes, concatenated; reading `i` is `readings[at[i]..at[i + 1]]`.
    readings: Vec<u8>,
    at: Vec<u32>,
    /// Reading `i`'s entries are `entries[first[i]..first[i + 1]]`.
    first: Vec<u32>,
    entries: Vec<Entry>,
    surfaces: Vec<u8>,
}

impl std::fmt::Debug for Kanji {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Kanji({} readings, {} entries)", self.at.len().saturating_sub(1), self.entries.len())
    }
}

/// A little-endian reader that fails instead of panicking.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], BadModel> {
        let end = self.at.checked_add(n).ok_or(BadModel)?;
        let bytes = self.data.get(self.at..end).ok_or(BadModel)?;
        self.at = end;
        Ok(bytes)
    }
    fn u8(&mut self) -> Result<u8, BadModel> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, BadModel> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, BadModel> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// One path through the lattice so far.
#[derive(Clone, Copy)]
struct Node {
    cost: i32,
    right: u16,
    /// The node before it (`u32::MAX`: the start).
    prev: u32,
    /// Where its word starts in the input (in kana) and how long it is.
    start: u8,
    len: u8,
    /// The word's surface, as in [`Entry::surface`]; [`SAME`] for an unknown kana.
    surface: u32,
}

impl Kanji {
    /// Parses `data/kanji.bin`. Malformed data is an error, never a panic.
    pub fn parse(data: &[u8]) -> Result<Kanji, BadModel> {
        let mut r = Reader { data, at: 0 };
        if r.take(4)? != b"JKKJ" || r.u32()? != 1 {
            return Err(BadModel);
        }
        let ids = r.u32()? as usize;
        let unknown = r.u32()?;
        let runs = r.u32()? as usize;
        let count = r.u32()? as usize;
        let entry_count = r.u32()? as usize;
        let surface_bytes = r.u32()? as usize;
        if ids == 0 || ids > u16::MAX as usize || unknown as usize >= ids {
            return Err(BadModel);
        }
        let cells = ids * ids;
        let mut matrix = Vec::with_capacity(cells);
        for _ in 0..runs {
            let value = r.u8()?;
            let length = r.u8()? as usize;
            if length == 0 || matrix.len() + length > cells {
                return Err(BadModel);
            }
            matrix.extend(std::iter::repeat_n(value, length));
        }
        if matrix.len() != cells {
            return Err(BadModel);
        }
        let mut readings = Vec::new();
        let mut at = Vec::with_capacity(count + 1);
        let mut counts = Vec::with_capacity(count);
        let mut previous = 0..0;
        for _ in 0..count {
            let shared = r.u8()? as usize;
            let rest = r.u8()? as usize;
            if shared > previous.len() {
                return Err(BadModel);
            }
            let start = readings.len();
            readings.extend_from_within(previous.start..previous.start + shared);
            readings.extend_from_slice(r.take(rest)?);
            let reading = &readings[start..];
            // Sorted, distinct and UTF-8, or the binary search and the slicing below go wrong.
            if std::str::from_utf8(reading).is_err() || (start > 0 && readings[previous.clone()] >= *reading) {
                return Err(BadModel);
            }
            at.push(start as u32);
            counts.push(r.u8()? as usize);
            previous = start..readings.len();
        }
        at.push(readings.len() as u32);
        if counts.iter().sum::<usize>() != entry_count {
            return Err(BadModel);
        }
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let entry = Entry { left: r.u16()?, right: r.u16()?, cost: r.u16()?, surface: r.u32()? };
            if entry.left as usize >= ids || entry.right as usize >= ids {
                return Err(BadModel);
            }
            entries.push(entry);
        }
        let surfaces = r.take(surface_bytes)?.to_vec();
        if r.at != data.len() {
            return Err(BadModel);
        }
        let mut first = Vec::with_capacity(count + 1);
        let mut sum = 0u32;
        for c in &counts {
            first.push(sum);
            sum += *c as u32;
        }
        first.push(sum);
        let kanji = Kanji { ids, unknown: unknown as u16, matrix, readings, at, first, entries, surfaces };
        if kanji.entries.iter().any(|e| e.surface < KATAKANA && kanji.surface_at(e.surface).is_none()) {
            return Err(BadModel);
        }
        Ok(kanji)
    }

    fn surface_at(&self, offset: u32) -> Option<&str> {
        let at = offset as usize;
        let len = *self.surfaces.get(at)? as usize;
        std::str::from_utf8(self.surfaces.get(at + 1..at + 1 + len)?).ok()
    }

    fn reading(&self, i: usize) -> &[u8] {
        &self.readings[self.at[i] as usize..self.at[i + 1] as usize]
    }

    /// Every reading with its lowest cost (lower is more common), in byte order.
    pub fn words(&self) -> impl Iterator<Item = (&str, u16)> + '_ {
        (0..self.at.len().saturating_sub(1)).filter_map(move |i| {
            let reading = std::str::from_utf8(&self.readings[self.at[i] as usize..self.at[i + 1] as usize]).ok()?;
            let entries = self.entries.get(self.first[i] as usize..self.first[i + 1] as usize)?;
            Some((reading, entries.iter().map(|e| e.cost).min()?))
        })
    }

    /// The lowest cost of `hiragana` as a whole word (lower is more common), if it is one.
    pub fn word_cost(&self, hiragana: &str) -> Option<u16> {
        self.lookup(hiragana).iter().map(|e| e.cost).min()
    }

    /// The entries of `reading`, if it's a word.
    fn lookup(&self, reading: &str) -> &[Entry] {
        let key = reading.as_bytes();
        let n = self.at.len() - 1;
        let (mut lo, mut hi) = (0, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            match self.reading(mid).cmp(key) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return &self.entries[self.first[mid] as usize..self.first[mid + 1] as usize],
            }
        }
        &[]
    }

    fn connection(&self, right: u16, left: u16) -> i32 {
        self.matrix[right as usize * self.ids + left as usize] as i32 * STEP
    }

    fn text(&self, surface: u32, kana: &[char]) -> String {
        match surface {
            SAME => kana.iter().collect(),
            KATAKANA => crate::kana::to_katakana(&kana.iter().collect::<String>()),
            offset => self.surface_at(offset).unwrap_or_default().to_string(),
        }
    }

    /// Up to `n` conversions of `hiragana`, cheapest first, distinct, each with its cost (lower is
    /// likelier). Empty for an empty or too long input. The all-kana path is among them when it's
    /// cheap enough; callers drop it if they already show the kana.
    pub fn convert(&self, hiragana: &str, n: usize) -> Vec<(String, i32)> {
        let kana: Vec<char> = hiragana.chars().collect();
        let len = kana.len();
        if len == 0 || len > MAX_INPUT || n == 0 {
            return Vec::new();
        }
        let mut nodes: Vec<Node> = Vec::new();
        // ends[i]: the nodes whose word ends at kana i (kept to the beam once i is reached).
        let mut ends: Vec<Vec<u32>> = vec![Vec::new(); len + 1];
        let mut word = String::new();
        for i in 0..len {
            let preds: Vec<(i32, u16, u32)> = if i == 0 {
                vec![(0, BOS, u32::MAX)]
            } else {
                self.prune(&nodes, &mut ends[i]);
                ends[i].iter().map(|&k| (nodes[k as usize].cost, nodes[k as usize].right, k)).collect()
            };
            if preds.is_empty() {
                continue;
            }
            for l in 1..=MAX_WORD.min(len - i) {
                word.clear();
                word.extend(&kana[i..i + l]);
                let unknown = Entry { left: self.unknown, right: self.unknown, cost: 0, surface: SAME };
                let found = self.lookup(&word);
                let extra = (l == 1).then_some(unknown);
                for (e, entry) in found.iter().copied().chain(extra).enumerate() {
                    let cost = if e == found.len() { UNKNOWN_COST } else { entry.cost as i32 };
                    // The cheapest few predecessors.
                    let mut best: [(i32, u32); PREDECESSORS] = [(i32::MAX, u32::MAX); PREDECESSORS];
                    for &(c, right, k) in &preds {
                        let total = c + self.connection(right, entry.left) + cost;
                        if let Some(slot) = best.iter().position(|b| total < b.0) {
                            best[slot..].rotate_right(1);
                            best[slot] = (total, k);
                        }
                    }
                    for &(total, prev) in best.iter().filter(|b| b.0 != i32::MAX) {
                        ends[i + l].push(nodes.len() as u32);
                        nodes.push(Node { cost: total, right: entry.right, prev, start: i as u8, len: l as u8, surface: entry.surface });
                    }
                }
            }
        }
        // The end: each path with its connection to the end of the sentence, cheapest first.
        let mut finals: Vec<(i32, u32)> = ends[len].iter().map(|&k| (nodes[k as usize].cost + self.connection(nodes[k as usize].right, BOS), k)).collect();
        finals.sort();
        let mut out: Vec<(String, i32)> = Vec::new();
        for (cost, mut k) in finals {
            let mut pieces = Vec::new();
            while k != u32::MAX {
                let node = nodes[k as usize];
                let start = node.start as usize;
                pieces.push(self.text(node.surface, &kana[start..start + node.len as usize]));
                k = node.prev;
            }
            pieces.reverse();
            let text = pieces.concat();
            if !out.iter().any(|(t, _)| *t == text) {
                out.push((text, cost));
                if out.len() == n {
                    break;
                }
            }
        }
        out
    }

    /// Keeps the cheapest [`BEAM`] nodes ending here, at most [`PER_POS`] per right-hand part of
    /// speech (what the next word connects to).
    fn prune(&self, nodes: &[Node], ends: &mut Vec<u32>) {
        ends.sort_by_key(|&k| nodes[k as usize].cost);
        let mut kept = Vec::with_capacity(BEAM);
        let mut per: Vec<(u16, usize)> = Vec::new();
        for &k in ends.iter() {
            let right = nodes[k as usize].right;
            let seen = match per.iter_mut().find(|(r, _)| *r == right) {
                Some((_, c)) => {
                    *c += 1;
                    *c
                }
                None => {
                    per.push((right, 1));
                    1
                }
            };
            if seen <= PER_POS {
                kept.push(k);
                if kept.len() == BEAM {
                    break;
                }
            }
        }
        *ends = kept;
    }
}
