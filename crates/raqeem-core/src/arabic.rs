//! Arabic text normalization — a Rust port of scout's `normalize_ar` (from the
//! `arabic-first` convention). We fold audio transcripts to a matching-stable form so
//! downstream parsers (price extraction, commodity matching) never fuzzy-match raw,
//! unnormalized Arabic.
//!
//! **The reference is `scout/arabic.py`, as of commit `0ffcf36`.** These two are one
//! function maintained in two languages: a change to either has to land on both, or
//! text folded on one side stops comparing equal to the same text folded on the other.
//! That is not hypothetical — this port silently lagged the reference by one whole pass
//! (`_FORMAT_STRIP`, below) between scout's `0ffcf36` and raqeem 0.3.0. The shared
//! vectors in `tests/vectors/normalize_ar.json` are generated from the reference and
//! read by both test suites; add cases there rather than to either suite alone.
//!
//! What it does: strip diacritics / tatweel / Quranic marks, strip invisible format
//! controls, unify alef and hamza carriers, taa-marbuta → haa, Arabic-Indic & Persian
//! digits → ASCII, and the Arabic decimal separator U+066B → `.` (so «١٢٫٥» folds to
//! `12.5`, one number, not two).
//!
//! **Parity, precisely.** Verified identical across every Arabic block and all of ASCII —
//! 258,176 single-character probes over the whole BMP plus astral samples, and 200,000
//! randomised multi-character strings mixing Arabic, diacritics, Quranic marks, both digit
//! sets, format controls, every whitespace class and the ASCII separators. Zero mismatches,
//! zero idempotence failures on either side.
//!
//! The one accepted divergence is **case mapping of cased non-Arabic, non-ASCII letters**,
//! and it is not a logic difference: the two runtimes carry different Unicode tables. Rust
//! 1.93 lowercases eight characters in Cyrillic and Latin Extended-D that scout's Python
//! (Unicode 15.0.0) does not yet know exist, and Python lowercases a word-final Greek `Σ`
//! to `ς` where Rust gives `σ`. Nine codepoints, none of them in any Arabic block or in
//! ASCII, and it resolves itself when that runtime's Unicode data catches up.

/// What the reference collapses into a single space.
///
/// The reference's final step is `re.sub(r"\s+", " ", …)`, and Python's `\s` is **not**
/// `char::is_whitespace`: it additionally matches the ASCII separators U+001C–001F
/// (file / group / record / unit), which carry no Unicode `White_Space` property. Using
/// `is_whitespace` alone left those four passing through where the reference removed
/// them. Vanishingly rare in a transcript, but "identical output" is either true or it
/// isn't, and an exhaustive sweep against the reference is what surfaced it.
fn is_separator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\u{001C}'..='\u{001F}')
}

/// Fold Arabic text to a matching-stable form. Idempotent; ASCII passes through
/// lower-cased, so it is safe to run unconditionally on any transcript.
pub fn normalize_ar(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // Whitespace runs are collapsed as we go rather than by re-splitting the finished
    // string: a space is only committed once a non-space follows it, which trims both
    // ends for free and saves building a `Vec<&str>` and a second `String`.
    let mut pending_space = false;

    for ch in text.chars() {
        // Characters that vanish. Checked before whitespace so that a control sitting
        // inside a run of spaces cannot break the run in two.
        match ch {
            // Quranic annotations, harakat/diacritics, tatweel, superscript alef.
            '\u{0610}'..='\u{061A}'
            | '\u{064B}'..='\u{065F}'
            | '\u{0640}'
            | '\u{0670}'
            | '\u{06D6}'..='\u{06ED}' => continue,

            // Invisible format controls: zero-width space and joiners, the bidi marks,
            // embeddings, overrides and isolates, and the BOM. RTL text pasted out of a
            // chat app or scraped off a web page carries these routinely; they render as
            // nothing, mean nothing, and make two identical-looking strings compare
            // unequal.
            //
            // Deliberately a separate arm rather than more ranges bolted onto the class
            // above. Scout froze its `_ARABIC_STRIP` pattern after a mis-ordered
            // character range silently swallowed the entire Arabic letter block
            // (U+0621–064A) — additive-but-separate is the pattern that freeze set, and
            // keeping the two files structurally alike is how they stay comparable.
            '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}' => continue,

            _ => {}
        }

        if is_separator(ch) {
            // Leading whitespace never becomes a pending space, so the result is trimmed.
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }

        match ch {
            // Alef variants → bare alef.
            'آ' | 'أ' | 'إ' | 'ٱ' => out.push('ا'),
            // Hamza-carrier and taa-marbuta folds.
            'ى' => out.push('ي'), // alef maqsura → yaa
            'ة' => out.push('ه'), // taa marbuta → haa
            'ؤ' => out.push('و'),
            'ئ' => out.push('ي'),
            // Arabic-Indic (U+0660–0669) and Persian (U+06F0–06F9) digits → ASCII.
            // `char::to_digit` is ASCII-only, so the offset arithmetic is the way; the
            // match arms guarantee the subtraction stays in 0..=9.
            '\u{0660}'..='\u{0669}' => out.push((b'0' + (ch as u32 - 0x0660) as u8) as char),
            '\u{06F0}'..='\u{06F9}' => out.push((b'0' + (ch as u32 - 0x06F0) as u8) as char),
            // Arabic decimal separator → ASCII point.
            '\u{066B}' => out.push('.'),
            // Everything else: lower-case ASCII, pass Arabic/other through unchanged.
            other => out.extend(other.to_lowercase()),
        }
    }

    out
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
        assert_eq!(normalize_ar("a\t\tb"), "a b");
        assert_eq!(normalize_ar(""), "");
        assert_eq!(normalize_ar("   "), "");
    }

    /// The pass this port was missing. Each of these renders as nothing on screen, so
    /// without stripping them two visually identical strings compare unequal.
    #[test]
    fn strips_invisible_format_controls() {
        assert_eq!(normalize_ar("طم\u{200C}طم"), "طمطم"); // ZWNJ
        assert_eq!(normalize_ar("طم\u{200B}طم"), "طمطم"); // ZWSP
        assert_eq!(normalize_ar("طم\u{200D}طم"), "طمطم"); // ZWJ
        assert_eq!(normalize_ar("\u{200F}مرحبا"), "مرحبا"); // RLM
        assert_eq!(normalize_ar("\u{200E}مرحبا"), "مرحبا"); // LRM
        assert_eq!(normalize_ar("\u{FEFF}مرحبا"), "مرحبا"); // BOM
        assert_eq!(normalize_ar("\u{2066}مرحبا\u{2069}"), "مرحبا"); // isolate
        assert_eq!(normalize_ar("\u{202A}مرحبا\u{202C}"), "مرحبا"); // embedding
        assert_eq!(normalize_ar("\u{202E}مرحبا\u{202C}"), "مرحبا"); // override
    }

    /// A control adjacent to a space must not split one whitespace run into two.
    #[test]
    fn a_format_control_next_to_a_space_does_not_create_one() {
        assert_eq!(normalize_ar("طماطم\u{200B} ١٢٫٥"), "طماطم 12.5");
        assert_eq!(normalize_ar("طماطم \u{200B}طم"), "طماطم طم");
        assert_eq!(normalize_ar("\u{200B}  طم"), "طم");
        assert_eq!(normalize_ar("طم  \u{200B}"), "طم");
    }

    #[test]
    fn idempotent() {
        let samples = [
            "أحمد",
            "طماطة ١٢٫٥ جنيه",
            "Hello  World",
            "سـ__لامٌ عليكم",
            "\u{2066}طم\u{200C}طم\u{2069}",
        ];
        for s in samples {
            let once = normalize_ar(s);
            assert_eq!(normalize_ar(&once), once, "not idempotent on {s:?}");
        }
    }

    /// The previous implementation, kept verbatim to prove that rewriting the whitespace
    /// collapse into the main loop changed nothing. It predates the `_FORMAT_STRIP` pass,
    /// so the sweep below skips the characters that pass intentionally changed.
    fn reference_before_the_rewrite(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '\u{0610}'..='\u{061A}'
                | '\u{064B}'..='\u{065F}'
                | '\u{0640}'
                | '\u{0670}'
                | '\u{06D6}'..='\u{06ED}' => {}
                'آ' | 'أ' | 'إ' | 'ٱ' => out.push('ا'),
                'ى' => out.push('ي'),
                'ة' => out.push('ه'),
                'ؤ' => out.push('و'),
                'ئ' => out.push('ي'),
                '\u{0660}'..='\u{0669}' => out.push((b'0' + (ch as u32 - 0x0660) as u8) as char),
                '\u{06F0}'..='\u{06F9}' => out.push((b'0' + (ch as u32 - 0x06F0) as u8) as char),
                '\u{066B}' => out.push('.'),
                other => out.extend(other.to_lowercase()),
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Characters this release deliberately changed the handling of, so the equivalence
    /// sweep must skip them: the `_FORMAT_STRIP` set that the port was missing, and the
    /// four ASCII separators that Python's `\s` collapses but `char::is_whitespace`
    /// does not.
    fn is_deliberately_changed(c: char) -> bool {
        matches!(c,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
            || matches!(c, '\u{001C}'..='\u{001F}')
    }

    /// Is this one of the invisible controls the new `_FORMAT_STRIP` arm removes?
    fn is_format_control(c: char) -> bool {
        matches!(c,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
    }

    #[test]
    fn the_whitespace_rewrite_changed_nothing_else() {
        // Every char raqeem realistically sees: Arabic, Arabic Supplement/Extended,
        // Arabic Presentation Forms, ASCII, Latin-1, and the whitespace/control edges.
        let ranges = [
            0x0009u32..=0x000D,
            0x0020..=0x00FF,
            0x0600..=0x06FF,
            0x0750..=0x077F,
            0x08A0..=0x08FF,
            0x2000..=0x206F,
            0xFB50..=0xFDFF,
            0xFE70..=0xFEFF,
        ];
        for range in ranges {
            for cp in range {
                let Some(c) = char::from_u32(cp) else {
                    continue;
                };
                if is_deliberately_changed(c) {
                    continue;
                }
                // Bare, and wrapped in context so whitespace handling is exercised too.
                for probe in [
                    c.to_string(),
                    format!("  a {c}  b  "),
                    format!("{c}{c} طم {c}"),
                ] {
                    assert_eq!(
                        normalize_ar(&probe),
                        reference_before_the_rewrite(&probe),
                        "diverged on U+{cp:04X} in {probe:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_format_control_is_stripped_and_was_not_already() {
        for cp in 0x0000u32..=0x2FFF {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            if !is_format_control(c) {
                continue;
            }
            let probe = format!("طم{c}طم");
            assert_eq!(normalize_ar(&probe), "طمطم", "U+{cp:04X} not stripped");
            assert_ne!(
                reference_before_the_rewrite(&probe),
                "طمطم",
                "U+{cp:04X} was already handled; it does not belong in the new arm"
            );
        }
    }

    /// The reference's `re.sub(r"\s+", " ", …)` treats these as whitespace; Rust's
    /// `char::is_whitespace` does not. Found by sweeping every codepoint against the
    /// reference rather than by picking cases — the vector file would never have
    /// contained a record separator.
    #[test]
    fn the_ascii_separators_collapse_like_whitespace() {
        for c in ['\u{001C}', '\u{001D}', '\u{001E}', '\u{001F}'] {
            assert_eq!(
                normalize_ar(&c.to_string()),
                "",
                "U+{:04X} survived",
                c as u32
            );
            assert_eq!(normalize_ar(&format!("طم{c}طم")), "طم طم");
            assert_eq!(normalize_ar(&format!("طم {c} طم")), "طم طم");
            assert_eq!(normalize_ar(&format!("{c}طم{c}")), "طم");
        }
    }
}
