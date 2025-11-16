//! RO:WHAT — Error taxonomy for ron-app-sdk.
//! RO:WHY  — Give applications a small, stable set of error classes
//!           they can reason about (timeouts vs caps vs conflicts).
//! RO:INTERACTS — Used across all planes and the transport shim;
//!                mapping from HTTP/OAP/wire happens here.
//! RO:INVARIANTS —
//!   - Enum is `#[non_exhaustive]` per API.md.
//!   - Retry classification is centralized here (I-5/I-8).
//! RO:SECURITY — Messages are safe for logs; no secrets included.

use std::{error::Error, fmt, time::Duration};

/// Stable, non-exhaustive SDK error type.
///
/// This matches the shape laid out in `docs/API.md`. New variants may
/// be added over time, but existing ones will not be removed or
/// renamed without a SemVer bump.
#[non_exhaustive]
#[derive(Debug)]
pub enum SdkError {
    /// Overall deadline for the operation was exceeded.
    DeadlineExceeded,

    /// Underlying transport failed (TCP, DNS, etc.).
    Transport(std::io::ErrorKind),

    /// TLS handshake/verification error.
    Tls,

    /// Tor was requested but not available/usable.
    TorUnavailable,

    /// OAP/1 protocol or bounds violation (e.g., frame too large).
    OapViolation {
        /// Human-readable reason string (static).
        reason: &'static str,
    },

    /// Capability has expired (e.g., `nbf`/`exp` window).
    CapabilityExpired,

    /// Capability does not grant access to the requested resource.
    CapabilityDenied,

    /// Schema/validation error at the SDK boundary.
    ///
    /// Examples: invalid `AddrB3` string, wrong DTO shape, etc.
    SchemaViolation {
        /// Logical path (e.g., `"addr_b3"`, `"payload.body"`).
        path: String,
        /// Short description of what went wrong.
        detail: String,
    },

    /// Resource not found (404-style).
    NotFound,

    /// Conflict (409-style) — usually idempotency or version clash.
    Conflict,

    /// Rate-limited by the remote service.
    ///
    /// Optional `retry_after` allows well-behaved exponential backoff
    /// or respect for concrete `Retry-After` hints when present.
    RateLimited { retry_after: Option<Duration> },

    /// Remote server error with raw status code.
    Server(u16),

    /// Catch-all for errors that don’t fit other variants yet.
    Unknown(String),
}

/// Coarse retry classification for SDK errors.
///
/// This keeps the retry/backoff logic in one place and lets callers
/// apply their own policies if they want something fancier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// Retrying *may* succeed (timeouts, 5xx, backpressure).
    Retriable,
    /// Retrying is not expected to help (caps, schema, conflicts).
    NoRetry,
}

impl SdkError {
    /// Create a schema violation error with structured fields.
    pub fn schema_violation(path: impl Into<String>, detail: impl Into<String>) -> Self {
        SdkError::SchemaViolation {
            path: path.into(),
            detail: detail.into(),
        }
    }

    /// Create a rate-limited error with optional retry hint.
    pub fn rate_limited(retry_after: Option<Duration>) -> Self {
        SdkError::RateLimited { retry_after }
    }

    /// Map an IO error into the transport bucket.
    pub fn from_io(err: std::io::Error) -> Self {
        SdkError::Transport(err.kind())
    }

    /// Classify this error for retry purposes.
    pub fn retry_class(&self) -> RetryClass {
        use RetryClass::{NoRetry, Retriable};

        match *self {
            SdkError::DeadlineExceeded => Retriable,
            SdkError::Transport(_) => Retriable,
            SdkError::RateLimited { .. } => Retriable,
            SdkError::Server(code) if (500..600).contains(&code) => Retriable,

            // Everything else we conservatively treat as non-retriable.
            SdkError::Tls
            | SdkError::TorUnavailable
            | SdkError::OapViolation { .. }
            | SdkError::CapabilityExpired
            | SdkError::CapabilityDenied
            | SdkError::SchemaViolation { .. }
            | SdkError::NotFound
            | SdkError::Conflict
            | SdkError::Server(_)
            | SdkError::Unknown(_) => NoRetry,
        }
    }

    /// Convenience helper.
    pub fn is_retriable(&self) -> bool {
        self.retry_class() == RetryClass::Retriable
    }
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SdkError::*;

        match self {
            DeadlineExceeded => write!(f, "deadline exceeded"),
            Transport(kind) => write!(f, "transport error ({kind:?})"),
            Tls => write!(f, "TLS error"),
            TorUnavailable => write!(f, "Tor transport unavailable"),
            OapViolation { reason } => write!(f, "OAP violation: {reason}"),
            CapabilityExpired => write!(f, "capability expired"),
            CapabilityDenied => write!(f, "capability denied"),
            SchemaViolation { path, detail } => {
                write!(f, "schema violation at `{path}`: {detail}")
            }
            NotFound => write!(f, "not found"),
            Conflict => write!(f, "conflict"),
            RateLimited { retry_after } => {
                if let Some(d) = retry_after {
                    write!(f, "rate limited (retry after {d:?})")
                } else {
                    write!(f, "rate limited")
                }
            }
            Server(code) => write!(f, "server error ({code})"),
            Unknown(msg) => write!(f, "unknown error: {msg}"),
        }
    }
}

impl Error for SdkError {}
