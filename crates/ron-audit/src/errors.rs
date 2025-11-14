//! Error types for ron-audit.
//!
//! These are intentionally small and host-friendly; higher layers can wrap them
//! in richer error stacks if desired.

use std::io;

use thiserror::Error;

use crate::canon::CanonError;

/// Errors produced while appending audit records into a sink.
#[derive(Debug, Error)]
pub enum AppendError {
    /// The sink is full / backpressure threshold hit.
    #[error("audit sink is full or backpressured")]
    Full,

    /// The record exceeded size bounds (attrs or full record).
    #[error("audit record size exceeded bounds")]
    SizeExceeded,

    /// The sink detected tampering or chain breakage (e.g. prev/self mismatch).
    #[error("audit chain tamper detected")]
    Tamper,

    /// Underlying IO error from a WAL or storage layer.
    #[error("io error while appending audit record: {0}")]
    Io(#[from] io::Error),

    /// Canonical form violated schema or invariants.
    #[error("schema / canonicalization error")]
    Schema,
}

/// Errors produced while verifying canonical form or chain linkage.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// The computed hash did not match the record's `self_hash`.
    #[error("self hash mismatch")]
    HashMismatch,

    /// Canonicalization failed (NFC, floats, unknown fields, etc).
    #[error("canonicalization error: {0}")]
    Canon(#[from] CanonError),

    /// Chain linkage between two adjacent records failed.
    #[error("prev/self linkage mismatch")]
    LinkMismatch,
}

/// Errors produced by bounds checks (size limits).
#[derive(Debug, Error)]
pub enum BoundsError {
    /// `attrs` payload exceeded the configured maximum number of bytes.
    #[error("attrs too large: {actual} bytes (max {max})")]
    AttrsTooLarge {
        /// Actual serialized size in bytes.
        actual: usize,
        /// Configured maximum size in bytes.
        max: usize,
    },

    /// Full record exceeded the configured maximum number of bytes.
    #[error("record too large: {actual} bytes (max {max})")]
    RecordTooLarge {
        /// Actual serialized size in bytes.
        actual: usize,
        /// Configured maximum size in bytes.
        max: usize,
    },
}
