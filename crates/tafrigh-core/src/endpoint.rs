//! The one adapter. Cohere-hosted and self-hosted vLLM differ only in URL,
//! auth header, and model id — so a single `Endpoint` struct + two constructors
//! covers both, and any future OpenAI-compatible backend.

use crate::provider::Provider;

/// Cohere's hosted transcription endpoint.
pub const COHERE_URL: &str = "https://api.cohere.com/v2/audio/transcriptions";
/// Default model id for the Arabic transcriber.
pub const DEFAULT_COHERE_MODEL: &str = "cohere-transcribe-arabic";

/// Where and how to reach a transcription server.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Full URL to POST the multipart form to.
    pub url: String,
    /// Bearer token, if the backend requires auth.
    pub api_key: Option<String>,
    /// Model id sent as the `model` form field.
    pub model: String,
    /// Provenance tag recorded on the transcript.
    pub provider: Provider,
}

impl Endpoint {
    /// Cohere's hosted API. `model` defaults to [`DEFAULT_COHERE_MODEL`].
    pub fn cohere(api_key: impl Into<String>, model: Option<String>) -> Self {
        Endpoint {
            url: COHERE_URL.to_string(),
            api_key: Some(api_key.into()),
            model: model.unwrap_or_else(|| DEFAULT_COHERE_MODEL.to_string()),
            provider: Provider::Cohere,
        }
    }

    /// A self-hosted OpenAI-compatible endpoint — pass the full route URL
    /// (e.g. `http://localhost:8000/v1/audio/transcriptions`).
    pub fn openai_compatible(
        url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Endpoint {
            url: url.into(),
            api_key,
            model: model.into(),
            provider: Provider::OpenAiCompatible,
        }
    }
}
