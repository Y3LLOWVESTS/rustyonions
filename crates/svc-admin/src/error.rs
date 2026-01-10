// crates/svc-admin/src/error.rs

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("upstream node error: {0}")]
    Upstream(String),

    /// Like `Upstream`, but preserves an upstream HTTP status code when
    /// we explicitly need to distinguish semantics (e.g., bench run_id not found).
    #[error("upstream node error: status {status}: {message}")]
    UpstreamStatus { status: u16, message: String },

    #[error("other: {0}")]
    Other(String),
}
