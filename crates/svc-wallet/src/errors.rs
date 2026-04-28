//! RO:WHAT — Stable wallet error taxonomy and deterministic status mapping.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/DX/GOV. API errors must be stable, countable, and OpenAPI-aligned.
//! RO:INTERACTS — dto::errors, routes, ledger::client, seq, idem, policy, auth.
//! RO:INVARIANTS — malformed input maps to BAD_REQUEST; nonce/idempotency/economic conflicts map to 409; no panics.
//! RO:METRICS — map `WalletError::code()` into wallet_rejects_total{reason}.
//! RO:CONFIG — none.
//! RO:SECURITY — messages are intentionally small and must not include secrets or Authorization headers.
//! RO:TEST — unit: status_and_retryability_are_stable.

use ron_ledger::RejectReason;
use thiserror::Error;

/// Result alias for wallet internals.
pub type WalletResult<T> = Result<T, WalletError>;

/// Stable wallet API error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WalletErrorCode {
    /// DTO parse/validation failure.
    BadRequest,
    /// Missing or invalid bearer token.
    Unauthorized,
    /// Capability or policy denied the operation.
    Forbidden,
    /// Amount/body/ratio/ceiling exceeded.
    LimitsExceeded,
    /// Non-negativity guard rejected debit.
    InsufficientFunds,
    /// Per-account nonce mismatch.
    NonceConflict,
    /// Same idempotency key with different canonical request.
    IdempotencyConflict,
    /// Queue/backpressure/rate limit.
    Busy,
    /// Service degraded or readiness false.
    RetryLater,
    /// Ledger/auth/policy dependency unavailable.
    UpstreamUnavailable,
    /// Unknown transaction or receipt.
    NotFound,
}

impl WalletErrorCode {
    /// Stable string label for metrics and JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::LimitsExceeded => "LIMITS_EXCEEDED",
            Self::InsufficientFunds => "INSUFFICIENT_FUNDS",
            Self::NonceConflict => "NONCE_CONFLICT",
            Self::IdempotencyConflict => "IDEMPOTENCY_CONFLICT",
            Self::Busy => "BUSY",
            Self::RetryLater => "RETRY_LATER",
            Self::UpstreamUnavailable => "UPSTREAM_UNAVAILABLE",
            Self::NotFound => "NOT_FOUND",
        }
    }

    /// HTTP status code as an integer to keep the core independent of axum/http.
    pub const fn http_status(self) -> u16 {
        match self {
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden | Self::LimitsExceeded => 403,
            Self::InsufficientFunds | Self::NonceConflict | Self::IdempotencyConflict => 409,
            Self::NotFound => 404,
            Self::Busy => 429,
            Self::RetryLater | Self::UpstreamUnavailable => 503,
        }
    }

    /// True when clients can retry without changing request identity.
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::Busy | Self::RetryLater | Self::UpstreamUnavailable
        )
    }
}

/// Wallet error with stable code and small message.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{code:?}: {message}")]
pub struct WalletError {
    /// Stable code.
    pub code: WalletErrorCode,
    /// Human-readable redacted message.
    pub message: String,
}

impl WalletError {
    /// Construct a new error.
    pub fn new(code: WalletErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Bad request constructor.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(WalletErrorCode::BadRequest, message)
    }

    /// Limits exceeded constructor.
    pub fn limits_exceeded(message: impl Into<String>) -> Self {
        Self::new(WalletErrorCode::LimitsExceeded, message)
    }

    /// Forbidden constructor.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(WalletErrorCode::Forbidden, message)
    }

    /// Nonce conflict constructor.
    pub fn nonce_conflict(message: impl Into<String>) -> Self {
        Self::new(WalletErrorCode::NonceConflict, message)
    }

    /// Idempotency conflict constructor.
    pub fn idempotency_conflict(message: impl Into<String>) -> Self {
        Self::new(WalletErrorCode::IdempotencyConflict, message)
    }

    /// Upstream unavailable constructor.
    pub fn upstream(message: impl Into<String>) -> Self {
        Self::new(WalletErrorCode::UpstreamUnavailable, message)
    }

    /// Numeric HTTP status for route adapters.
    pub const fn http_status(&self) -> u16 {
        self.code.http_status()
    }

    /// Retryability flag for route adapters.
    pub const fn retryable(&self) -> bool {
        self.code.retryable()
    }
}

impl From<ron_ledger::LedgerError> for WalletError {
    fn from(value: ron_ledger::LedgerError) -> Self {
        match value.reject_reason() {
            Some(RejectReason::Invalid) => Self::bad_request(value.to_string()),
            Some(RejectReason::TooLarge) => Self::limits_exceeded(value.to_string()),
            Some(RejectReason::Timeout) => {
                Self::new(WalletErrorCode::RetryLater, value.to_string())
            }
            Some(RejectReason::Conflict) => {
                let msg = value.to_string();
                if msg.contains("insufficient balance") {
                    Self::new(WalletErrorCode::InsufficientFunds, msg)
                } else {
                    Self::new(WalletErrorCode::NonceConflict, msg)
                }
            }
            None => Self::upstream(value.to_string()),
        }
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(value: serde_json::Error) -> Self {
        Self::bad_request(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_retryability_are_stable() {
        assert_eq!(WalletErrorCode::BadRequest.http_status(), 400);
        assert_eq!(WalletErrorCode::NonceConflict.http_status(), 409);
        assert_eq!(WalletErrorCode::Busy.http_status(), 429);
        assert!(WalletErrorCode::RetryLater.retryable());
        assert!(!WalletErrorCode::Forbidden.retryable());
    }
}
