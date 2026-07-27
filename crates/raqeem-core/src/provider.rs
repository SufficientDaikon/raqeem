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

    /// Every variant, so callers can enumerate the backends without matching by hand.
    pub const ALL: &'static [Provider] = &[Provider::Cohere, Provider::OpenAiCompatible];
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The name a caller typed didn't match any backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownProvider(pub String);

impl std::fmt::Display for UnknownProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown provider {:?} (expected one of: ", self.0)?;
        for (i, p) in Provider::ALL.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{p}")?;
        }
        f.write_str(")")
    }
}

impl std::error::Error for UnknownProvider {}

impl std::str::FromStr for Provider {
    type Err = UnknownProvider;

    /// Parse a backend name. `"openai"` is accepted alongside the canonical
    /// `"openai-compatible"` because that is the spelling the CLI and the Python
    /// binding have always exposed to users.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cohere" => Ok(Provider::Cohere),
            "openai" | "openai-compatible" => Ok(Provider::OpenAiCompatible),
            other => Err(UnknownProvider(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;

    #[test]
    fn every_tag_round_trips_through_from_str() {
        for &p in Provider::ALL {
            assert_eq!(
                p.as_str().parse::<Provider>(),
                Ok(p),
                "{p} did not round-trip"
            );
        }
    }

    #[test]
    fn the_user_facing_openai_spelling_is_accepted() {
        assert_eq!("openai".parse::<Provider>(), Ok(Provider::OpenAiCompatible));
    }

    #[test]
    fn an_unknown_name_lists_the_valid_ones() {
        let err = "nope".parse::<Provider>().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("cohere"), "{msg}");
    }
}
