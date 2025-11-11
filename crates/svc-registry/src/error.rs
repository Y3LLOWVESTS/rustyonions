//! Error types (intentionally small public surface). :contentReference[oaicite:10]{index=10}
use thiserror::Error;

/// Service-level error (non-exhaustive to allow additions without SemVer break). :contentReference[oaicite:11]{index=11}
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("configuration: {0}")]
    Config(String),
    #[error("storage: {0}")]
    Storage(String),
    #[error("busy")]
    Busy,
    #[error("unauthorized")]
    Unauthorized,
}
