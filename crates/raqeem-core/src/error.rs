use std::path::PathBuf;

/// How much of an endpoint's response body to quote back in an error. Enough to show a
/// JSON error object or the top of an HTML error page; not so much that a misconfigured
/// endpoint returning a megabyte of markup floods the terminal.
const BODY_EXCERPT_LIMIT: usize = 512;

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

    #[error("could not parse endpoint response (expected a JSON object with a string \"text\" field), got: {body}")]
    BadResponse { body: String },
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
    use super::{excerpt, BODY_EXCERPT_LIMIT};

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
