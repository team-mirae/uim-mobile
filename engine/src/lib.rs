//! uim engine (SPEC.md): mode state, layouts, composition and candidates. The Android shell turns
//! touches into (key index, direction) and applies the [`Output`]; it never decides what a key types.

pub mod ffi;
pub mod kana;
pub mod kanji;
pub mod layout;
pub mod predict;

use jis_korean::{Engine as Korean, Key as K, Mods};
use layout::{Act, KeyDef, Mode, View};
use predict::Dict;
use std::sync::Arc;

/// The bar's slots (SPEC §6): JA asks the converter for a few more, to fill them after dropping
/// duplicates and the kana as typed.
const SLOTS: usize = 4;
const CONVERSIONS: usize = 8;

/// What the shell does after applying commit and preedit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Action {
    #[default]
    None = 0,
    /// Delete one character before the cursor (Backspace with nothing composing).
    Delete = 1,
    /// Enter: the field's action or a new line.
    Enter = 2,
    Copy = 3,
    Paste = 4,
    /// Move the cursor by `arg` characters (negative = left).
    Cursor = 5,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub commit: String,
    pub preedit: String,
    pub action: Action,
    pub arg: i32,
}

pub struct Uim {
    mode: Mode,
    sym: u8,
    shift: u8,
    wide: bool,
    bottom: u8,
    last_en: Mode,
    ko: Korean,
    ko_preedit: String,
    ko_english: String,
    kana: String,
    eseq: Vec<u8>,
    /// Per typed EN predictive key: upper case (E/⇧ tapped before it, or held).
    eupper: Vec<bool>,
    held: bool,
    prev_word: String,
    dict: Dict,
    kanji: Option<Arc<kanji::Kanji>>,
    /// JA candidates for `ja_for` (kana, mode): converted once per change, not per query.
    ja_cands: Vec<String>,
    ja_for: (String, Mode),
    out: Output,
    rows: Vec<Vec<KeyDef>>,
}

impl Default for Uim {
    fn default() -> Self {
        Self::new()
    }
}

impl Uim {
    pub fn new() -> Uim {
        let mut u = Uim {
            mode: Mode::EnLower,
            sym: 0,
            shift: 0,
            wide: false,
            bottom: 0,
            last_en: Mode::EnLower,
            ko: Korean::new(),
            ko_preedit: String::new(),
            ko_english: String::new(),
            kana: String::new(),
            eseq: Vec::new(),
            eupper: Vec::new(),
            held: false,
            prev_word: String::new(),
            dict: Dict::default(),
            kanji: None,
            ja_cands: Vec::new(),
            ja_for: (String::new(), Mode::EnLower),
            out: Output::default(),
            rows: Vec::new(),
        };
        u.relayout();
        u
    }

    fn view(&self) -> View {
        View { mode: self.mode, sym: self.sym, shift: self.shift, wide: self.wide, bottom: self.bottom, held: self.held }
    }

    fn relayout(&mut self) {
        self.rows = layout::build(&self.view());
    }

    pub fn rows(&self) -> &[Vec<KeyDef>] {
        &self.rows
    }

    pub fn layout_text(&self) -> String {
        layout::serialize(&self.rows)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn output(&self) -> &Output {
        &self.out
    }

    // ---- settings and data

    pub fn set_wide(&mut self, wide: bool) {
        if wide != self.wide {
            self.wide = wide;
            self.shift = 0;
            self.relayout();
        }
    }

    pub fn set_bottom(&mut self, bottom: u8) {
        self.bottom = if bottom < layout::BOTTOM_ROWS { bottom } else { 0 };
        self.relayout();
    }

    pub fn load_words(&mut self, text: &str) {
        self.dict = Dict::load_words(text);
    }

    pub fn load_bigrams(&mut self, text: &str) {
        self.dict.load_bigrams(text);
    }

    /// After [`Self::load_words`]: the words usually written in all caps (`data/en-acronyms.txt`).
    pub fn load_acronyms(&mut self, text: &str) {
        self.dict.load_acronyms(text);
    }

    /// JA's kana-kanji converter (`data/kanji.bin`, parsed by the host once, off the UI thread).
    pub fn set_kanji(&mut self, kanji: Option<Arc<kanji::Kanji>>) {
        self.kanji = kanji;
        self.ja_for = (String::new(), Mode::EnLower);
        self.update_ja();
    }

    // ---- input

    fn begin(&mut self) {
        self.out = Output::default();
    }

    fn finish(&mut self) {
        self.update_ja();
        self.out.preedit = self.preedit();
        self.relayout();
    }

    /// A key of the current layout: row, index in row, direction (0 tap, 1 N, 2 E, 3 S, 4 W).
    pub fn press(&mut self, row: usize, col: usize, dir: usize) -> &Output {
        self.begin();
        let act = self.rows.get(row).and_then(|r| r.get(col)).map(|k| {
            let a = k.act(dir);
            if *a == Act::None && dir != layout::TAP { Act::None } else { a.clone() }
        });
        if let Some(act) = act {
            self.act(act);
        }
        self.finish();
        &self.out
    }

    /// A [`layout::HOLD`] key went down (`true`) or up. The tap, if any, comes separately.
    pub fn hold(&mut self, on: bool) -> &Output {
        self.begin();
        self.held = on;
        self.finish();
        &self.out
    }

    /// Everything typed in EN predictive was upper case (2+ letters): acronyms rank first and
    /// words show in all caps. Anything else is "not all caps" and ranks as lower case.
    fn all_caps(&self) -> bool {
        self.eupper.len() >= 2 && self.eupper.iter().all(|&u| u)
    }

    /// `word` as typed: all caps if [`Self::all_caps`], else the letters typed in upper case
    /// (E/⇧) upper-cased, position by position.
    fn cased(&self, word: &str) -> String {
        if self.all_caps() {
            return word.to_uppercase();
        }
        word.chars()
            .enumerate()
            .map(|(i, c)| if self.eupper.get(i).copied().unwrap_or(false) { c.to_uppercase().collect() } else { c.to_string() })
            .collect()
    }

    /// Dragging the caret key (JA narrow ・) by `steps` characters.
    pub fn drag(&mut self, steps: i32) -> &Output {
        self.begin();
        self.commit_composition();
        self.out.action = Action::Cursor;
        self.out.arg = steps;
        self.finish();
        &self.out
    }

    /// The candidate at `index` on the bar.
    pub fn candidate(&mut self, index: usize) -> &Output {
        self.begin();
        let cands = self.candidates();
        if let Some(c) = cands.get(index).cloned() {
            match self.mode {
                Mode::Ko => {
                    self.ko.clear();
                    self.ko_preedit.clear();
                    self.ko_english.clear();
                    self.out.commit.push_str(&c);
                }
                Mode::JaHira | Mode::JaKata => {
                    self.kana.clear();
                    self.out.commit.push_str(&c);
                }
                Mode::EnPred => {
                    self.eseq.clear();
                    self.eupper.clear();
                    self.out.commit.push_str(&c);
                    self.out.commit.push(' ');
                    self.prev_word = c.to_lowercase();
                }
                _ => {}
            }
        }
        self.finish();
        &self.out
    }

    /// Commits everything composing (focus loss, cursor moved by the user).
    pub fn flush(&mut self) -> &Output {
        self.begin();
        self.commit_composition();
        self.finish();
        &self.out
    }

    /// Up to 4 candidates for the bar.
    pub fn candidates(&self) -> Vec<String> {
        match self.mode {
            Mode::Ko if !self.ko_preedit.is_empty() && !self.ko_english.is_empty() => vec![self.ko_english.clone()],
            Mode::JaHira | Mode::JaKata if !self.kana.is_empty() => self.ja_cands.clone(),
            Mode::EnPred => self.dict.candidates(&self.eseq, &self.prev_word, 4, self.all_caps()).0.iter().map(|w| self.cased(w)).collect(),
            _ => Vec::new(),
        }
    }

    /// Recomputes the JA candidates if the kana or the script changed.
    fn update_ja(&mut self) {
        let ja = matches!(self.mode, Mode::JaHira | Mode::JaKata);
        if !ja || self.kana.is_empty() {
            self.ja_cands.clear();
            self.ja_for = (String::new(), self.mode);
            return;
        }
        if self.ja_for.0 == self.kana && self.ja_for.1 == self.mode {
            return;
        }
        self.ja_cands = self.ja_candidates();
        self.ja_for = (self.kana.clone(), self.mode);
    }

    /// The JA bar (SPEC §6): kanji conversions of the whole composition plus the other script.
    /// The kana as typed isn't listed: it's the preedit, and Space commits it. The active script
    /// weighs more: in カ, all-katakana conversions come first and hiragana takes the last slot;
    /// in ひ, katakana does.
    fn ja_candidates(&self) -> Vec<String> {
        let hira = kana::to_hiragana(&self.kana);
        let kata = kana::to_katakana(&self.kana);
        let kata_mode = self.mode == Mode::JaKata;
        let Some(kanji) = &self.kanji else {
            return if kata_mode { vec![kata, hira] } else { vec![hira, kata] };
        };
        let other = if kata_mode { hira.clone() } else { kata.clone() };
        // ー right after a kanji (こー火ー) is a lattice artefact, never a word.
        let artefact = |t: &str| {
            let cs: Vec<char> = t.chars().collect();
            cs.windows(2).any(|w| w[1] == 'ー' && ('\u{4E00}'..='\u{9FFF}').contains(&w[0]))
        };
        let mut convs: Vec<String> =
            kanji.convert(&hira, CONVERSIONS).into_iter().map(|(t, _)| t).filter(|t| !artefact(t)).collect();
        if kata_mode {
            let all_kata = |t: &String| t.chars().all(|c| ('\u{30A0}'..='\u{30FF}').contains(&c));
            convs.sort_by_key(|t| !all_kata(t)); // stable: cost order within each group
        }
        let mut out: Vec<String> = Vec::new();
        for t in convs.into_iter().chain([hira, kata]) {
            if t != self.kana && t != other && !out.contains(&t) {
                out.push(t);
            }
        }
        out.truncate(SLOTS - 1);
        if other != self.kana {
            out.push(other);
        }
        out
    }

    fn preedit(&self) -> String {
        match self.mode {
            Mode::Ko => self.ko_preedit.clone(),
            Mode::JaHira | Mode::JaKata => self.kana.clone(),
            Mode::EnPred => self.e_preedit(),
            _ => String::new(),
        }
    }

    fn e_preedit(&self) -> String {
        if self.eseq.is_empty() {
            return String::new();
        }
        let (cands, n_exact) = self.dict.candidates(&self.eseq, &self.prev_word, 4, self.all_caps());
        let p = match cands.first() {
            Some(w) if n_exact > 0 => w.clone(),
            // A completion: show as many of its letters as were typed.
            Some(w) => w.chars().take(self.eseq.len()).collect(),
            None => predict::fallback(&self.eseq),
        };
        self.cased(&p)
    }

    fn commit_composition(&mut self) {
        match self.mode {
            Mode::Ko => {
                let o = self.ko.flush();
                self.out.commit.push_str(&o.commit);
                self.ko_preedit.clear();
                self.ko_english.clear();
            }
            Mode::JaHira | Mode::JaKata => {
                let k = std::mem::take(&mut self.kana);
                self.out.commit.push_str(&k);
            }
            Mode::EnPred => {
                let p = self.e_preedit();
                if !p.is_empty() {
                    self.out.commit.push_str(&p);
                    self.prev_word = p.to_lowercase();
                }
                self.eseq.clear();
                self.eupper.clear();
            }
            _ => {}
        }
    }

    fn set_mode(&mut self, m: Mode) {
        let ja = |m: Mode| matches!(m, Mode::JaHira | Mode::JaKata);
        // ひ ↔ カ keeps the composition (SPEC §1.1).
        if !(ja(m) && ja(self.mode)) {
            self.commit_composition();
        }
        self.mode = m;
        self.sym = 0;
        self.shift = 0;
        if matches!(m, Mode::EnLower | Mode::EnUpper) {
            self.last_en = m;
        }
    }

    fn act(&mut self, act: Act) {
        match act {
            Act::None => {}
            Act::Text(s) => {
                self.commit_composition();
                self.out.commit.push_str(&s);
                if self.shift == 1 {
                    self.shift = 0;
                }
            }
            Act::Ko(keys) => {
                for (k, shift) in keys {
                    self.ko_key(k, if shift { Mods::SHIFTED } else { Mods::NONE });
                }
                if self.shift == 1 {
                    self.shift = 0;
                }
            }
            Act::Kana(s) => {
                let s = if self.mode == Mode::JaKata { kana::to_katakana(&s) } else { s };
                self.kana.push_str(&s);
                if self.shift == 1 {
                    self.shift = 0;
                }
            }
            Act::Dakuten => kana::apply_mark(&mut self.kana, false),
            Act::Handakuten => kana::apply_mark(&mut self.kana, true),
            Act::EKey(k) => {
                self.eseq.push(k);
                self.eupper.push(self.shift == 1 || self.held);
                if self.shift == 1 {
                    self.shift = 0;
                }
            }
            Act::EShift => self.shift = if self.shift == 1 { 0 } else { 1 },
            Act::Mode(m) => self.set_mode(m),
            Act::Sym => {
                self.commit_composition();
                self.sym = (self.sym + 1) % 3;
                self.shift = 0;
            }
            Act::CycKo => self.cycle(&["ko", "en", "sym"]),
            Act::CycJa => self.cycle(&["hi", "ka", "en", "sym"]),
            Act::ToggleTo(m) => {
                let other = if m == Mode::EnUpper { Mode::EnLower } else { Mode::EnUpper };
                let target = if self.sym == 0 && self.mode == m { other } else { m };
                self.set_mode(target);
            }
            Act::Caps => {
                let m = if self.mode == Mode::EnUpper { Mode::EnLower } else { Mode::EnUpper };
                self.set_mode(m);
            }
            Act::Shift => self.shift = (self.shift + 1) % 3,
            Act::Copy => self.out.action = Action::Copy,
            Act::Paste => {
                self.commit_composition();
                self.out.action = Action::Paste;
            }
            Act::Backspace => self.backspace(),
            Act::Space => {
                self.commit_composition();
                self.out.commit.push(' ');
            }
            Act::Enter => {
                self.commit_composition();
                self.out.action = Action::Enter;
            }
        }
    }

    /// 한A1 / あA1 (SPEC §1.2): step from the current state if it's in `list`, else go to its
    /// first state. "en" is English in the last used case.
    fn cycle(&mut self, list: &[&str]) {
        let en = matches!(self.mode, Mode::EnLower | Mode::EnUpper);
        let cur = if self.sym > 0 {
            "sym"
        } else if en {
            "en"
        } else {
            match self.mode {
                Mode::Ko => "ko",
                Mode::JaHira => "hi",
                Mode::JaKata => "ka",
                _ => "other",
            }
        };
        let next = match list.iter().position(|s| *s == cur) {
            Some(i) => list[(i + 1) % list.len()],
            None => list[0],
        };
        match next {
            "ko" => self.set_mode(Mode::Ko),
            "hi" => self.set_mode(Mode::JaHira),
            "ka" => self.set_mode(Mode::JaKata),
            "en" => self.set_mode(self.last_en),
            _ => {
                self.commit_composition();
                self.sym = 1;
            }
        }
    }

    fn ko_key(&mut self, k: K, mods: Mods) {
        let o = self.ko.key(k, mods);
        self.out.commit.push_str(&o.commit);
        self.ko_preedit = o.preedit;
        self.ko_english = o.english;
        if !o.consumed {
            // A key Mode A doesn't map: type what its legend shows.
            if let Some(c) = jis_korean::output(jis_korean::Layout::JisKorean, k, mods.shift) {
                self.out.commit.push(c.char());
            }
        }
    }

    fn backspace(&mut self) {
        match self.mode {
            Mode::Ko if !self.ko_preedit.is_empty() => {
                let o = self.ko.key(K::Backspace, Mods::NONE);
                self.out.commit.push_str(&o.commit);
                self.ko_preedit = o.preedit;
                self.ko_english = o.english;
                if !o.consumed {
                    self.out.action = Action::Delete;
                }
            }
            Mode::JaHira | Mode::JaKata if !self.kana.is_empty() => {
                self.kana.pop();
            }
            Mode::EnPred if !self.eseq.is_empty() => {
                self.eseq.pop();
                self.eupper.pop();
            }
            _ => self.out.action = Action::Delete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout::{E, N, S, TAP, W};
    #[allow(unused_imports)]
    use layout::Kind;

    fn find(u: &Uim, label: &str) -> (usize, usize) {
        for (r, row) in u.rows().iter().enumerate() {
            for (c, k) in row.iter().enumerate() {
                if k.label == label {
                    return (r, c);
                }
            }
        }
        panic!("no key {label}");
    }

    /// The bottom row's key with this label.
    fn mode_tap(u: &mut Uim, label: &str) -> Output {
        let r = u.rows().len() - 1;
        let c = u.rows()[r].iter().position(|k| k.label == label).expect(label);
        u.press(r, c, TAP).clone()
    }

    fn tap(u: &mut Uim, label: &str) -> Output {
        let (r, c) = find(u, label);
        u.press(r, c, TAP).clone()
    }

    fn flick(u: &mut Uim, label: &str, dir: usize) -> Output {
        let (r, c) = find(u, label);
        u.press(r, c, dir).clone()
    }

    #[test]
    fn english_narrow() {
        let mut u = Uim::new();
        assert_eq!(tap(&mut u, "h").commit, "h");
        assert_eq!(tap(&mut u, "T").commit, "T"); // other-case key, case unchanged
        assert_eq!(u.mode(), Mode::EnLower);
        mode_tap(&mut u, "A");
        assert_eq!(u.mode(), Mode::EnUpper);
        assert_eq!(tap(&mut u, "Q").commit, "Q");
    }

    #[test]
    fn korean_narrow_flicks() {
        let mut u = Uim::new();
        mode_tap(&mut u, "한");
        let o = flick(&mut u, "ㄱ", E);
        assert_eq!(o.preedit, "가");
        let o = tap(&mut u, "ㄴ");
        assert_eq!(o.preedit, "간");
        let o = tap(&mut u, "space");
        assert_eq!(o.commit, "간 ");
        // ㅙ is two keys inside one gesture.
        tap(&mut u, "ㄷ");
        let o = tap(&mut u, "ㅙ");
        assert_eq!(o.preedit, "돼");
        // Backspace: the unfinished syllable loses its last jamo keystroke.
        let o = tap(&mut u, "⌫");
        assert_eq!(o.preedit, "도");
        // A consonant flicked S is the consonant + ㅜ.
        let o = flick(&mut u, "ㄹ", S);
        assert!(o.preedit.ends_with('루'), "{o:?}");
        let o = flick(&mut u, "ㅁ", W);
        assert!(o.preedit.ends_with('머'), "{o:?}");
        let _ = N;
    }

    #[test]
    fn korean_wide_shift_z() {
        let mut u = Uim::new();
        u.set_wide(true);
        mode_tap(&mut u, "한");
        let (r, c) = find(&u, "Shift");
        u.press(r, c, TAP);
        let o = tap(&mut u, "ㅋ");
        assert_eq!(o.preedit, "ㅋ");
    }

    #[test]
    fn japanese_narrow() {
        let mut u = Uim::new();
        mode_tap(&mut u, "ひ");
        let o = flick(&mut u, "か", N);
        assert_eq!(o.preedit, "く");
        let o = tap(&mut u, "゛");
        assert_eq!(o.preedit, "ぐ");
        // カ keeps the composition and types katakana from here.
        mode_tap(&mut u, "カ");
        let o = tap(&mut u, "ア");
        assert_eq!(o.preedit, "ぐア");
        assert_eq!(u.candidates()[0], "グア");
        let o = u.candidate(1).clone();
        assert_eq!(o.commit, "ぐあ");
    }

    #[test]
    fn predictive() {
        let mut u = Uim::new();
        u.load_words("the\t100\nput\t10\nbut\t50\n");
        mode_tap(&mut u, "E");
        assert_eq!(u.mode(), Mode::EnPred);
        tap(&mut u, "T");
        tap(&mut u, "H");
        tap(&mut u, "E");
        assert_eq!(u.candidates().first().map(String::as_str), Some("the"));
        let o = tap(&mut u, "space");
        assert_eq!(o.commit, "the ");
    }

    #[test]
    fn e_shift_rows() {
        for (bottom, order) in [(5, ["한", "ひ", "カ"]), (6, ["ひ", "カ", "한"])] {
            let mut u = Uim::new();
            u.load_words("the\t100\nput\t10\nbut\t50\n");
            u.set_bottom(bottom);
            let last = u.rows().len() - 1;
            let labels: Vec<&str> = u.rows()[last].iter().map(|k| k.label.as_str()).collect();
            assert_eq!(labels, ["S", "E", ",", "space", order[0], order[1], order[2], "⏎"]);
            let widths: f32 = u.rows()[last].iter().map(|k| k.w).sum();
            assert_eq!(widths, 13.0);
            // , flicks: E . / W @ / N -.
            assert_eq!(flick(&mut u, ",", E).commit, ".");
            assert_eq!(flick(&mut u, ",", W).commit, "@");
            assert_eq!(flick(&mut u, ",", N).commit, "-");
            assert_eq!(mode_tap(&mut u, ",").commit, ",");
            // E enters EN predictive, where the key becomes ⇧.
            mode_tap(&mut u, "E");
            assert_eq!(u.mode(), Mode::EnPred);
            // Tap ⇧: the next letter only is upper case.
            mode_tap(&mut u, "⇧");
            tap(&mut u, "T");
            tap(&mut u, "H");
            tap(&mut u, "E");
            assert_eq!(u.output().preedit, "The");
            assert_eq!(u.candidates().first().map(String::as_str), Some("The"));
            assert_eq!(mode_tap(&mut u, "space").commit, "The ");
            // ⇧ twice cancels: no caps lock.
            mode_tap(&mut u, "⇧");
            mode_tap(&mut u, "⇧");
            tap(&mut u, "T");
            assert_eq!(u.output().preedit, "t");
            u.flush();
            // Held: every letter typed meanwhile is upper case.
            u.hold(true);
            tap(&mut u, "T");
            tap(&mut u, "H");
            tap(&mut u, "E");
            u.hold(false);
            assert_eq!(u.output().preedit, "THE");
            let o = u.candidate(0).clone();
            assert_eq!(o.commit, "THE ");
            // Leaving EN predictive: the key is E again.
            mode_tap(&mut u, order[0]);
            assert!(u.rows()[last].iter().any(|k| k.label == "E"));
        }
    }

    #[test]
    fn cycle_rows() {
        let mut u = Uim::new();
        u.set_bottom(2);
        // From EN, あA1 steps on to symbols; from symbols, to its first state.
        mode_tap(&mut u, "あA1");
        assert_eq!(u.sym, 1);
        mode_tap(&mut u, "あA1");
        assert_eq!((u.sym, u.mode()), (0, Mode::JaHira));
        mode_tap(&mut u, "あA1");
        assert_eq!(u.mode(), Mode::JaKata);
        mode_tap(&mut u, "あA1");
        assert_eq!(u.mode(), Mode::EnLower);
        // From JA, 한A1 goes to its first state, KO.
        mode_tap(&mut u, "あA1");
        assert_eq!(u.sym, 1);
        mode_tap(&mut u, "한A1");
        assert_eq!(u.mode(), Mode::Ko);
        mode_tap(&mut u, "A");
        assert_eq!(u.mode(), Mode::EnUpper);
        mode_tap(&mut u, "A");
        assert_eq!(u.mode(), Mode::EnLower);
    }
}
