//! `tafrigh` — تفريغ. Transcribe Arabic audio from the command line.
//!
//! The binary is the universal calling surface: any language (scout's Python,
//! a shell script, a Node service) drives it by shelling out and reading stdout.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use tafrigh_core::{Endpoint, OutputFormat, Transcriber, DEFAULT_COHERE_MODEL};

/// تفريغ — transcribe Arabic audio with Cohere's open ASR model.
#[derive(Parser)]
#[command(
    name = "tafrigh",
    version,
    about = "تفريغ — transcribe Arabic audio via Cohere's open ASR model"
)]
struct Cli {
    /// Path to the audio file (flac / mp3 / mpeg / mpga / ogg / wav).
    audio: PathBuf,

    /// Backend: `cohere` (hosted API) or `openai` (self-hosted, needs --endpoint).
    #[arg(long, value_enum, default_value_t = ProviderArg::Cohere)]
    provider: ProviderArg,

    /// API key. Falls back to $TAFRIGH_API_KEY. For `--provider cohere` only, it
    /// also falls back to $COHERE_API_KEY — that Cohere-scoped key is never sent to
    /// a self-hosted `--endpoint`.
    #[arg(long, env = "TAFRIGH_API_KEY")]
    api_key: Option<String>,

    /// Full endpoint URL for a self-hosted OpenAI-compatible server, e.g.
    /// http://localhost:8000/v1/audio/transcriptions (required for --provider openai).
    #[arg(long)]
    endpoint: Option<String>,

    /// Model id to request. Cohere needs a dated id; defaults to
    /// cohere-transcribe-arabic-07-2026 (undated aliases 404).
    #[arg(long)]
    model: Option<String>,

    /// Transcription language (ISO-639-1).
    #[arg(long, default_value = "ar")]
    lang: String,

    /// Total request timeout in seconds — covers upload + inference + download.
    /// Raise it for slow CPU inference or long recordings.
    #[arg(long, default_value_t = 300)]
    timeout: u64,

    /// Output format.
    #[arg(long, value_enum, default_value_t = FormatArg::Text)]
    format: FormatArg,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProviderArg {
    Cohere,
    Openai,
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("خطأ: {msg}");
            ExitCode::FAILURE
        }
    }
}

/// The API key to send, given the provider and the two already-read sources.
/// `explicit` is `--api-key` (clap also folds in `$TAFRIGH_API_KEY` via `env=`).
/// Only the Cohere provider may fall back to the Cohere-scoped `$COHERE_API_KEY`;
/// a self-hosted `--endpoint` must never receive it, or that key leaks to a
/// third-party server.
fn api_key_for(
    provider: ProviderArg,
    explicit: Option<String>,
    cohere_env: Option<String>,
) -> Option<String> {
    match provider {
        ProviderArg::Cohere => explicit.or(cohere_env),
        ProviderArg::Openai => explicit,
    }
}

fn run(cli: Cli) -> std::result::Result<String, String> {
    let api_key = api_key_for(
        cli.provider,
        cli.api_key,
        std::env::var("COHERE_API_KEY").ok(),
    );

    let endpoint = match cli.provider {
        ProviderArg::Cohere => {
            let key = api_key.ok_or(
                "cohere provider needs an API key (--api-key, or $TAFRIGH_API_KEY / $COHERE_API_KEY)",
            )?;
            Endpoint::cohere(key, cli.model)
        }
        ProviderArg::Openai => {
            let url = cli
                .endpoint
                .ok_or("--provider openai needs --endpoint <url>")?;
            let model = cli
                .model
                .unwrap_or_else(|| DEFAULT_COHERE_MODEL.to_string());
            Endpoint::openai_compatible(url, model, api_key)
        }
    };

    let format = match cli.format {
        FormatArg::Text => OutputFormat::Text,
        FormatArg::Json => OutputFormat::Json,
    };

    let transcript = Transcriber::with_timeout(endpoint, Duration::from_secs(cli.timeout))
        .language(cli.lang)
        .transcribe(&cli.audio)
        .map_err(|e| e.to_string())?;

    Ok(format.render(&transcript))
}

#[cfg(test)]
mod tests {
    use super::{api_key_for, ProviderArg};

    #[test]
    fn cohere_falls_back_to_cohere_env() {
        // no explicit key → the Cohere-scoped env is used
        assert_eq!(
            api_key_for(ProviderArg::Cohere, None, Some("cohere-key".into())),
            Some("cohere-key".into())
        );
        // an explicit key (--api-key / $TAFRIGH_API_KEY) wins over the fallback
        assert_eq!(
            api_key_for(
                ProviderArg::Cohere,
                Some("explicit".into()),
                Some("cohere-key".into())
            ),
            Some("explicit".into())
        );
    }

    #[test]
    fn openai_never_uses_cohere_env() {
        // a self-hosted endpoint must NOT receive the Cohere-scoped key
        assert_eq!(
            api_key_for(ProviderArg::Openai, None, Some("cohere-key".into())),
            None
        );
        // but an explicit key set for that endpoint is still honored
        assert_eq!(
            api_key_for(
                ProviderArg::Openai,
                Some("mykey".into()),
                Some("cohere-key".into())
            ),
            Some("mykey".into())
        );
    }
}
