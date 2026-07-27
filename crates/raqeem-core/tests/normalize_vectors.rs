//! `normalize_ar` against the shared vectors in `tests/vectors/normalize_ar.json`.
//!
//! Those vectors are generated from scout's `normalize_ar` (`scout/arabic.py`) and are
//! read by *both* this suite and the Python binding's. That is deliberate: the two
//! implementations previously listed their cases separately by hand, and the port drifted
//! a whole pass behind the reference without either suite noticing. A case added to the
//! JSON is a case both languages must satisfy — there is nothing to remember to copy.

use raqeem_core::normalize_ar;

#[derive(serde::Deserialize)]
struct Vector {
    input: String,
    expected: String,
    note: String,
}

fn vectors() -> Vec<Vector> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/vectors/normalize_ar.json"
    );
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("shared vectors missing at {path}: {e}"));
    serde_json::from_str(&raw).expect("vectors are valid JSON")
}

#[test]
fn matches_the_reference_on_every_shared_vector() {
    let vectors = vectors();
    assert!(!vectors.is_empty(), "vector file is empty");

    let mut failures = Vec::new();
    for v in &vectors {
        let got = normalize_ar(&v.input);
        if got != v.expected {
            failures.push(format!(
                "  {:?}\n    expected {:?}\n    got      {:?}\n    ({})",
                v.input, v.expected, got, v.note
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} shared vectors diverge from scout's reference:\n{}",
        failures.len(),
        vectors.len(),
        failures.join("\n")
    );
}

#[test]
fn every_shared_vector_is_idempotent() {
    for v in vectors() {
        let twice = normalize_ar(&v.expected);
        assert_eq!(
            twice, v.expected,
            "not idempotent: {:?} ({})",
            v.input, v.note
        );
    }
}
