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
            url: self.endpoint.url.clone(),
            // Drop reqwest's own copy of the URL; we print it ourselves, and it can
            // carry userinfo credentials.
            source: source.without_url(),
        };

        let resp = req.send().map_err(http_err)?;
        let status = resp.status();
        let body = resp.text().map_err(http_err)?;
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
    use super::extract_text;

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
