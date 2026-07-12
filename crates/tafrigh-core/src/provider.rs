//! Backend presets. Every supported transcription server speaks the same
//! OpenAI-compatible multipart `/audio/transcriptions` shape, so a "provider"
//! is just a label + the defaults ([`crate::Endpoint`] carries the actual
//! URL / auth / model). Adding a backend = adding a variant here and a
//! constructor on `Endpoint` — see `.claude/skills/add-endpoint-adapter`.

/// A known transcription backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Cohere's hosted API — `https://api.cohere.com/v2/audio/transcriptions`.
    Cohere,
    /// Any self-hosted OpenAI-compatible server (e.g. vLLM at
    /// `/v1/audio/transcriptions`), reached via an explicit URL.
    OpenAiCompatible,
}

impl Provider {
    /// Stable machine-readable tag, recorded on every [`crate::Transcript`].
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Cohere => "cohere",
            Provider::OpenAiCompatible => "openai-compatible",
        }
    }
}
