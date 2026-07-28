//! `raqeem-core` — رقيم. Turn an audio file into Arabic text by delegating
//! inference to a transcription endpoint (Cohere's hosted API, or your own
//! vLLM). This crate never loads model weights: it decodes nothing, runs no
//! model — it POSTs the audio and folds the result. That is what keeps it
//! light and callable from any language over the CLI.
//!
//! ```no_run
//! use raqeem_core::{Endpoint, Transcriber};
//!
//! let endpoint = Endpoint::cohere(std::env::var("COHERE_API_KEY").unwrap(), None);
//! let transcript = Transcriber::new(endpoint)?
//!     .language("ar")
//!     .transcribe(std::path::Path::new("voice_note.ogg"))?;
//! println!("{}", transcript.text);
//! # Ok::<(), raqeem_core::Error>(())
//! ```

#![forbid(unsafe_code)]

mod arabic;
mod credentials;
mod endpoint;
mod error;
mod output;
mod provider;

pub use arabic::normalize_ar;
pub use credentials::resolve_api_key;
pub use endpoint::{Endpoint, COHERE_URL, DEFAULT_COHERE_MODEL};
pub use error::{Error, Result};
pub use output::OutputFormat;
pub use provider::{Provider, UnknownProvider};

use std::path::Path;
use std::time::Duration;

/// Default transcription language (ISO-639-1). Arabic-first, by design.
pub const DEFAULT_LANGUAGE: &str = "ar";

/// Default total-request timeout, in seconds. Generous on purpose: it must cover
/// upload + model inference + download, and CPU inference of a voice note can take
/// many seconds. (reqwest's blocking client otherwise silently defaults to 30s,
/// which truncates a slow transcription mid-inference.)
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Time allowed to establish the TCP/TLS connection, within the total timeout above.
/// Separate because a host that never answers should fail in seconds, not in minutes.
pub const CONNECT_TIMEOUT_SECS: u64 = 15;

/// Largest response body accepted from an endpoint, in bytes.
///
/// A transcript is kilobytes; 64 MiB is roughly six hundred times more headroom than any
/// real one needs. The cap exists because everything downstream — the UTF-8 decode, the
/// JSON parse, [`normalize_ar`] — allocates again, so an endpoint returning 200 MB cost
/// ~580 MB of resident memory before this was here.
pub const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

/// A completed transcription. `text` is the model's verbatim output; the folded
/// `text_normalized` (see [`normalize_ar`]) is what downstream parsers consume.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Transcript {
    pub text: String,
    pub text_normalized: String,
    pub provider: String,
    pub model: String,
    pub language: String,
}

/// Transcribes audio by POSTing it to a configured [`Endpoint`].
pub struct Transcriber {
    endpoint: Endpoint,
    language: String,
    client: reqwest::blocking::Client,
}

impl Transcriber {
    /// Build a transcriber for the given endpoint, defaulting to Arabic and a
    /// [`DEFAULT_TIMEOUT_SECS`] total-request timeout.
    pub fn new(endpoint: Endpoint) -> Result<Self> {
        Self::with_timeout(endpoint, Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }

    /// Like [`new`](Self::new) but with an explicit total-request timeout — raise it
    /// for slow CPU inference or large files, lower it to fail faster. The timeout
    /// spans the whole round-trip (connect + upload + inference + download).
    ///
    /// Fails only if the HTTP client cannot be built, which in practice means the TLS
    /// backend would not initialize. This used to fall back to `Client::new()` on
    /// error, which is not a fallback at all: that constructor panics on the very same
    /// failure, and had it succeeded it would have installed reqwest's default 30s
    /// timeout — silently truncating exactly the slow inference this parameter exists
    /// to accommodate.
    pub fn with_timeout(endpoint: Endpoint, timeout: Duration) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            // A black-holed host would otherwise burn the entire total timeout — five
            // minutes by default — before admitting it never connected.
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            // Redirects are never followed. reqwest's default is to follow up to ten,
            // and for a client posting to one known API route that only causes harm:
            // a 302/303 downgrades POST to GET and drops the body, so the audio is never
            // uploaded and whatever comes back gets parsed as a transcript. See
            // `Error::Redirect`.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|source| Error::Client { source })?;
        Ok(Transcriber {
            endpoint,
            language: DEFAULT_LANGUAGE.to_string(),
            client,
        })
    }

    /// Override the transcription language (ISO-639-1).
    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }

    /// Stream `audio` to the endpoint and return the transcript.
    pub fn transcribe(&self, audio: &Path) -> Result<Transcript> {
        let read_err = |source| Error::ReadFile {
            path: audio.to_path_buf(),
            source,
        };
        // Opened and streamed rather than read into a Vec: an hour of 16-bit 44.1kHz WAV
        // is ~600MB, and buffering it made peak memory scale with the recording.
        let file = std::fs::File::open(audio).map_err(read_err)?;
        let len = file.metadata().map_err(read_err)?.len();

        let filename = audio
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio")
            .to_string();

        // Same multipart shape for every backend: model + language + file.
        // Order matters: Cohere rejects the request unless the text fields
        // (model, language) appear BEFORE the file part in the body.
        //
        // reader_with_length, not reader: the length is what keeps this a
        // Content-Length body. Without it reqwest switches to chunked transfer-encoding,
        // which not every endpoint accepts.
        let file_part =
            reqwest::blocking::multipart::Part::reader_with_length(file, len).file_name(filename);
        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.endpoint.model.clone())
            .text("language", self.language.clone())
            .part("file", file_part);

        let mut req = self.client.post(&self.endpoint.url).multipart(form);
        if let Some(key) = &self.endpoint.api_key {
            req = req.bearer_auth(key);
        }

        let http_err = |source: reqwest::Error| Error::Http {
            // Redacted, and reqwest's own copy dropped: it appends " for url (...)" to its
            // Display, which would print the endpoint twice — credentials included.
            url: error::redact_url(&self.endpoint.url),
            source: source.without_url(),
        };

        let resp = req.send().map_err(http_err)?;
        let status = resp.status();

        // Refuse a redirect rather than follow it. See `Error::Redirect` for why following
        // is worse than failing here.
        if status.is_redirection() {
            return Err(Error::Redirect {
                status: status.as_u16(),
                location: resp
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    // A redirect target can carry credentials too.
                    .map(error::redact_url),
            });
        }

        // Bound the response before reading it. `Content-Length` is the cheap check; a
        // chunked reply advertises none, so that case is caught after the fact instead.
        if let Some(len) = resp.content_length() {
            if len > MAX_RESPONSE_BYTES {
                return Err(Error::ResponseTooLarge {
                    got: len,
                    limit: MAX_RESPONSE_BYTES,
                });
            }
        }
        let body = resp.text().map_err(http_err)?;
        if body.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(Error::ResponseTooLarge {
                got: body.len() as u64,
                limit: MAX_RESPONSE_BYTES,
            });
        }
        // Anything the endpoint sent back may be quoted into an error below, so take the
        // caller's own key out of it first.
        let body = error::scrub_secret(body, self.endpoint.api_key.as_deref());
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body: error::excerpt(body),
            });
        }

        let text = match extract_text(&body) {
            Some(text) => text,
            None => {
                return Err(Error::BadResponse {
                    body: error::excerpt(body),
                })
            }
        };
        Ok(Transcript {
            text_normalized: normalize_ar(&text),
            text,
            provider: self.endpoint.provider.as_str().to_string(),
            model: self.endpoint.model.clone(),
            language: self.language.clone(),
        })
    }
}

/// The OpenAI-standard transcription response, which vLLM and (per Cohere's docs) the
/// hosted API both return. Unknown fields are ignored, so a backend sending extra
/// metadata alongside `text` still parses.
#[derive(serde::Deserialize)]
struct Response {
    text: String,
}

/// Pull the transcript out of an endpoint response.
///
// Assumes the `{"text": "..."}` shape. If a backend nests it differently, add a branch
// here rather than a whole response type — an excerpt of the raw body is surfaced in
// `Error::BadResponse` so a mismatch is obvious on first run.
fn extract_text(body: &str) -> Option<String> {
    serde_json::from_str::<Response>(body).ok().map(|r| r.text)
}

#[cfg(test)]
mod tests {
    use super::{Transcriber, Transcript};

    use super::extract_text;

    /// The README tells people to share one `Transcriber` across threads instead of
    /// building one per file, which is only sound if this holds. Compile-time check: it
    /// fails to build, not at runtime.
    #[test]
    fn transcriber_and_transcript_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Transcriber>();
        assert_send_sync::<Transcript>();
    }

    #[test]
    fn pulls_text_and_ignores_extra_fields() {
        let body = r#"{"text": "مرحبا", "duration": 1.5, "language": "ar"}"#;
        assert_eq!(extract_text(body).as_deref(), Some("مرحبا"));
    }

    #[test]
    fn rejects_responses_that_are_not_a_transcript() {
        // Each of these used to be indistinguishable from "no text field" — they still
        // are, but the point is that none of them panic or yield a bogus transcript.
        assert_eq!(extract_text("not json at all"), None);
        assert_eq!(extract_text(r#"{"error": "model not found"}"#), None);
        assert_eq!(extract_text(r#"{"text": 42}"#), None);
        assert_eq!(extract_text(r#"{"text": null}"#), None);
        assert_eq!(extract_text("[]"), None);
        assert_eq!(extract_text(""), None);
    }
}
