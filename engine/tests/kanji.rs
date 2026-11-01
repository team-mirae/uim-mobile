//! JA kana-kanji conversion (SPEC §6, §8 M6) on the shipped data (data/kanji.bin): accuracy on
//! Mozc's evaluation set, malformed data never panicking, and the candidate bar.

use std::sync::{Arc, OnceLock};
use std::time::Instant;

use uim::kanji::{Kanji, MAX_INPUT};
use uim::layout::{Act, Mode, TAP};
use uim::Uim;

fn data() -> &'static [u8] {
    static DATA: OnceLock<Vec<u8>> = OnceLock::new();
    DATA.get_or_init(|| std::fs::read("data/kanji.bin").expect("data/kanji.bin (python3 -I tools/kanji/build_kanji.py)"))
}

fn kanji() -> Arc<Kanji> {
    static KANJI: OnceLock<Arc<Kanji>> = OnceLock::new();
    KANJI
        .get_or_init(|| {
            let t = Instant::now();
            let k = Kanji::parse(data()).unwrap();
            eprintln!("parse: {:?}", t.elapsed());
            Arc::new(k)
        })
        .clone()
}

/// Where `c` is on the current layout: (row, col, dir).
fn key_for(u: &Uim, c: char) -> Option<(usize, usize, usize)> {
    let want = Act::Kana(c.to_string());
    for (r, row) in u.rows().iter().enumerate() {
        for (col, k) in row.iter().enumerate() {
            if let Some(dir) = k.acts.iter().position(|a| *a == want) {
                return Some((r, col, dir));
            }
        }
    }
    None
}

fn press_label(u: &mut Uim, label: &str) {
    for (r, row) in u.rows().iter().enumerate() {
        if let Some(col) = row.iter().position(|k| k.label == label) {
            u.press(r, col, TAP);
            return;
        }
    }
    panic!("no key {label}");
}

/// Types hiragana on JA narrow, voiced kana as the base kana then ゛ or ゜.
fn type_kana(u: &mut Uim, text: &str) {
    for c in text.chars() {
        if let Some((r, col, dir)) = key_for(u, c) {
            u.press(r, col, dir);
            continue;
        }
        let base = |d: u32| char::from_u32(c as u32 - d).and_then(|b| key_for(u, b));
        if let Some((r, col, dir)) = base(1) {
            u.press(r, col, dir);
            press_label(u, "゛");
        } else if let Some((r, col, dir)) = base(2) {
            u.press(r, col, dir);
            press_label(u, "゜");
        } else {
            panic!("can't type {c}");
        }
    }
}

fn ja_engine(mode_label: &str) -> Uim {
    let mut u = Uim::new();
    u.set_kanji(Some(kanji()));
    press_label(&mut u, mode_label);
    u
}

#[test]
fn the_converter_meets_its_accuracy_floor_on_mozcs_evaluation_set() {
    // Exact whole-sentence matches on Mozc's own evaluation sentences (BSD-3, tests/data). In
    // jis-moonshot, the same data scored about 70% exact.
    let text = std::fs::read_to_string("tests/data/kanji-evaluation.tsv").unwrap();
    let k = kanji();
    let (mut total, mut exact, mut listed) = (0, 0, 0);
    let t = Instant::now();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [_, input, expected, command, ..] = fields[..] else { continue };
        if !command.starts_with("Conversion") {
            continue;
        }
        total += 1;
        let got = k.convert(input, 3);
        exact += (got.first().map(|g| g.0.as_str()) == Some(expected)) as usize;
        listed += got.iter().any(|g| g.0 == expected) as usize;
    }
    eprintln!("kanji: {exact}/{total} exact, {listed}/{total} in the top 3, {:?}", t.elapsed());
    assert!(total > 600, "{total}");
    assert!(exact * 100 >= total * 66, "{exact}/{total} exact");
    assert!(listed * 100 >= total * 75, "{listed}/{total} in the top 3");
}

#[test]
fn everyday_phrases_convert() {
    let k = kanji();
    for (kana, want) in [
        ("きょうはいいてんきですね", "今日はいい天気ですね"),
        ("わたしはがくせいです", "私は学生です"),
        ("にほんご", "日本語"),
        ("ありがとうございます", "ありがとうございます"),
    ] {
        let got = k.convert(kana, 3);
        assert!(got.iter().any(|g| g.0 == want), "{kana}: {got:?}");
    }
}

#[test]
fn malformed_data_is_an_error_never_a_panic() {
    let data = data();
    assert!(Kanji::parse(&[]).is_err());
    assert!(Kanji::parse(b"JKKJ").is_err());
    for len in (0..4096).chain((4096..data.len()).step_by(data.len() / 64 + 1)) {
        assert!(Kanji::parse(&data[..len]).is_err(), "{len}");
    }
    let mut seed = 0x2545_f491_u64;
    for _ in 0..24 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut copy = data.to_vec();
        let at = if seed % 3 == 0 { (seed as usize >> 8) % 32 } else { (seed as usize >> 8) % copy.len() };
        copy[at] ^= (seed >> 40) as u8 | 1;
        if let Ok(k) = Kanji::parse(&copy) {
            let _ = k.convert("きょうはいいてんきですね", 3);
        }
    }
    let k = kanji();
    assert!(k.convert("", 3).is_empty());
    assert!(k.convert(&"あ".repeat(MAX_INPUT + 1), 3).is_empty());
    assert!(!k.convert("゛ゔっ", 3).is_empty());
}

#[test]
fn hiragana_mode_offers_kanji_then_katakana() {
    let mut u = ja_engine("ひ");
    assert_eq!(u.mode(), Mode::JaHira);
    type_kana(&mut u, "わたしは");
    assert_eq!(u.output().preedit, "わたしは");
    let c = u.candidates();
    eprintln!("わたしは: {c:?}");
    assert_eq!(c.len(), 4, "{c:?}");
    assert_eq!(c[0], "私は", "{c:?}");
    assert_eq!(c[3], "ワタシハ", "katakana takes the last slot: {c:?}");
    assert!(!c.contains(&"わたしは".to_string()), "the kana as typed is the preedit, not a candidate: {c:?}");
    // Picking one commits it and clears the composition.
    let o = u.candidate(0).clone();
    assert_eq!(o.commit, "私は");
    assert!(o.preedit.is_empty() && u.candidates().is_empty());
    // Space still commits the kana as typed (SPEC §6: never an automatic conversion).
    type_kana(&mut u, "にほんご");
    assert!(u.candidates().contains(&"日本語".to_string()), "{:?}", u.candidates());
    press_label(&mut u, "space");
    assert_eq!(u.output().commit, "にほんご ");
}

#[test]
fn katakana_mode_puts_katakana_first_and_hiragana_last() {
    let mut u = ja_engine("カ");
    assert_eq!(u.mode(), Mode::JaKata);
    type_kana(&mut u, "コーヒー".chars().map(|c| uim::kana::to_hiragana(&c.to_string())).collect::<String>().as_str());
    let c = u.candidates();
    eprintln!("コーヒー: {c:?}");
    assert_eq!(u.output().preedit, "コーヒー");
    assert!(!c.contains(&"コーヒー".to_string()), "{c:?}");
    assert_eq!(c.last().map(String::as_str), Some("こーひー"), "{c:?}");
    assert_eq!(c[0], "珈琲", "{c:?}");
    assert!(!c.iter().any(|t| t.contains("火ー")), "no ー after a kanji: {c:?}");
}

#[test]
fn a_keystroke_stays_fast() {
    // Every keystroke reconverts the whole composition: keep the worst case (a long one) small.
    let mut u = ja_engine("ひ");
    let t = Instant::now();
    type_kana(&mut u, "きょうはいいてんきですねあしたもはれるといいですね");
    let per = t.elapsed() / 25;
    eprintln!("per keystroke: {per:?}; {:?}", u.candidates());
    assert!(per.as_millis() < 250, "{per:?}");
}
