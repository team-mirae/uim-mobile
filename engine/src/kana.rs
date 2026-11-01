//! Kana helpers for JA (SPEC §4.3, §4.4): script conversion and ゛ ゜.

/// Hiragana → katakana (U+3041–U+3096 shift by 0x60). Everything else is unchanged.
pub fn to_katakana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{3041}'..='\u{3096}' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// Katakana → hiragana (the inverse of [`to_katakana`]).
pub fn to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{30A1}'..='\u{30F6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn is_katakana(c: char) -> bool {
    ('\u{30A1}'..='\u{30F6}').contains(&c)
}

const DAKUTEN: &[(char, char)] = &[
    ('か', 'が'), ('き', 'ぎ'), ('く', 'ぐ'), ('け', 'げ'), ('こ', 'ご'),
    ('さ', 'ざ'), ('し', 'じ'), ('す', 'ず'), ('せ', 'ぜ'), ('そ', 'ぞ'),
    ('た', 'だ'), ('ち', 'ぢ'), ('つ', 'づ'), ('て', 'で'), ('と', 'ど'),
    ('は', 'ば'), ('ひ', 'び'), ('ふ', 'ぶ'), ('へ', 'べ'), ('ほ', 'ぼ'),
    ('う', 'ゔ'),
];

const HANDAKUTEN: &[(char, char)] = &[('は', 'ぱ'), ('ひ', 'ぴ'), ('ふ', 'ぷ'), ('へ', 'ぺ'), ('ほ', 'ぽ')];

/// Applies ゛ (`handaku` false) or ゜ (true) to the last kana of `buf`, keeping its script. After
/// a kana it can't apply to, the mark is appended as is (SPEC §4.3).
pub fn apply_mark(buf: &mut String, handaku: bool) {
    let table = if handaku { HANDAKUTEN } else { DAKUTEN };
    if let Some(last) = buf.chars().last() {
        let kata = is_katakana(last);
        let hira = if kata { to_hiragana(&last.to_string()).chars().next().unwrap_or(last) } else { last };
        if let Some(&(_, marked)) = table.iter().find(|(base, _)| *base == hira) {
            buf.pop();
            let s = marked.to_string();
            buf.push_str(&if kata { to_katakana(&s) } else { s });
            return;
        }
    }
    buf.push(if handaku { '゜' } else { '゛' });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks() {
        let mut s = String::from("か");
        apply_mark(&mut s, false);
        assert_eq!(s, "が");
        let mut s = String::from("ハ");
        apply_mark(&mut s, true);
        assert_eq!(s, "パ");
        let mut s = String::from("う");
        apply_mark(&mut s, false);
        assert_eq!(s, "ゔ");
        let mut s = String::from("あ");
        apply_mark(&mut s, false);
        assert_eq!(s, "あ゛");
        assert_eq!(to_katakana("ひらがなー"), "ヒラガナー");
    }
}
