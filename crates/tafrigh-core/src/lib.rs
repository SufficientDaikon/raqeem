//! `tafrigh-core` — تفريغ. Turn an audio file into Arabic text by delegating
//! inference to a transcription endpoint (Cohere's hosted API, or your own
//! vLLM). This crate never loads model weights: it decodes nothing, runs no
//! model — it POSTs the audio and folds the result. That is what keeps it
//! light and callable from any language over the CLI.
//!
//! ```no_run
//! use tafrigh_core::{Endpoint, Transcriber};
//!
//! let endpoint = Endpoint::cohere(std::env::var("COHERE_API_KEY").unwrap(), None);
//! let transcript = Transcriber::new(endpoint)
//!     .language("ar")
//!     .transcribe(std::path::Path::new("voice_note.ogg"))?;
//! println!("{}", transcript.text);
//! # Ok::<(), tafrigh_core::Error>(())
//! ```

mod arabic;
mod endpoint;
mod error;
mod output;
mod provider;

pub use arabic::normalize_ar;
pub use endpoint::{Endpoint, COHERE_URL, DEFAULT_COHERE_MODEL};
pub use error::{Error, Result};
pub use output::OutputFormat;
pub use provider::Provider;

use std::path::Path;

/// Default transcription language (ISO-639-1). Arabic-first, by design.
pub const DEFAULT_LANGUAGE: &str = "ar";

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
    /// Build a transcriber for the given endpoint, defaulting to Arabic.
    pub fn new(endpoint: Endpoint) -> Self {
        Transcriber {
            endpoint,
            language: DEFAULT_LANGUAGE.to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Override the transcription language (ISO-639-1).
    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }

    /// Read `audio`, send it to the endpoint, and return the transcript.
    pub fn transcribe(&self, audio: &Path) -> Result<Transcript> {
        let bytes = std::fs::read(audio).map_err(|source| Error::ReadFile {
            path: audio.to_path_buf(),
            source,
        })?;
        let filename = audio
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio")
            .to_string();

        // Same multipart shape for every backend: file + model + language.
        let file_part = reqwest::blocking::multipart::Part::bytes(bytes).file_name(filename);
        let form = reqwest::blocking::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.endpoint.model.clone())
            .text("language", self.language.clone());

        let mut req = self.client.post(&self.endpoint.url).multipart(form);
        if let Some(key) = &self.endpoint.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req.send().map_err(|source| Error::Http {
            url: self.endpoint.url.clone(),
            source,
        })?;
        let status = resp.status();
        let body = resp.text().map_err(|source| Error::Http {
            url: self.endpoint.url.clone(),
            source,
        })?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body,
            });
        }

        let text = extract_text(&body).ok_or_else(|| Error::BadResponse { body: body.clone() })?;
        Ok(Transcript {
            text_normalized: normalize_ar(&text),
            text,
            provider: self.endpoint.provider.as_str().to_string(),
            model: self.endpoint.model.clone(),
            language: self.language.clone(),
        })
    }
}

/// Pull the transcript out of an endpoint response.
///
// ponytail: assumes the OpenAI-standard `{"text": "..."}` shape, which vLLM and
// (per Cohere's docs) the hosted API both return. If a backend nests it
// differently, add a branch here rather than a whole response type — the raw
// body is surfaced in `Error::BadResponse` so a mismatch is obvious on first run.
fn extract_text(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("text")?.as_str().map(str::to_string)
}
