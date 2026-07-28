use std::path::PathBuf;

/// How much of an endpoint's response body to quote back in an error. Enough to show a
/// JSON error object or the top of an HTML error page; not so much that a misconfigured
/// endpoint returning a megabyte of markup floods the terminal.
const BODY_EXCERPT_LIMIT: usize = 512;

/// What a URL looks like once its credentials are removed.
const REDACTED: &str = "***";

/// Everything that can go wrong on the way from an audio file to a transcript.
///
/// Non-exhaustive: matching on it must carry a `_` arm, so that a future variant is not a
/// breaking change.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("could not read audio file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    /// The HTTP client could not be constructed — in practice, TLS backend
    /// initialization failing.
    #[error("could not build the HTTP client: {source}")]
    Client { source: reqwest::Error },

    // `source` is stripped of its URL before it gets here: reqwest appends
    // " for url (...)" to its own Display, which would print the endpoint a second time
    // and, if the URL carries userinfo credentials, print those to stderr twice over.
    #[error("request to {url} failed: {source}")]
    Http { url: String, source: reqwest::Error },

    #[error("endpoint returned HTTP {status}: {body}")]
    Api { status: u16, body: String },

    /// The endpoint answered with a redirect instead of a transcript.
    ///
    /// Deliberately not followed. A 302/303 downgrades POST to GET and drops the body, so
    /// the audio would never be uploaded and whatever the target returned would be parsed
    /// as a transcript — a silently wrong answer. A 307/308 preserves the method but
    /// cannot replay a streamed body. Either way the configuration is what needs fixing.
    #[error(
        "endpoint redirected (HTTP {status}{})—point --endpoint at the final URL instead; \
         redirects are not followed, because following one would either upload nothing or \
         fabricate a transcript from the wrong response",
        .location.as_ref().map(|l| format!(" to {l}")).unwrap_or_default()
    )]
    Redirect {
        status: u16,
        location: Option<String>,
    },

    /// The endpoint URL could not be used — empty, unparseable, or not http(s).
    #[error("--endpoint {url:?} is not a usable URL: {reason}")]
    InvalidEndpoint { url: String, reason: String },

    /// The response was larger than [`MAX_RESPONSE_BYTES`](crate::MAX_RESPONSE_BYTES).
    #[error(
        "endpoint response is too large ({got} bytes, limit {limit}); a transcript is \
             normally a few kilobytes, so this endpoint is probably not returning one"
    )]
    ResponseTooLarge { got: u64, limit: u64 },

    #[error("could not parse endpoint response (expected a JSON object with a string \"text\" field), got: {body}")]
    BadResponse { body: String },
}

/// Strip any credentials out of a URL before it goes anywhere a human can read.
///
/// `https://bob:hunter2@host/v1` becomes `https://***:***@host/v1`. Endpoints sitting
/// behind basic auth are ordinary, and an error message ends up in terminals, CI logs and
/// pasted issue reports.
///
/// A string that doesn't parse as a URL has no userinfo to leak, so it passes through —
/// which also keeps the message useful when the URL itself is what's malformed.
pub(crate) fn redact_url(raw: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(raw) else {
        return raw.to_string();
    };
    if parsed.username().is_empty() && parsed.password().is_none() {
        return raw.to_string();
    }
    // Both setters only fail on URLs that cannot have credentials (`mailto:` and such),
    // and those have none to begin with — so falling back to the parsed form is safe.
    let _ = parsed.set_username(REDACTED);
    let _ = parsed.set_password(Some(REDACTED));
    parsed.to_string()
}

/// Blank out a secret wherever it appears in text we're about to show someone.
///
/// An endpoint that reflects the `Authorization` header into its error body — hostile, or
/// merely verbose — would otherwise put the caller's own API key on their terminal. We
/// hold the key at that moment, so removing it costs one scan.
pub(crate) fn scrub_secret(text: String, secret: Option<&str>) -> String {
    match secret {
        Some(s) if !s.is_empty() => text.replace(s, REDACTED),
        _ => text,
    }
}

/// Trim a response body to something safe to put in an error message, marking it when cut.
///
/// Truncates on a character boundary — endpoint errors are often UTF-8 Arabic, and slicing
/// a `String` mid-codepoint panics.
pub(crate) fn excerpt(body: String) -> String {
    if body.len() <= BODY_EXCERPT_LIMIT {
        return body;
    }
    let cut = (0..=BODY_EXCERPT_LIMIT)
        .rev()
        .find(|&i| body.is_char_boundary(i))
        .unwrap_or(0);
    format!("{}… ({} bytes total)", &body[..cut], body.len())
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::{excerpt, redact_url, scrub_secret, BODY_EXCERPT_LIMIT};

    #[test]
    fn a_url_password_never_survives_redaction() {
        let out = redact_url("https://bob:hunter2@host/v1/audio/transcriptions");
        assert!(!out.contains("hunter2"), "{out}");
        assert!(!out.contains("bob"), "{out}");
        assert!(
            out.contains("host"),
            "the host is what makes it diagnosable: {out}"
        );
    }

    #[test]
    fn a_username_alone_is_still_redacted() {
        let out = redact_url("https://bob@host/v1");
        assert!(!out.contains("bob"), "{out}");
    }

    #[test]
    fn a_url_without_credentials_is_left_alone() {
        let plain = "https://api.cohere.com/v2/audio/transcriptions";
        assert_eq!(redact_url(plain), plain);
    }

    /// A malformed URL has no userinfo to leak, and echoing it verbatim is what makes the
    /// "your endpoint isn't a URL" case diagnosable.
    #[test]
    fn an_unparseable_url_passes_through() {
        assert_eq!(redact_url("not-a-url"), "not-a-url");
        assert_eq!(redact_url(""), "");
    }

    #[test]
    fn a_reflected_api_key_is_scrubbed_from_a_body() {
        let body = r#"{"error": "bad key: Bearer sk-LEAKME-9999"}"#.to_string();
        let out = scrub_secret(body, Some("sk-LEAKME-9999"));
        assert!(!out.contains("sk-LEAKME-9999"), "{out}");
        assert!(out.contains("bad key"), "the diagnosis survives: {out}");
    }

    #[test]
    fn scrubbing_without_a_key_changes_nothing() {
        let body = "plain failure".to_string();
        assert_eq!(scrub_secret(body.clone(), None), body);
        assert_eq!(scrub_secret(body.clone(), Some("")), body);
    }

    #[test]
    fn a_short_body_is_untouched() {
        assert_eq!(excerpt("unauthorized".into()), "unauthorized");
    }

    #[test]
    fn a_long_body_is_cut_and_marked() {
        let out = excerpt("x".repeat(BODY_EXCERPT_LIMIT * 3));
        assert!(
            out.len() < BODY_EXCERPT_LIMIT * 2,
            "still huge: {}",
            out.len()
        );
        assert!(out.contains("bytes total"), "{out}");
    }

    #[test]
    fn cutting_multibyte_arabic_does_not_panic() {
        // Every char is 2 bytes, so the limit lands mid-codepoint if sliced naively.
        let out = excerpt("ط".repeat(BODY_EXCERPT_LIMIT));
        assert!(out.contains("bytes total"), "{out}");
    }
}
