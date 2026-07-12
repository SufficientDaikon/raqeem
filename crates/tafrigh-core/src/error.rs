use std::path::PathBuf;

/// Everything that can go wrong on the way from an audio file to a transcript.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read audio file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("request to {url} failed: {source}")]
    Http { url: String, source: reqwest::Error },

    #[error("endpoint returned HTTP {status}: {body}")]
    Api { status: u16, body: String },

    #[error("could not parse endpoint response (expected a JSON object with a string \"text\" field), got: {body}")]
    BadResponse { body: String },
}

pub type Result<T> = std::result::Result<T, Error>;
