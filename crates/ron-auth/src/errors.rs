//! RO:WHAT  Public error/deny taxonomy (stable).
//! RO:WHY   Callers map to metrics and user-facing problem docs.
//! RO:INVARIANTS Codes/reasons are semver-stable.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("malformed token: {0}")]
    Malformed(&'static str),
    #[error("bounds exceeded")]
    Bounds,
    #[error("unknown kid")]
    UnknownKid,
    #[error("mac mismatch")]
    MacMismatch,
    #[error("expired")]
    Expired,
    #[error("not yet valid")]
    NotYetValid,
    #[error("policy deny")]
    PolicyDeny, // Decision::Deny will contain reasons; this is for strict-mode callers.
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "reason", content = "detail")]
pub enum DenyReason {
    // Time
    Expired,
    NotYetValid,
    // Shape
    BadAudience,
    MethodNotAllowed,
    PathNotAllowed,
    IpNotAllowed,
    TenantMismatch,
    BytesExceed,
    RateExceeded, // placeholder for future rate caveat eval
    Custom(String),
}
