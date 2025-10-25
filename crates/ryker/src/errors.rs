//! RO:WHAT — Typed error taxonomy for ryker (mailbox/runtime/config).
//! RO:WHY  — Deterministic mapping for hosts; Concerns: RES/DX.
//! RO:INTERACTS — mailbox (Busy/TooLarge/Closed/Timeout), config loader.
//! RO:INVARIANTS — errors stable across minors; strings are non-sensitive.

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("mailbox at capacity (Busy)")]
    Busy,
    #[error("message too large (max {max} bytes)")]
    TooLarge { max: usize },
    #[error("mailbox closed")]
    Closed,
    #[error("deadline exceeded")]
    Timeout,
    #[error("configuration error: {0}")]
    Config(ConfigError),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid value: {0}")]
    Invalid(String),
    #[error("unsupported in production: {0}")]
    ProdGuard(String),
}

// Allow `?` on ConfigError to bubble as Error::Config
impl From<ConfigError> for Error {
    fn from(e: ConfigError) -> Self {
        Error::Config(e)
    }
}
