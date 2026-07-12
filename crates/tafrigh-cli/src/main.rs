//! `tafrigh` — تفريغ. Transcribe Arabic audio from the command line.
//!
//! The binary is the universal calling surface: any language (scout's Python,
//! a shell script, a Node service) drives it by shelling out and reading stdout.

use std::path::PathBuf;
use std::process::ExitCode;

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

    /// API key. Falls back to $TAFRIGH_API_KEY, then $COHERE_API_KEY.
    #[arg(long, env = "TAFRIGH_API_KEY")]
    api_key: Option<String>,

    /// Full endpoint URL for a self-hosted OpenAI-compatible server, e.g.
    /// http://localhost:8000/v1/audio/transcriptions (required for --provider openai).
    #[arg(long)]
    endpoint: Option<String>,

    /// Model id to request (defaults to cohere-transcribe-arabic).
    #[arg(long)]
    model: Option<String>,

    /// Transcription language (ISO-639-1).
    #[arg(long, default_value = "ar")]
    lang: String,

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

fn run(cli: Cli) -> std::result::Result<String, String> {
    let api_key = cli.api_key.or_else(|| std::env::var("COHERE_API_KEY").ok());

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

    let transcript = Transcriber::new(endpoint)
        .language(cli.lang)
        .transcribe(&cli.audio)
        .map_err(|e| e.to_string())?;

    Ok(format.render(&transcript))
}
