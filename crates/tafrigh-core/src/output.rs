//! Render a [`Transcript`] for a caller. `Text` is the human-readable verbatim
//! transcription; `Json` carries both the verbatim and normalized forms plus
//! provenance (what scout consumes).

use crate::Transcript;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn render(self, t: &Transcript) -> String {
        match self {
            OutputFormat::Text => t.text.clone(),
            OutputFormat::Json => {
                // Transcript derives Serialize; serialization of these plain
                // fields cannot fail, so the fallback is unreachable in practice.
                serde_json::to_string_pretty(t).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }
}
