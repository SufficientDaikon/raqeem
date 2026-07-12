//! End-to-end test of the transcribe path against a mocked HTTP endpoint — no
//! network, no API key, no model. Asserts the multipart body carries the fields
//! every backend expects and that an Arabic response parses + normalizes.

use tafrigh_core::{Endpoint, Transcriber};

#[test]
fn posts_multipart_with_auth_and_parses_arabic_text() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/v2/audio/transcriptions")
        .match_header("authorization", "Bearer testkey")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::Regex("cohere-transcribe-arabic".to_string()),
            // Order matters: Cohere requires model + language BEFORE the file part.
            mockito::Matcher::Regex(r#"(?s)name="model".*name="language".*name="file""#.to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        // Arabic-Indic digits + U+066B on purpose, to prove normalization runs.
        .with_body(r#"{"text": "الطماطم بـ ١٢٫٥ جنيه"}"#)
        .create();

    let url = format!("{}/v2/audio/transcriptions", server.url());
    let endpoint =
        Endpoint::openai_compatible(url, "cohere-transcribe-arabic", Some("testkey".into()));

    let audio = std::env::temp_dir().join("tafrigh_test_clip.wav");
    std::fs::write(&audio, b"RIFF....WAVEfake").unwrap();

    let t = Transcriber::new(endpoint)
        .language("ar")
        .transcribe(&audio)
        .expect("transcribe should succeed against the mock");

    assert_eq!(t.text, "الطماطم بـ ١٢٫٥ جنيه"); // verbatim, untouched
    assert!(t.text_normalized.contains("12.5")); // digits + decimal folded
    assert!(!t.text_normalized.contains('ـ')); // tatweel stripped
    assert_eq!(t.language, "ar");
    assert_eq!(t.provider, "openai-compatible");

    mock.assert();
    let _ = std::fs::remove_file(&audio);
}

#[test]
fn surfaces_api_errors_with_status_and_body() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/v1/audio/transcriptions")
        .with_status(401)
        .with_body("unauthorized")
        .create();

    let url = format!("{}/v1/audio/transcriptions", server.url());
    let endpoint = Endpoint::openai_compatible(url, "m", None);

    let audio = std::env::temp_dir().join("tafrigh_test_clip2.wav");
    std::fs::write(&audio, b"x").unwrap();

    let err = Transcriber::new(endpoint).transcribe(&audio).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("401"), "expected status in error, got: {msg}");
    let _ = std::fs::remove_file(&audio);
}
