//! Layout data (SPEC §1–§5): every key's slot, legends and what each gesture does. The shell
//! draws these and sends back (key index, direction); it never decides what a key types.

use jis_korean::{Key as K, Layout as JkLayout, output};

use crate::kana::to_katakana;
use crate::predict::GROUPS;

/// Flick directions. 0 is a tap.
pub const TAP: usize = 0;
pub const N: usize = 1;
pub const E: usize = 2;
pub const S: usize = 3;
pub const W: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    EnLower,
    EnUpper,
    EnPred,
    Ko,
    JaHira,
    JaKata,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    None,
    /// Commit as is, after committing the composition.
    Text(String),
    /// jis-korean keys in order, each with Shift or not.
    Ko(Vec<(K, bool)>),
    /// Kana into the JA composition (given in hiragana; typed in the active script).
    Kana(String),
    Dakuten,
    Handakuten,
    /// An EN predictive key (index into GROUPS).
    EKey(u8),
    Mode(Mode),
    Sym,
    CycKo,
    CycJa,
    /// Cycle rows' A / a: English in that case, or the other case if already there (SPEC §1.2).
    ToggleTo(Mode),
    Caps,
    Shift,
    /// The E/⇧ key in EN predictive (SPEC §1.2, E-shift rows): the next letter upper case, once.
    /// Holding it types upper case while held (the shell reports the hold, [`HOLD`]).
    EShift,
    Copy,
    Paste,
    Backspace,
    Space,
    Enter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Char,
    Func,
    Mode,
    Cand,
    Reserved,
}

pub const ON: u32 = 1;
pub const LOCK: u32 = 2;
pub const SMALL: u32 = 4;
pub const CARET: u32 = 8;
pub const OTHER: u32 = 16;
pub const ICON_COPY: u32 = 32;
pub const ICON_PASTE: u32 = 64;
pub const ICON_BS: u32 = 128;
pub const ICON_ENTER: u32 = 256;
pub const ICON_CAPS: u32 = 512;
pub const ICON_SHIFT: u32 = 1024;
pub const REPEAT: u32 = 2048;
/// The shell reports touch-down and lift ([`crate::Uim::hold`]), and sends the tap only if no
/// other key was pressed meanwhile.
pub const HOLD: u32 = 4096;

#[derive(Clone, Debug)]
pub struct KeyDef {
    pub kind: Kind,
    pub w: f32,
    pub label: String,
    pub corner: String,
    /// Small legends N, E, S, W.
    pub legends: [String; 4],
    /// Tap, N, E, S, W.
    pub acts: [Act; 5],
    pub flags: u32,
}

impl KeyDef {
    fn new(kind: Kind, label: &str, tap: Act) -> KeyDef {
        KeyDef {
            kind,
            w: 1.0,
            label: label.to_string(),
            corner: String::new(),
            legends: Default::default(),
            acts: [tap, Act::None, Act::None, Act::None, Act::None],
            flags: 0,
        }
    }
    fn w(mut self, w: f32) -> Self {
        self.w = w;
        self
    }
    fn flag(mut self, f: u32) -> Self {
        self.flags |= f;
        self
    }
    fn on(self, on: bool) -> Self {
        if on { self.flag(ON) } else { self }
    }
    pub fn act(&self, dir: usize) -> &Act {
        &self.acts[dir.min(4)]
    }
}

fn text(s: &str) -> KeyDef {
    KeyDef::new(Kind::Char, s, Act::Text(s.to_string()))
}
fn texts(s: &str) -> Vec<KeyDef> {
    s.split(' ').map(text).collect()
}
fn rsv() -> KeyDef {
    KeyDef::new(Kind::Reserved, "", Act::None)
}
fn func(label: &str, act: Act, flags: u32) -> KeyDef {
    KeyDef::new(Kind::Func, label, act).flag(flags)
}
fn mode_key(label: &str, act: Act) -> KeyDef {
    KeyDef::new(Kind::Mode, label, act)
}

/// What the layout depends on.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub mode: Mode,
    pub sym: u8,
    pub shift: u8,
    pub wide: bool,
    pub bottom: u8,
    /// The E/⇧ key is held (E-shift rows).
    pub held: bool,
}

/// The number of bottom rows (SPEC §1.2): 0–4, then the two E-shift rows.
pub const BOTTOM_ROWS: u8 = 7;

pub fn build(v: &View) -> Vec<Vec<KeyDef>> {
    let mut rows = vec![top()];
    let keys = if v.sym > 0 {
        symbols(v.sym)
    } else {
        match v.mode {
            Mode::EnLower | Mode::EnUpper if v.wide => en_wide(v.mode == Mode::EnUpper),
            Mode::EnLower | Mode::EnUpper => en_narrow(v.mode == Mode::EnUpper),
            Mode::EnPred => en_pred(),
            Mode::Ko if v.wide => ko_wide(v.shift),
            Mode::Ko => ko_narrow(),
            Mode::JaHira | Mode::JaKata if v.wide => ja_wide(v.mode == Mode::JaKata, v.shift),
            Mode::JaHira | Mode::JaKata => ja_narrow(v.mode == Mode::JaKata),
        }
    };
    rows.extend(keys);
    rows.push(bottom(v));
    rows
}

fn top() -> Vec<KeyDef> {
    vec![
        func("Copy", Act::Copy, ICON_COPY),
        func("Paste", Act::Paste, ICON_PASTE),
        KeyDef::new(Kind::Cand, "", Act::None).w(8.0),
        func("⌫", Act::Backspace, ICON_BS | REPEAT).w(3.0),
    ]
}

fn bottom(v: &View) -> Vec<KeyDef> {
    let m = v.mode;
    let sym = v.sym > 0;
    let lit = |want: Mode| !sym && m == want;
    let s = mode_key(if sym { if v.sym == 1 { "S1" } else { "S2" } } else { "S" }, Act::Sym).on(sym);
    let e = mode_key("E", Act::Mode(Mode::EnPred)).on(lit(Mode::EnPred));
    let a_up = mode_key("A", Act::Mode(Mode::EnUpper)).on(lit(Mode::EnUpper));
    let a_lo = mode_key("a", Act::Mode(Mode::EnLower)).on(lit(Mode::EnLower));
    let ko = mode_key("한", Act::Mode(Mode::Ko)).on(lit(Mode::Ko));
    let hi = mode_key("ひ", Act::Mode(Mode::JaHira)).on(lit(Mode::JaHira));
    let ka = mode_key("カ", Act::Mode(Mode::JaKata)).on(lit(Mode::JaKata));
    let space = func("space", Act::Space, 0).w(3.0);
    let enter = func("⏎", Act::Enter, ICON_ENTER).w(3.0);
    let cyc_ko = mode_key("한A1", Act::CycKo).w(1.5).on(lit(Mode::Ko));
    let cyc_ja = mode_key("あA1", Act::CycJa).w(1.5).on(lit(Mode::JaHira) || lit(Mode::JaKata));
    // E-shift rows: E enters EN predictive; there, it's ⇧ (tap: next letter upper case; hold:
    // upper case while held). No A, a or caps lock.
    let e_shift = || {
        if m == Mode::EnPred && !sym {
            func("⇧", Act::EShift, ICON_SHIFT | HOLD).w(1.5).on(v.shift == 1 || v.held)
        } else {
            mode_key("E", Act::Mode(Mode::EnPred)).w(1.5)
        }
    };
    let comma = || {
        let mut k = text(",").w(1.5);
        k.acts[N] = Act::Text("-".into());
        k.acts[E] = Act::Text(".".into());
        k.acts[W] = Act::Text("@".into());
        k.legends = ["-".into(), ".".into(), String::new(), "@".into()];
        k
    };
    match v.bottom {
        5 => vec![s, e_shift(), comma(), space, ko, hi, ka, enter],
        6 => vec![s, e_shift(), comma(), space, hi, ka, ko, enter],
        1 => vec![s, e, a_up, a_lo, space, hi, ka, ko, enter],
        2 => vec![cyc_ko, cyc_ja, mode_key("A", Act::ToggleTo(Mode::EnUpper)).on(lit(Mode::EnUpper)), space.w(6.0), enter],
        3 => vec![cyc_ko, cyc_ja, mode_key("a", Act::ToggleTo(Mode::EnLower)).on(lit(Mode::EnLower)), space.w(6.0), enter],
        4 => vec![cyc_ko, cyc_ja, e, space.w(6.0), enter],
        _ => vec![s, e, a_up, a_lo, space, ko, hi, ka, enter],
    }
}

// ---- EN (§3.1, §3.2)

fn en_wide(upper: bool) -> Vec<Vec<KeyDef>> {
    let letters = |s: &str| if upper { s.to_uppercase() } else { s.to_string() };
    let caps = func("Caps", Act::Caps, ICON_CAPS).on(upper);
    let mut r4 = vec![caps];
    r4.extend(texts(&letters("z x c v b n m")));
    r4.extend(texts(", . / \\"));
    vec![
        texts("1 2 3 4 5 6 7 8 9 0 - ^"),
        [texts(&letters("q w e r t y u i o p")), texts("@ [")].concat(),
        [texts(&letters("a s d f g h j k l")), texts("; : ]")].concat(),
        r4,
    ]
}

fn en_narrow(upper: bool) -> Vec<Vec<KeyDef>> {
    let other = |s: &str| text(s).flag(OTHER);
    if upper {
        vec![
            [vec![other("e"), other("t")], texts("K L I O P")].concat(),
            texts("Q W E R T Y U"),
            texts("A S D F G H J"),
            texts("Z X C V B N M"),
        ]
    } else {
        vec![
            [vec![other("T"), other("A")], texts("k l i o p")].concat(),
            texts("q w e r t y u"),
            texts("a s d f g h j"),
            texts("z x c v b n m"),
        ]
    }
}

// ---- EN predictive (§3.3)

fn en_pred() -> Vec<Vec<KeyDef>> {
    GROUPS
        .chunks(5)
        .enumerate()
        .map(|(r, row)| {
            row.iter()
                .enumerate()
                .map(|(c, g)| KeyDef::new(Kind::Char, &g.to_uppercase(), Act::EKey((r * 5 + c) as u8)))
                .collect()
        })
        .collect()
}

// ---- KO wide (§4.1): Mode A on the 12 × 4 grid

const KO_WIDE: [[K; 12]; 4] = [
    [K::Digit1, K::Digit2, K::Digit3, K::Digit4, K::Digit5, K::Digit6, K::Digit7, K::Digit8, K::Digit9, K::Digit0, K::Minus, K::Equal],
    [K::KeyQ, K::KeyW, K::KeyE, K::KeyR, K::KeyT, K::KeyY, K::KeyU, K::KeyI, K::KeyO, K::KeyP, K::BracketLeft, K::BracketRight],
    [K::KeyA, K::KeyS, K::KeyD, K::KeyF, K::KeyG, K::KeyH, K::KeyJ, K::KeyK, K::KeyL, K::Semicolon, K::Quote, K::Backslash],
    // Row 4 starts with ⇧; 11 keys follow (the 12th slot is unused).
    [K::KeyZ, K::KeyX, K::KeyC, K::KeyV, K::KeyB, K::KeyN, K::KeyM, K::Comma, K::Period, K::Slash, K::IntlRo, K::Other],
];

fn legend(key: K, shift: bool) -> String {
    output(JkLayout::JisKorean, key, shift).map(|o| o.char().to_string()).unwrap_or_default()
}

fn ko_wide(shift: u8) -> Vec<Vec<KeyDef>> {
    let mut rows = Vec::new();
    for (r, keys) in KO_WIDE.iter().enumerate() {
        let mut row = Vec::new();
        if r == 3 {
            row.push(func("Shift", Act::Shift, ICON_SHIFT).flag(match shift {
                1 => ON,
                2 => ON | LOCK,
                _ => 0,
            }));
        }
        for &k in keys.iter().take(if r == 3 { 11 } else { 12 }) {
            // Shift+Z is not F10 (§4.1): ⇧ then ㅋ is plain ㅋ.
            let shiftable = k != K::KeyZ;
            let plain = legend(k, false);
            let shifted = if shiftable { legend(k, true) } else { String::new() };
            let shifted = if shifted == plain { String::new() } else { shifted };
            let mut d = KeyDef::new(Kind::Char, &plain, Act::Ko(vec![(k, shift > 0 && shiftable)]));
            if shift > 0 && !shifted.is_empty() {
                d.label = shifted;
                d.corner = plain;
            } else {
                d.corner = shifted;
            }
            row.push(d);
        }
        rows.push(row);
    }
    rows
}

// ---- KO narrow (§4.2): 8 × 4, flicks

fn vowel_keys(v: char) -> Vec<(K, bool)> {
    let k = |k: K| vec![(k, false)];
    match v {
        'ㅛ' => k(K::KeyY),
        'ㅕ' => k(K::KeyU),
        'ㅑ' => k(K::KeyI),
        'ㅐ' => k(K::KeyO),
        'ㅔ' => k(K::KeyP),
        'ㅒ' => vec![(K::KeyO, true)],
        'ㅖ' => vec![(K::KeyP, true)],
        'ㅗ' => k(K::KeyH),
        'ㅓ' => k(K::KeyJ),
        'ㅏ' => k(K::KeyK),
        'ㅣ' => k(K::KeyL),
        'ㅠ' => k(K::KeyB),
        'ㅜ' => k(K::KeyN),
        'ㅡ' => k(K::KeyM),
        'ㅟ' => k(K::Digit6),
        'ㅝ' => k(K::Digit7),
        'ㅚ' => k(K::Digit8),
        'ㅢ' => k(K::Digit9),
        'ㅘ' => k(K::Digit0),
        'ㅙ' => vec![(K::KeyH, false), (K::KeyO, false)],
        'ㅞ' => vec![(K::KeyN, false), (K::KeyP, false)],
        _ => Vec::new(),
    }
}

fn consonant(label: &str, key: K) -> KeyDef {
    let with = |v: char| [vec![(key, false)], vowel_keys(v)].concat();
    let mut d = KeyDef::new(Kind::Char, label, Act::Ko(vec![(key, false)]));
    d.acts[N] = Act::Ko(with('ㅗ'));
    d.acts[E] = Act::Ko(with('ㅏ'));
    d.acts[S] = Act::Ko(with('ㅜ'));
    d.acts[W] = Act::Ko(with('ㅓ'));
    d.legends = ["ㅗ".into(), "ㅏ".into(), "ㅜ".into(), "ㅓ".into()];
    d
}

/// A vowel key: "C N E S W", "-" for nothing.
fn vowel(spec: &str) -> KeyDef {
    let parts: Vec<&str> = spec.split(' ').collect();
    let act = |p: &str| match p.chars().next() {
        Some('-') | None => Act::None,
        Some('#') => Act::Text("#".into()),
        Some(c) => Act::Ko(vowel_keys(c)),
    };
    let mut d = KeyDef::new(Kind::Char, parts[0], act(parts[0]));
    for (i, dir) in [N, E, S, W].into_iter().enumerate() {
        let p = parts.get(i + 1).copied().unwrap_or("-");
        d.acts[dir] = act(p);
        d.legends[dir - 1] = if p == "-" { String::new() } else { p.to_string() };
    }
    d
}

fn ko_narrow() -> Vec<Vec<KeyDef>> {
    let c = |row: &[(&str, K)]| row.iter().map(|&(l, k)| consonant(l, k)).collect::<Vec<_>>();
    vec![
        [c(&[("ㅃ", K::Digit1), ("ㅉ", K::Digit2), ("ㄸ", K::Digit3), ("ㄲ", K::Digit4), ("ㅆ", K::Digit5)]),
            vec![vowel("ㅢ ㅡ ㅣ ㅡ ㅣ"), vowel("ㅙ ㅗ ㅐ ㅜ -"), vowel("ㅞ ㅗ ㅔ ㅜ -")]].concat(),
        [c(&[("ㅂ", K::KeyQ), ("ㅈ", K::KeyW), ("ㄷ", K::KeyE), ("ㄱ", K::KeyR), ("ㅅ", K::KeyT)]),
            vec![vowel("ㅚ ㅗ ㅣ ㅟ -"), vowel("ㅟ ㅚ ㅣ ㅜ -"), vowel("ㅣ ㅡ ㅏ ㅡ ㅓ")]].concat(),
        [c(&[("ㅁ", K::KeyA), ("ㄴ", K::KeyS), ("ㅇ", K::KeyD), ("ㄹ", K::KeyF), ("ㅎ", K::KeyG)]),
            vec![vowel("ㅡ ㅗ ㅣ ㅜ ㅣ"), vowel("ㅏ ㅘ ㅐ - ㅐ"), vowel("ㅓ - ㅔ ㅝ ㅔ")]].concat(),
        [c(&[("ㅋ", K::KeyZ), ("ㅌ", K::KeyX), ("ㅊ", K::KeyC), ("ㅍ", K::KeyV)]),
            vec![vowel("ㅘ ㅗ ㅏ ㅜ ㅓ"), vowel("# ㅛ ㅑ ㅠ ㅕ"), vowel("ㅒ - - - -"), vowel("ㅖ - - - -")]].concat(),
    ]
}

// ---- JA wide (§4.3): JIS kana, 13 × 4

fn kana_key(spec: &str, kata: bool, shift: u8) -> KeyDef {
    // "plain|shifted"
    let (plain, shifted) = spec.split_once('|').unwrap_or((spec, ""));
    let script = |s: &str| if kata { to_katakana(s) } else { s.to_string() };
    let act = |s: &str| match s {
        "゛" => Act::Dakuten,
        "゜" => Act::Handakuten,
        _ => Act::Kana(s.to_string()),
    };
    let use_shift = shift > 0 && !shifted.is_empty();
    let (main, corner) = if use_shift { (shifted, plain) } else { (plain, shifted) };
    let mut d = KeyDef::new(Kind::Char, &script(main), act(main));
    d.corner = script(corner);
    d
}

fn ja_wide(kata: bool, shift: u8) -> Vec<Vec<KeyDef>> {
    let row = |s: &str| s.split(' ').map(|k| kana_key(k, kata, shift)).collect::<Vec<_>>();
    let shift_key = func("Shift", Act::Shift, ICON_SHIFT).flag(match shift {
        1 => ON,
        2 => ON | LOCK,
        _ => 0,
    });
    vec![
        row("ぬ ふ あ|ぁ う|ぅ え|ぇ お|ぉ や|ゃ ゆ|ゅ よ|ょ わ|を ほ へ ー"),
        [row("た て い|ぃ す か ん な に ら せ ゛ ゜|「"), vec![func("Hi", Act::Mode(Mode::JaHira), 0).on(!kata)]].concat(),
        [row("ち と し は き く ま の り れ け む|」"), vec![func("Ka", Act::Mode(Mode::JaKata), 0).on(kata)]].concat(),
        [vec![shift_key], row("っ つ|っ さ そ ひ こ み も ね|、 る|。 め|・ ろ")].concat(),
    ]
}

// ---- JA narrow (§4.4): flick kana, 5 × 4. Gboard order: C a, W i, N u, E e, S o.

/// "C W N E S", "-" for nothing.
fn flick_kana(spec: &str, kata: bool) -> KeyDef {
    let parts: Vec<&str> = spec.split(' ').collect();
    let script = |s: &str| if kata { to_katakana(s) } else { s.to_string() };
    let mut d = KeyDef::new(Kind::Char, &script(parts[0]), Act::Kana(parts[0].to_string()));
    for (i, dir) in [W, N, E, S].into_iter().enumerate() {
        let p = parts.get(i + 1).copied().unwrap_or("-");
        if p != "-" {
            d.acts[dir] = Act::Kana(p.to_string());
            d.legends[dir - 1] = script(p);
        }
    }
    d
}

fn ja_narrow(kata: bool) -> Vec<Vec<KeyDef>> {
    let f = |s: &str| flick_kana(s, kata);
    let small = |s: &str| flick_kana(s, kata).flag(SMALL);
    let mut quote = flick_kana("\" 「 『 」 』", kata);
    quote.label = "「」".into();
    quote.acts[TAP] = Act::Text("\"".into());
    let mut dot = KeyDef::new(Kind::Char, "・", Act::Kana("・".into())).flag(CARET);
    dot.legends = [String::new(), "▸".into(), String::new(), "◂".into()];
    let mut daku = KeyDef::new(Kind::Char, "゛", Act::Dakuten);
    daku.label = "゛".into();
    let handa = KeyDef::new(Kind::Char, "゜", Act::Handakuten);
    vec![
        vec![f("あ い う え お"), f("か き く け こ"), f("さ し す せ そ"), small("ぁ ぃ ぅ ぇ ぉ"), f("、 ！ - ？ -")],
        vec![f("た ち つ て と"), f("な に ぬ ね の"), f("は ひ ふ へ ほ"), small("っ"), f("。 ！ - ？ -")],
        vec![f("ま み む め も"), f("や （ ゆ ） よ"), f("ら り る れ ろ"), small("ゃ - ゅ - ょ"), quote],
        vec![daku, f("わ を ん ー -"), handa, small("ゎ"), dot],
    ]
}

// ---- Symbols (§5): 10 × 4, two pages

fn symbols(page: u8) -> Vec<Vec<KeyDef>> {
    if page == 1 {
        vec![
            texts("1 2 3 4 5 6 7 8 9 0"),
            texts("! \" # $ % & ' ( ) ="),
            texts("- ^ ~ | @ ` [ { ] }"),
            texts("; + : * , . / ? \\ _"),
        ]
    } else {
        vec![
            texts("1 2 3 4 5 6 7 8 9 0"),
            texts("< > — … 、 。 ・ ※ 〜 ￥"),
            [texts("「 」 『 』 【 】"), (0..4).map(|_| rsv()).collect()].concat(),
            (0..10).map(|_| rsv()).collect(),
        ]
    }
}

/// The layout as text for the shell: rows by '\n', keys by 0x1E, fields by 0x1F:
/// kind, width, label, corner, legend N, E, S, W, flags.
pub fn serialize(rows: &[Vec<KeyDef>]) -> String {
    let mut out = String::new();
    for (r, row) in rows.iter().enumerate() {
        if r > 0 {
            out.push('\n');
        }
        for (i, k) in row.iter().enumerate() {
            if i > 0 {
                out.push('\u{1E}');
            }
            let kind = match k.kind {
                Kind::Char => "c",
                Kind::Func => "f",
                Kind::Mode => "m",
                Kind::Cand => "cand",
                Kind::Reserved => "rsv",
            };
            let fields = [
                kind.to_string(),
                k.w.to_string(),
                k.label.clone(),
                k.corner.clone(),
                k.legends[0].clone(),
                k.legends[1].clone(),
                k.legends[2].clone(),
                k.legends[3].clone(),
                k.flags.to_string(),
            ];
            out.push_str(&fields.join("\u{1F}"));
        }
    }
    out
}
