//! EN predictive on the shipped word list (data/).
use std::time::Instant;
use uim::predict::{Dict, GROUPS};

fn seq(w: &str) -> Vec<u8> {
    w.chars().map(|c| GROUPS.iter().position(|g| g.contains(c)).unwrap() as u8).collect()
}

#[test]
fn shipped_data() {
    let t = Instant::now();
    let mut d = Dict::load_words(&std::fs::read_to_string("data/en-words.txt").unwrap());
    d.load_bigrams(&std::fs::read_to_string("data/en-bigrams.txt").unwrap());
    d.load_acronyms(&std::fs::read_to_string("data/en-acronyms.txt").unwrap());
    eprintln!("load: {:?}", t.elapsed());
    let t = Instant::now();
    for (prev, w) in [("", "the"), ("i", "want"), ("to", "put"), ("but", "you"), ("thank", "you"), ("", "keyboard")] {
        let (c, _) = d.candidates(&seq(w), prev, 4, false);
        eprintln!("{prev:>6} {w:>9}: {c:?}");
        assert!(c.contains(&w.to_string()), "{w}: {c:?}");
    }
    eprintln!("6 lookups: {:?}", t.elapsed());
}

#[test]
fn all_caps_on_shipped_data() {
    let mut d = Dict::load_words(&std::fs::read_to_string("data/en-words.txt").unwrap());
    d.load_bigrams(&std::fs::read_to_string("data/en-bigrams.txt").unwrap());
    d.load_acronyms(&std::fs::read_to_string("data/en-acronyms.txt").unwrap());
    for w in ["api", "gpu", "usa", "url", "pdf", "faq"] {
        let (lower, _) = d.candidates(&seq(w), "", 4, false);
        let (caps, _) = d.candidates(&seq(w), "", 4, true);
        eprintln!("{w:>4}: lower {lower:?} / caps {caps:?}");
        assert_eq!(caps.first().map(String::as_str), Some(w), "{w}: {caps:?}");
    }
    // Not all caps: unchanged ranking, so ordinary words stay ahead of acronyms on shared keys.
    assert_eq!(d.candidates(&seq("the"), "", 4, false).0[0], "the");
}
