//! The one adapter. Cohere-hosted and self-hosted vLLM differ only in URL,
//! auth header, and model id — so a single `Endpoint` struct + two constructors
//! covers both, and any future OpenAI-compatible backend.

use crate::provider::Provider;

/// Cohere's hosted transcription endpoint.
pub const COHERE_URL: &str = "https://api.cohere.com/v2/audio/transcriptions";
/// Default model id for the Arabic transcriber. Cohere requires a **dated** model
/// id — undated aliases (`cohere-transcribe-arabic`) return HTTP 404 "model not
/// found". Bump this when Cohere ships a newer dated Arabic transcription model.
pub const DEFAULT_COHERE_MODEL: &str = "cohere-transcribe-arabic-07-2026";

/// Where and how to reach a transcription server.
///
/// `Debug` is implemented by hand rather than derived, so that printing an `Endpoint`
/// cannot disclose the key it holds — see the impl below.
#[derive(Clone)]
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

/// Hand-written so the API key can never be printed.
///
/// A derived `Debug` renders `api_key: Some("sk-…")` in full, which means any downstream
/// `dbg!(&endpoint)` or `tracing::debug!(?endpoint)` discloses the caller's credential.
/// That is a footgun a published library shouldn't hand out, so the field is masked and
/// only its presence is shown.
impl std::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("url", &crate::error::redact_url(&self.url))
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("model", &self.model)
            .field("provider", &self.provider)
            .finish()
    }
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
    ///
    /// The URL is parsed here so a typo fails immediately and by name. Left unchecked it
    /// surfaced much later as reqwest's `builder error`, which says nothing about which
    /// input was wrong.
    pub fn openai_compatible(
        url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> crate::Result<Self> {
        let url = url.into();
        if url.trim().is_empty() {
            return Err(crate::Error::InvalidEndpoint {
                url,
                reason: "it is empty".to_string(),
            });
        }
        let parsed = url::Url::parse(&url).map_err(|e| crate::Error::InvalidEndpoint {
            url: crate::error::redact_url(&url),
            reason: e.to_string(),
        })?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(crate::Error::InvalidEndpoint {
                url: crate::error::redact_url(&url),
                reason: format!("scheme {:?} is not http or https", parsed.scheme()),
            });
        }
        Ok(Endpoint {
            url,
            api_key,
            model: model.into(),
            provider: Provider::OpenAiCompatible,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Endpoint;

    #[test]
    fn debug_never_prints_the_api_key() {
        let ep = Endpoint::cohere("sk-SECRET-DO-NOT-PRINT", None);
        let shown = format!("{ep:?}");
        assert!(!shown.contains("sk-SECRET-DO-NOT-PRINT"), "{shown}");
        assert!(
            shown.contains("***"),
            "presence should still be visible: {shown}"
        );
    }

    #[test]
    fn debug_never_prints_url_credentials() {
        let ep = Endpoint::openai_compatible("https://bob:hunter2@host/v1", "m", None).unwrap();
        let shown = format!("{ep:?}");
        assert!(!shown.contains("hunter2"), "{shown}");
    }

    #[test]
    fn a_valid_url_is_accepted() {
        let ep = Endpoint::openai_compatible("http://localhost:8000/v1", "m", None).unwrap();
        assert_eq!(ep.url, "http://localhost:8000/v1");
    }

    #[test]
    fn a_bad_url_is_rejected_by_name_not_by_builder_error() {
        for bad in ["", "   ", "not-a-url", "ftp://host/v1", "/just/a/path"] {
            let err = Endpoint::openai_compatible(bad, "m", None)
                .expect_err("{bad:?} should be rejected");
            assert!(
                err.to_string().contains("endpoint"),
                "error should name the endpoint for {bad:?}: {err}"
            );
        }
    }
}
