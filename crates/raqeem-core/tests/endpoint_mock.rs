//! End-to-end test of the transcribe path against a mocked HTTP endpoint — no
//! network, no API key, no model. Asserts the multipart body carries the fields
//! every backend expects and that an Arabic response parses + normalizes.

use std::io::Write;

use raqeem_core::{Endpoint, Error, Transcriber};

/// A throwaway audio file that removes itself, including when a test panics.
/// `TempDir` gives each call its own directory, so concurrent runs can't collide.
fn fake_clip(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("clip.wav");
    let mut f = std::fs::File::create(&path).expect("write clip");
    f.write_all(bytes).expect("write clip");
    (dir, path)
}

#[test]
fn posts_multipart_with_auth_and_parses_arabic_text() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/v2/audio/transcriptions")
        .match_header("authorization", "Bearer testkey")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::Regex("cohere-transcribe-arabic".to_string()),
            // Order matters: Cohere requires model + language BEFORE the file part.
            mockito::Matcher::Regex(
                r#"(?s)name="model".*name="language".*name="file""#.to_string(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        // Arabic-Indic digits + U+066B on purpose, to prove normalization runs.
        .with_body(r#"{"text": "الطماطم بـ ١٢٫٥ جنيه"}"#)
        .create();

    let url = format!("{}/v2/audio/transcriptions", server.url());
    let endpoint =
        Endpoint::openai_compatible(url, "cohere-transcribe-arabic", Some("testkey".into()))
            .unwrap();

    let (_dir, audio) = fake_clip(b"RIFF....WAVEfake");

    let t = Transcriber::new(endpoint)
        .expect("client builds")
        .language("ar")
        .transcribe(&audio)
        .expect("transcribe should succeed against the mock");

    assert_eq!(t.text, "الطماطم بـ ١٢٫٥ جنيه"); // verbatim, untouched
    assert!(t.text_normalized.contains("12.5")); // digits + decimal folded
    assert!(!t.text_normalized.contains('ـ')); // tatweel stripped
    assert_eq!(t.language, "ar");
    assert_eq!(t.provider, "openai-compatible");

    mock.assert();
}

/// The upload streams from the file handle rather than a buffered `Vec`. It must still
/// send a `Content-Length` body — `Part::reader` without a length would switch reqwest to
/// chunked transfer-encoding, which not every endpoint accepts.
#[test]
fn streams_the_file_with_a_content_length_not_chunked() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/v1/audio/transcriptions")
        .match_header("transfer-encoding", mockito::Matcher::Missing)
        .match_header("content-length", mockito::Matcher::Any)
        .match_body(mockito::Matcher::Regex(
            r#"(?s)name="file".*RIFFSTREAMINGPAYLOAD"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"text": "تم"}"#)
        .create();

    let url = format!("{}/v1/audio/transcriptions", server.url());
    let endpoint = Endpoint::openai_compatible(url, "m", None).unwrap();
    let (_dir, audio) = fake_clip(b"RIFFSTREAMINGPAYLOAD");

    let t = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .expect("transcribe should succeed");

    assert_eq!(t.text, "تم");
    mock.assert();
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
    let endpoint = Endpoint::openai_compatible(url, "m", None).unwrap();
    let (_dir, audio) = fake_clip(b"x");

    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .unwrap_err();

    assert!(matches!(err, Error::Api { status: 401, .. }), "{err:?}");
    let msg = err.to_string();
    assert!(msg.contains("401"), "expected status in error, got: {msg}");
    assert!(msg.contains("unauthorized"), "expected body, got: {msg}");
}

/// A huge error body must not be pasted into the terminal wholesale.
#[test]
fn truncates_a_giant_error_body() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/v1/audio/transcriptions")
        .with_status(500)
        .with_body("E".repeat(100_000))
        .create();

    let url = format!("{}/v1/audio/transcriptions", server.url());
    let endpoint = Endpoint::openai_compatible(url, "m", None).unwrap();
    let (_dir, audio) = fake_clip(b"x");

    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.len() < 2_000,
        "error body not truncated: {} chars",
        msg.len()
    );
    assert!(msg.contains("bytes total"), "{msg}");
}

/// A 200 whose body isn't a transcript is a distinct failure from an HTTP error, and the
/// body has to reach the user or the mismatch is undiagnosable.
#[test]
fn a_success_without_a_text_field_is_a_bad_response() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/v1/audio/transcriptions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result": {"transcript": "nested somewhere else"}}"#)
        .create();

    let url = format!("{}/v1/audio/transcriptions", server.url());
    let endpoint = Endpoint::openai_compatible(url, "m", None).unwrap();
    let (_dir, audio) = fake_clip(b"x");

    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .unwrap_err();

    assert!(matches!(err, Error::BadResponse { .. }), "{err:?}");
    assert!(
        err.to_string().contains("nested somewhere else"),
        "the raw body is what makes this diagnosable: {err}"
    );
}

#[test]
fn a_missing_audio_file_names_the_path() {
    let endpoint = Endpoint::openai_compatible("http://127.0.0.1:9/v1", "m", None).unwrap();
    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(std::path::Path::new("definitely-not-here.wav"))
        .unwrap_err();

    assert!(matches!(err, Error::ReadFile { .. }), "{err:?}");
    assert!(err.to_string().contains("definitely-not-here.wav"), "{err}");
}

/// The worst defect this project has had: a 302 makes reqwest downgrade POST to GET and
/// drop the body, so the audio is never uploaded and whatever the target answers gets
/// parsed and printed as a transcript. It must be an error, never a result.
#[test]
fn a_redirect_is_refused_rather_than_producing_a_fabricated_transcript() {
    for status in [301u16, 302, 303, 307, 308] {
        let mut server = mockito::Server::new();
        let _redirect = server
            .mock("POST", "/v1/audio/transcriptions")
            .with_status(status.into())
            .with_header("location", "/somewhere-else")
            .create();
        // Present but must never be reached: if the client follows, it lands here and
        // returns this as though it were a real transcription.
        let _trap = server
            .mock("GET", "/somewhere-else")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"text": "fabricated"}"#)
            .create();

        let url = format!("{}/v1/audio/transcriptions", server.url());
        let endpoint = Endpoint::openai_compatible(url, "m", None).unwrap();
        let (_dir, audio) = fake_clip(b"RIFF");

        let err = Transcriber::new(endpoint)
            .expect("client builds")
            .transcribe(&audio)
            .unwrap_err();

        assert!(
            matches!(err, Error::Redirect { .. }),
            "HTTP {status} should be refused, got {err:?}"
        );
        let msg = err.to_string();
        assert!(!msg.contains("fabricated"), "followed the redirect: {msg}");
        assert!(
            msg.contains("somewhere-else"),
            "should name Location: {msg}"
        );
    }
}

/// An endpoint that reflects the Authorization header into its error body must not get the
/// caller's own key printed back at them.
#[test]
fn a_reflected_api_key_never_reaches_the_error_message() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/v1/audio/transcriptions")
        .with_status(401)
        .with_body(r#"{"error": "bad key: Bearer sk-LEAKME-9999"}"#)
        .create();

    let url = format!("{}/v1/audio/transcriptions", server.url());
    let endpoint = Endpoint::openai_compatible(url, "m", Some("sk-LEAKME-9999".into())).unwrap();
    let (_dir, audio) = fake_clip(b"RIFF");

    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .unwrap_err();

    let msg = err.to_string();
    assert!(!msg.contains("sk-LEAKME-9999"), "key echoed back: {msg}");
    assert!(msg.contains("bad key"), "diagnosis should survive: {msg}");
}

/// Credentials in the endpoint URL must not reach the error text either.
#[test]
fn url_credentials_never_reach_the_error_message() {
    // Port 9 is discard: the connection fails, which is the path that builds Error::Http.
    let endpoint =
        Endpoint::openai_compatible("http://bob:hunter2@127.0.0.1:9/v1", "m", None).unwrap();
    let (_dir, audio) = fake_clip(b"RIFF");

    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .unwrap_err();

    let msg = err.to_string();
    assert!(!msg.contains("hunter2"), "password leaked: {msg}");
    assert!(!msg.contains("bob"), "username leaked: {msg}");
    assert!(msg.contains("127.0.0.1"), "host should survive: {msg}");
}

/// An oversized response is refused before it can be decoded, parsed and normalized —
/// each of which allocates the whole thing again.
#[test]
fn an_oversized_response_is_refused() {
    let mut server = mockito::Server::new();
    let huge = "E".repeat((raqeem_core::MAX_RESPONSE_BYTES + 1) as usize);
    let _mock = server
        .mock("POST", "/v1/audio/transcriptions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(huge)
        .create();

    let url = format!("{}/v1/audio/transcriptions", server.url());
    let endpoint = Endpoint::openai_compatible(url, "m", None).unwrap();
    let (_dir, audio) = fake_clip(b"RIFF");

    let err = Transcriber::new(endpoint)
        .expect("client builds")
        .transcribe(&audio)
        .unwrap_err();

    assert!(matches!(err, Error::ResponseTooLarge { .. }), "{err:?}");
}
