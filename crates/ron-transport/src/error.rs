//! RO:WHAT — Error types for ron-transport.
//! RO:WHY  — Stable taxonomy for callers (deterministic).
//! RO:INTERACTS — reason::RejectReason.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("bind error: {0}")]
    Bind(std::io::Error),
    #[error("accept loop failed: {0}")]
    Accept(std::io::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("limit exceeded: {0}")]
    Limit(&'static str),
    #[error("timeout")]
    Timeout,
    #[error("tls error")]
    Tls,
    #[error("closed")]
    Closed,
}
