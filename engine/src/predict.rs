//! EN predictive (SPEC §3.3): 20 keys, each any letter on its label; a word list and a bigram
//! model resolve the shared keys. Words are stored in lower case; case is only "all caps or not":
//! all caps ranks acronyms first (A·ZQP·I is API), anything else ranks as if lower case.

use std::collections::HashMap;

/// The keys in grid order (rows of 5). Every letter is on exactly one key.
pub const GROUPS: [&str; 20] = [
    "a", "e", "i", "o", "u", //
    "s", "t", "zqp", "n", "h", //
    "xjk", "r", "l", "d", "vby", //
    "g", "c", "m", "w", "f",
];

fn group_of(c: char) -> Option<u8> {
    GROUPS.iter().position(|g| g.contains(c)).map(|i| i as u8)
}

#[derive(Default)]
pub struct Dict {
    words: Vec<String>,
    counts: Vec<u64>,
    total: f64,
    index: HashMap<String, u32>,
    /// (key sequence, word index), sorted by sequence: exact matches and prefixes are ranges.
    seqs: Vec<(Vec<u8>, u32)>,
    bigrams: HashMap<(u32, u32), u64>,
    /// Per word: usually written in all caps (data/en-acronyms.txt).
    acronym: Vec<bool>,
}

impl Dict {
    /// `words`: lines "word\tcount". Words with letters outside a–z are skipped.
    pub fn load_words(text: &str) -> Dict {
        let mut d = Dict::default();
        for line in text.lines() {
            let Some((w, c)) = line.split_once('\t') else { continue };
            let Ok(c) = c.trim().parse::<u64>() else { continue };
            let Some(seq) = w.chars().map(group_of).collect::<Option<Vec<u8>>>() else { continue };
            if seq.is_empty() || d.index.contains_key(w) {
                continue;
            }
            let i = d.words.len() as u32;
            d.index.insert(w.to_string(), i);
            d.words.push(w.to_string());
            d.counts.push(c);
            d.total += c as f64;
            d.seqs.push((seq, i));
        }
        d.seqs.sort();
        d.acronym = vec![false; d.words.len()];
        d
    }

    /// `acronyms`: lines "word\tcount", the word in lower case. Marks them; adds those the word
    /// list lacks, with their count (at least 1).
    pub fn load_acronyms(&mut self, text: &str) {
        let mut added = false;
        for line in text.lines() {
            let Some((w, c)) = line.split_once('\t') else { continue };
            let Some(seq) = w.chars().map(group_of).collect::<Option<Vec<u8>>>() else { continue };
            if seq.is_empty() {
                continue;
            }
            let i = match self.index.get(w) {
                Some(&i) => i,
                None => {
                    let c = c.trim().parse::<u64>().unwrap_or(0).max(1);
                    let i = self.words.len() as u32;
                    self.index.insert(w.to_string(), i);
                    self.words.push(w.to_string());
                    self.counts.push(c);
                    self.total += c as f64;
                    self.seqs.push((seq, i));
                    added = true;
                    i
                }
            };
            self.acronym.resize(self.words.len(), false);
            self.acronym[i as usize] = true;
        }
        if added {
            self.seqs.sort();
        }
    }

    fn is_acronym(&self, w: u32) -> bool {
        self.acronym.get(w as usize).copied().unwrap_or(false)
    }

    /// `bigrams`: lines "a b\tcount", both words in the list.
    pub fn load_bigrams(&mut self, text: &str) {
        for line in text.lines() {
            let Some((ab, c)) = line.split_once('\t') else { continue };
            let Some((a, b)) = ab.split_once(' ') else { continue };
            let (Some(&a), Some(&b), Ok(c)) = (self.index.get(a), self.index.get(b), c.trim().parse::<u64>()) else {
                continue;
            };
            self.bigrams.insert((a, b), c);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    fn score(&self, prev: Option<u32>, w: u32) -> f64 {
        if let Some(p) = prev
            && let Some(&c) = self.bigrams.get(&(p, w))
        {
            return c as f64 / self.counts[p as usize] as f64;
        }
        // Stupid backoff to the word's own frequency.
        0.4 * self.counts[w as usize] as f64 / self.total
    }

    /// Up to `n` candidates for `seq` after `prev`: whole-word matches first, then completions.
    /// `all_caps`: everything typed was upper case, so acronyms come first within each group.
    pub fn candidates(&self, seq: &[u8], prev: &str, n: usize, all_caps: bool) -> (Vec<String>, usize) {
        if seq.is_empty() || self.is_empty() {
            return (Vec::new(), 0);
        }
        let prev = self.index.get(prev).copied();
        let lo = self.seqs.partition_point(|(s, _)| s.as_slice() < seq);
        let mut exact = Vec::new();
        let mut longer = Vec::new();
        for (s, w) in &self.seqs[lo..] {
            if !s.starts_with(seq) {
                break;
            }
            if s.len() == seq.len() { exact.push(*w) } else { longer.push(*w) }
        }
        let rank = |v: &mut Vec<u32>| {
            v.sort_by(|a, b| {
                let caps = if all_caps { self.is_acronym(*b).cmp(&self.is_acronym(*a)) } else { std::cmp::Ordering::Equal };
                caps.then(self.score(prev, *b).total_cmp(&self.score(prev, *a)))
            })
        };
        rank(&mut exact);
        rank(&mut longer);
        let n_exact = exact.len().min(n);
        let out: Vec<String> =
            exact.iter().chain(longer.iter()).take(n).map(|&w| self.words[w as usize].clone()).collect();
        (out, n_exact)
    }
}

/// What the preedit shows for `seq` when nothing matches: each key's first letter.
pub fn fallback(seq: &[u8]) -> String {
    seq.iter().map(|&k| GROUPS[k as usize].chars().next().unwrap_or('?')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes() {
        let mut d = Dict::load_words("the\t100\nput\t10\nbut\t50\nsee\t5\n");
        d.load_bigrams("to put\t40\n");
        let seq: Vec<u8> = "put".chars().map(|c| group_of(c).unwrap()).collect();
        // p and b are on different keys (ZQP, VBY), so put and but don't collide.
        assert_eq!(d.candidates(&seq, "", 4, false).0, vec!["put"]);
        let seq: Vec<u8> = "th".chars().map(|c| group_of(c).unwrap()).collect();
        assert_eq!(d.candidates(&seq, "", 4, false), (vec!["the".to_string()], 0));
    }

    #[test]
    fn all_caps_ranks_acronyms_first() {
        // a·ZQP·i: api and the commoner (made-up) aqi share keys.
        let mut d = Dict::load_words("aqi\t1000\napi\t10\n");
        d.load_acronyms("api\t10\ngpu\t5\n");
        let seq: Vec<u8> = "api".chars().map(|c| group_of(c).unwrap()).collect();
        assert_eq!(d.candidates(&seq, "", 4, false).0, vec!["aqi", "api"]);
        assert_eq!(d.candidates(&seq, "", 4, true).0, vec!["api", "aqi"]);
        // An acronym missing from the word list is added.
        let seq: Vec<u8> = "gpu".chars().map(|c| group_of(c).unwrap()).collect();
        assert_eq!(d.candidates(&seq, "", 4, true).0, vec!["gpu"]);
    }
}
