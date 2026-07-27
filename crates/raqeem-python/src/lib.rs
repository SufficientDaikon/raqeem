//! PyO3 bindings — `import raqeem` from Python, backed by the same Rust core the
//! CLI uses. This layer holds **no logic of its own**: it maps Python arguments
//! onto [`raqeem_core::Transcriber`] and maps errors onto one Python exception.
//! Anything smarter belongs in `raqeem-core`, so every binding inherits it.

use std::path::PathBuf;
use std::time::Duration;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use raqeem_core::{
    resolve_api_key, Endpoint, Provider, Transcriber, UnknownProvider, DEFAULT_TIMEOUT_SECS,
};

pyo3::create_exception!(
    raqeem,
    TranscriptionError,
    pyo3::exceptions::PyException,
    "Raised when transcription fails: unreadable audio, transport error, an HTTP\n\
     error from the endpoint, or an unparseable response."
);

/// A completed transcription. Output-only: Python never converts *into* this type,
/// so it deliberately does not derive `Clone`/`FromPyObject`.
#[pyclass(frozen, module = "raqeem")]
struct Transcript {
    /// The model's verbatim output — show this to a human.
    #[pyo3(get)]
    text: String,
    /// Arabic-folded form (see `normalize_ar`) — parse/match against this.
    #[pyo3(get)]
    text_normalized: String,
    #[pyo3(get)]
    provider: String,
    #[pyo3(get)]
    model: String,
    #[pyo3(get)]
    language: String,
}

#[pymethods]
impl Transcript {
    /// The transcript as a plain dict — same shape the CLI's `--format json` emits.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        d.set_item("text", &self.text)?;
        d.set_item("text_normalized", &self.text_normalized)?;
        d.set_item("provider", &self.provider)?;
        d.set_item("model", &self.model)?;
        d.set_item("language", &self.language)?;
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "Transcript(text={:?}, language={:?}, provider={:?}, model={:?})",
            self.text, self.language, self.provider, self.model
        )
    }
}

/// Build the endpoint.
///
/// Which key each backend may be given is decided by [`resolve_api_key`] in
/// `raqeem-core`, not here. This function used to re-implement that rule and carried a
/// comment promising it "mirrored the CLI's credential rules exactly" — two copies of a
/// security rule, kept in step by hand. Reading the environment stays on this side,
/// because the core function is deliberately pure.
fn build_endpoint(
    provider: &str,
    api_key: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
) -> PyResult<Endpoint> {
    let provider: Provider = provider
        .parse()
        .map_err(|e: UnknownProvider| PyValueError::new_err(e.to_string()))?;

    // `$RAQEEM_API_KEY` is ours, so it counts as an explicit key for any backend;
    // `$COHERE_API_KEY` is Cohere's, and resolve_api_key is what keeps it there.
    let explicit = api_key.or_else(|| std::env::var("RAQEEM_API_KEY").ok());
    let key = resolve_api_key(provider, explicit, std::env::var("COHERE_API_KEY").ok());

    match provider {
        Provider::Cohere => {
            let key = key.ok_or_else(|| {
                PyValueError::new_err(
                    "provider='cohere' needs an API key (pass api_key=..., \
                     or set $RAQEEM_API_KEY / $COHERE_API_KEY)",
                )
            })?;
            Ok(Endpoint::cohere(key, model))
        }
        Provider::OpenAiCompatible => {
            let url = endpoint.ok_or_else(|| {
                PyValueError::new_err(
                    "provider='openai' needs endpoint=\
                     'http://host:8000/v1/audio/transcriptions'",
                )
            })?;
            // Not defaulting to Cohere's model id: your server has its own, and sending
            // Cohere's just fails at the server with a confusing error.
            let model = model.ok_or_else(|| {
                PyValueError::new_err(
                    "provider='openai' needs model='...' (your server's model name)",
                )
            })?;
            Ok(Endpoint::openai_compatible(url, model, key))
        }
    }
}

/// Transcribe an audio file and return a [`Transcript`].
#[pyfunction]
#[pyo3(signature = (
    path,
    *,
    lang = "ar",
    provider = "cohere",
    api_key = None,
    endpoint = None,
    model = None,
    timeout = None,
))]
#[allow(clippy::too_many_arguments)]
fn transcribe(
    py: Python<'_>,
    path: PathBuf,
    lang: &str,
    provider: &str,
    api_key: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
    timeout: Option<u64>,
) -> PyResult<Transcript> {
    let ep = build_endpoint(provider, api_key, endpoint, model)?;
    let secs = timeout.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let transcriber = Transcriber::with_timeout(ep, Duration::from_secs(secs))
        .map_err(|e| TranscriptionError::new_err(e.to_string()))?
        .language(lang);

    // Detach from the interpreter (release the GIL) for the blocking round-trip:
    // upload + inference + download can take minutes, and holding on would freeze
    // the caller's whole interpreter.
    let result = py.detach(|| transcriber.transcribe(&path));

    match result {
        Ok(t) => Ok(Transcript {
            text: t.text,
            text_normalized: t.text_normalized,
            provider: t.provider,
            model: t.model,
            language: t.language,
        }),
        Err(e) => Err(TranscriptionError::new_err(e.to_string())),
    }
}

/// Fold Arabic text to a matching-stable form (alef/hamza, taa-marbuta, tatweel and
/// diacritics stripped, Arabic-Indic & Persian digits to ASCII, `U+066B` to `.`).
/// Idempotent, and safe on mixed Arabic/ASCII.
#[pyfunction]
fn normalize_ar(text: &str) -> String {
    raqeem_core::normalize_ar(text)
}

#[pymodule]
fn raqeem(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("TranscriptionError", m.py().get_type::<TranscriptionError>())?;
    m.add_class::<Transcript>()?;
    m.add_function(wrap_pyfunction!(transcribe, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_ar, m)?)?;
    Ok(())
}
