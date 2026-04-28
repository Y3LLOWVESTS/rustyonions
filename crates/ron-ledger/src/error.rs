//! RO:WHAT — Typed ledger errors and the stable reject taxonomy used by DTOs, tests, and future service wrappers.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/GOV. Reject reasons must be machine-countable and stable.
//! RO:INTERACTS — crate::api, crate::engine, crate::types.
//! RO:INVARIANTS — reject taxonomy is append-only in practice; malformed input maps to Invalid; IO errors stay distinct.
//! RO:METRICS — future service wrappers should map RejectReason directly into reject counters.
//! RO:CONFIG — none.
//! RO:SECURITY — redaction belongs outside; this layer never stores secrets inside error values.
//! RO:TEST — reject_taxonomy.rs and interop_vectors.rs assert taxonomy stability and malformed-input behavior.

use thiserror::Error;

/// Stable, machine-countable reject reasons for the library surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RejectReason {
    /// Input shape, field value, or invariant validation failed.
    Invalid,
    /// Request exceeded configured bounds.
    TooLarge,
    /// External wait or deadline would have been exceeded.
    Timeout,
    /// Duplicate or otherwise conflicting write.
    Conflict,
}

impl RejectReason {
    /// Stable string label for future metrics adapters.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::TooLarge => "too_large",
            Self::Timeout => "timeout",
            Self::Conflict => "conflict",
        }
    }
}

/// Ledger crate error.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// Reject with a stable reason and small human message.
    #[error("{reason:?}: {message}")]
    Reject {
        /// Stable reject reason.
        reason: RejectReason,
        /// Human-readable context.
        message: String,
    },
    /// Storage or filesystem failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON encode/decode failure for file-backed storage or strict DTO parsing.
    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    /// Base64 decode failure.
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    /// Hex decode failure.
    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
}

impl LedgerError {
    /// Construct a reject error.
    pub fn reject(reason: RejectReason, message: impl Into<String>) -> Self {
        Self::Reject {
            reason,
            message: message.into(),
        }
    }

    /// Extract the reject reason when present.
    ///
    /// Parsing/decoding failures are classified as `Invalid` so callers and tests
    /// can map malformed input into the stable reject taxonomy.
    pub const fn reject_reason(&self) -> Option<RejectReason> {
        match self {
            Self::Reject { reason, .. } => Some(*reason),
            Self::SerdeJson(_) | Self::Base64(_) | Self::Hex(_) => Some(RejectReason::Invalid),
            Self::Io(_) => None,
        }
    }
}
