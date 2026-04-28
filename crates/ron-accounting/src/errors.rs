//! RO:WHAT — Typed error taxonomy for ron-accounting core, export, config, and WAL paths.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Deterministic errors keep adapters honest.
//! RO:INTERACTS — all modules through Result<T>; HTTP/OAP adapters map variants to wire errors.
//! RO:INVARIANTS — non_exhaustive enum; no string matching required; IO stays wrapped.
//! RO:METRICS — callers map variants into rejected/export failure counters.
//! RO:CONFIG — validation failures use SchemaViolation.
//! RO:SECURITY — avoids leaking secrets; messages should remain operational, not sensitive.
//! RO:TEST — unit: config/recording/exporter tests assert specific variants.

use thiserror::Error;

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Deterministic error taxonomy for accounting, export, config, and WAL operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A bounded hot-path resource rejected new work.
    #[error("accounting resource is busy")]
    Busy,

    /// Ordered export could not accept a slice without breaking sequence order.
    #[error("ordered export buffer overflow or sequence gap")]
    OrderOverflow,

    /// An operation exceeded its configured deadline.
    #[error("operation timed out")]
    Timeout,

    /// The downstream exporter reported an idempotent duplicate.
    #[error("duplicate export")]
    DuplicateExport,

    /// Persistence quota or bounded staging capacity was exceeded.
    #[error("persistence quota exceeded")]
    PersistenceFull,

    /// WAL replay or checksum validation failed.
    #[error("wal corrupt: {0}")]
    WalCorrupt(String),

    /// Strict schema validation failed.
    #[error("schema violation: {0}")]
    SchemaViolation(String),

    /// Filesystem or OS I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Fallback for non-contractual internal errors.
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Construct a schema violation with a stable message string.
    pub fn schema(msg: impl Into<String>) -> Self {
        Self::SchemaViolation(msg.into())
    }

    /// Construct a fallback error with a stable message string.
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
