//! Arabic text normalization — a Rust port of scout's verified `normalize_ar`
//! (from the `arabic-first` convention). We fold audio transcripts to a
//! matching-stable form so downstream parsers (price extraction, commodity
//! matching) never fuzzy-match raw, unnormalized Arabic.
//!
//! Kept byte-for-byte faithful to the Python reference: strip diacritics /
//! tatweel / Quranic marks, unify alef and hamza carriers, taa-marbuta → haa,
//! Arabic-Indic & Persian digits → ASCII, and the Arabic decimal separator
//! U+066B → `.` (so «١٢٫٥» folds to `12.5`, one number, not two).

/// Fold Arabic text to a matching-stable form. Idempotent; ASCII passes through
/// lower-cased, so it is safe to run unconditionally on any transcript.
pub fn normalize_ar(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            // Strip: Quranic annotations, harakat/diacritics, tatweel, superscript alef.
            '\u{0610}'..='\u{061A}'
            | '\u{064B}'..='\u{065F}'
            | '\u{0640}'
            | '\u{0670}'
            | '\u{06D6}'..='\u{06ED}' => {}
            // Alef variants → bare alef.
            'آ' | 'أ' | 'إ' | 'ٱ' => out.push('ا'),
            // Hamza-carrier and taa-marbuta folds.
            'ى' => out.push('ي'), // alef maqsura → yaa
            'ة' => out.push('ه'), // taa marbuta → haa
            'ؤ' => out.push('و'),
            'ئ' => out.push('ي'),
            // Arabic-Indic (U+0660–0669) and Persian (U+06F0–06F9) digits → ASCII.
            '\u{0660}'..='\u{0669}' => out.push((b'0' + (ch as u32 - 0x0660) as u8) as char),
            '\u{06F0}'..='\u{06F9}' => out.push((b'0' + (ch as u32 - 0x06F0) as u8) as char),
            // Arabic decimal separator → ASCII point.
            '\u{066B}' => out.push('.'),
            // Everything else: lower-case ASCII, pass Arabic/other through unchanged.
            other => out.extend(other.to_lowercase()),
        }
    }
    // Collapse internal whitespace runs and trim.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalize_ar;

    #[test]
    fn folds_alef_and_hamza_carriers() {
        assert_eq!(normalize_ar("أ"), "ا");
        assert_eq!(normalize_ar("إ"), "ا");
        assert_eq!(normalize_ar("آ"), "ا");
        assert_eq!(normalize_ar("ٱ"), "ا");
        assert_eq!(normalize_ar("مصطفى"), "مصطفي"); // alef maqsura → yaa
        assert_eq!(normalize_ar("مسؤول"), "مسوول"); // ؤ → و
    }

    #[test]
    fn taa_marbuta_becomes_haa() {
        let r = normalize_ar("طماطة");
        assert!(!r.contains('ة'));
        assert!(r.ends_with('ه'));
    }

    #[test]
    fn strips_tatweel_and_diacritics() {
        assert_eq!(normalize_ar("سـلام"), "سلام"); // tatweel U+0640
        assert_eq!(normalize_ar("سَلاَم"), "سلام"); // harakat
    }

    #[test]
    fn digits_and_decimal_separator() {
        assert_eq!(normalize_ar("١٢٣"), "123"); // arabic-indic
        assert_eq!(normalize_ar("۱۲۳"), "123"); // persian
        assert_eq!(normalize_ar("١٢٫٥"), "12.5"); // U+066B decimal → one number
    }

    #[test]
    fn ascii_lowercased_and_whitespace_collapsed() {
        assert_eq!(normalize_ar("Tomato"), "tomato");
        assert_eq!(normalize_ar("  a   b  "), "a b");
    }

    #[test]
    fn idempotent() {
        let samples = ["أحمد", "طماطة ١٢٫٥ جنيه", "Hello  World", "سـلامٌ عليكم"];
        for s in samples {
            let once = normalize_ar(s);
            assert_eq!(normalize_ar(&once), once, "not idempotent on {s:?}");
        }
    }
}
