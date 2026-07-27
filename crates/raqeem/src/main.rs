//! `raqeem` — رقيم. Transcribe Arabic audio from the command line.
//!
//! The binary is the universal calling surface: any language (scout's Python,
//! a shell script, a Node service) drives it by shelling out and reading stdout.

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use raqeem_core::{resolve_api_key, Endpoint, OutputFormat, Provider, Transcriber};

/// رقيم — transcribe Arabic audio with Cohere's open ASR model.
#[derive(Parser)]
#[command(
    name = "raqeem",
    version,
    about = "رقيم — transcribe Arabic audio via Cohere's open ASR model"
)]
struct Cli {
    /// Path to the audio file (flac / mp3 / mpeg / mpga / ogg / wav).
    audio: PathBuf,

    /// Backend: `cohere` (hosted API) or `openai` (self-hosted, needs --endpoint).
    #[arg(long, value_enum, default_value_t = ProviderArg::Cohere)]
    provider: ProviderArg,

    /// API key. Prefer $RAQEEM_API_KEY over this flag: an argument is visible to
    /// every other process on the machine (`ps`, /proc/<pid>/cmdline) and lands in
    /// shell history. For `--provider cohere` only, it also falls back to
    /// $COHERE_API_KEY — that Cohere-scoped key is never sent to a self-hosted
    /// `--endpoint`.
    //
    // hide_env_values is not optional here: clap defaults it to false, which renders
    // the *value* of a set env var into --help. That printed the live API key to
    // anyone who pasted help output into a CI log or an issue.
    #[arg(long, env = "RAQEEM_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    /// Full endpoint URL for a self-hosted OpenAI-compatible server, e.g.
    /// http://localhost:8000/v1/audio/transcriptions (required for --provider openai).
    #[arg(long)]
    endpoint: Option<String>,

    /// Model id to request. For `--provider cohere` this defaults to
    /// cohere-transcribe-arabic-07-2026 (Cohere needs a dated id; undated aliases 404).
    /// Required for `--provider openai`, where only your server knows its own model ids.
    #[arg(long)]
    model: Option<String>,

    /// Transcription language (ISO-639-1).
    #[arg(long, default_value = "ar")]
    lang: String,

    /// Total request timeout in seconds — covers upload + inference + download.
    /// Raise it for slow CPU inference or long recordings.
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..=86_400))]
    timeout: u64,

    /// Output format.
    #[arg(long, value_enum, default_value_t = FormatArg::Text)]
    format: FormatArg,
}

/// The backend names the CLI exposes. Distinct from [`Provider`] so that clap's derive
/// stays out of the core crate; the [`From`] impl below is the only mapping between them.
#[derive(Clone, Copy, ValueEnum)]
enum ProviderArg {
    Cohere,
    Openai,
}

impl From<ProviderArg> for Provider {
    fn from(arg: ProviderArg) -> Self {
        match arg {
            ProviderArg::Cohere => Provider::Cohere,
            ProviderArg::Openai => Provider::OpenAiCompatible,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Text,
    Json,
}

impl From<FormatArg> for OutputFormat {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Text => OutputFormat::Text,
            FormatArg::Json => OutputFormat::Json,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(out) => match write_line(&out) {
            Ok(()) => ExitCode::SUCCESS,
            // `raqeem clip.wav | head -1` closes the pipe on us. That is a normal way for
            // a CLI in a pipeline to end, not a failure — and `println!` panicked on it.
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("خطأ: {e}");
                ExitCode::FAILURE
            }
        },
        Err(msg) => {
            eprintln!("خطأ: {msg}");
            ExitCode::FAILURE
        }
    }
}

/// Write the transcript to stdout. Flushes explicitly: piped stdout is block-buffered,
/// so a broken pipe surfaces at the flush rather than at the write.
fn write_line(out: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{out}")?;
    lock.flush()
}

fn run(cli: Cli) -> std::result::Result<String, String> {
    // `--endpoint` only means something for a self-hosted server. Accepting it under
    // `--provider cohere` and ignoring it sent the audio and the key to Cohere while the
    // user believed they were talking to their own box.
    if matches!(cli.provider, ProviderArg::Cohere) && cli.endpoint.is_some() {
        return Err(
            "--endpoint only applies to --provider openai; --provider cohere always \
                    posts to Cohere's hosted API. Add --provider openai to use your own server."
                .to_string(),
        );
    }

    let api_key = resolve_api_key(
        cli.provider.into(),
        // clap has already folded $RAQEEM_API_KEY into this via `env =`.
        cli.api_key,
        std::env::var("COHERE_API_KEY").ok(),
    );

    let endpoint = match cli.provider {
        ProviderArg::Cohere => {
            let key = api_key.ok_or(
                "cohere provider needs an API key (--api-key, or $RAQEEM_API_KEY / $COHERE_API_KEY)",
            )?;
            Endpoint::cohere(key, cli.model)
        }
        ProviderArg::Openai => {
            let url = cli
                .endpoint
                .ok_or("--provider openai needs --endpoint <url>")?;
            // Deliberately not defaulting to Cohere's dated model id: your server has its
            // own ids, and sending Cohere's just fails at the server with a confusing 404.
            let model = cli
                .model
                .ok_or("--provider openai needs --model <id> (your server's model name)")?;
            Endpoint::openai_compatible(url, model, api_key)
        }
    };

    let transcript = Transcriber::with_timeout(endpoint, Duration::from_secs(cli.timeout))
        .and_then(|t| t.language(cli.lang).transcribe(&cli.audio))
        .map_err(|e| e.to_string())?;

    Ok(OutputFormat::from(cli.format).render(&transcript))
}

#[cfg(test)]
mod tests {
    use super::{Cli, FormatArg, ProviderArg};
    use clap::{CommandFactory, Parser};
    use raqeem_core::{resolve_api_key, OutputFormat, Provider};

    /// The credential rule itself is owned and tested by `raqeem-core`
    /// (`credentials.rs`). What the CLI has to get right is handing that rule the correct
    /// provider — so this asserts the mapping, not the rule.
    #[test]
    fn provider_arg_maps_to_the_core_provider() {
        assert_eq!(Provider::from(ProviderArg::Cohere), Provider::Cohere);
        assert_eq!(
            Provider::from(ProviderArg::Openai),
            Provider::OpenAiCompatible
        );
    }

    #[test]
    fn format_arg_maps_to_the_core_format() {
        assert_eq!(OutputFormat::from(FormatArg::Text), OutputFormat::Text);
        assert_eq!(OutputFormat::from(FormatArg::Json), OutputFormat::Json);
    }

    /// Wiring check: a Cohere-scoped key must not reach a self-hosted endpoint. Duplicated
    /// deliberately at one case — if the CLI ever passes the wrong provider through, the
    /// core tests would still pass while this one fails.
    #[test]
    fn the_cohere_key_does_not_reach_a_self_hosted_endpoint() {
        assert_eq!(
            resolve_api_key(ProviderArg::Openai.into(), None, Some("cohere-key".into())),
            None
        );
        assert_eq!(
            resolve_api_key(ProviderArg::Cohere.into(), None, Some("cohere-key".into())),
            Some("cohere-key".into())
        );
    }

    #[test]
    fn the_arg_definitions_are_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn timeout_rejects_zero_and_accepts_a_normal_value() {
        assert!(Cli::try_parse_from(["raqeem", "a.wav", "--timeout", "0"]).is_err());
        let cli = Cli::try_parse_from(["raqeem", "a.wav", "--timeout", "600"]).unwrap();
        assert_eq!(cli.timeout, 600);
    }

    #[test]
    fn endpoint_under_cohere_is_rejected_rather_than_ignored() {
        let cli = Cli::try_parse_from([
            "raqeem",
            "a.wav",
            "--endpoint",
            "http://localhost:8000/v1/audio/transcriptions",
        ])
        .expect("parses; the rejection is a runtime check, not a clap one");
        let err = super::run(cli).unwrap_err();
        assert!(err.contains("--endpoint"), "{err}");
        assert!(err.contains("--provider openai"), "{err}");
    }

    #[test]
    fn openai_requires_an_explicit_model() {
        let cli = Cli::try_parse_from([
            "raqeem",
            "a.wav",
            "--provider",
            "openai",
            "--endpoint",
            "http://localhost:8000/v1/audio/transcriptions",
        ])
        .unwrap();
        let err = super::run(cli).unwrap_err();
        assert!(err.contains("--model"), "{err}");
    }
}
