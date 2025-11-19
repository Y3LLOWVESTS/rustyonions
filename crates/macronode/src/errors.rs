//! RO:WHAT — Error type and Result alias for Macronode.
//! RO:WHY  — Keep error plumbing boring and consistent across modules.
//! RO:INVARIANTS —
//!   - All fallible public fns in this crate return `errors::Result<T>`.
//!   - Config parsing collapses to `Error::Config` with human-readable messages.

use thiserror::Error;

/// Crate-local Result alias.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// Configuration issues (env/file/cli overlays).
    #[error("config error: {0}")]
    Config(String),

    /// I/O errors (sockets, files, etc.).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// JSON serialization / formatting issues.
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// Catch-all for higher level composition until we tighten types.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::Config(msg.into())
    }
}
