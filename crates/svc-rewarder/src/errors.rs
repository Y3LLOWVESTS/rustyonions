//! RO:WHAT — Central error taxonomy for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Stable errors keep clients, tests, and runbooks deterministic.
//! RO:INTERACTS — http::error maps these variants into JSON envelopes and HTTP statuses.
//! RO:INVARIANTS — no secrets in messages; quarantine is explicit; rejects are classified.
//! RO:METRICS — handlers map variants to rejected_total{reason}.
//! RO:CONFIG — config validation returns Config errors.
//! RO:SECURITY — auth failures are distinct from internal errors.
//! RO:TEST — http error mapping and config/invariant tests.

use thiserror::Error;

/// Crate-wide result type.
pub type Result<T> = std::result::Result<T, RewarderError>;

/// Stable service error taxonomy.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RewarderError {
    /// Request payload, path, or DTO was invalid.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// The caller is missing usable authentication.
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),
    /// The caller has auth but not the needed rewarder capability.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// Same epoch was already sealed with a different commitment triple.
    #[error("conflict: {0}")]
    Conflict(String),
    /// Economic invariant breach; no ledger effect may be emitted.
    #[error("quarantined: {0}")]
    Quarantined(String),
    /// Requested object is absent.
    #[error("not found: {0}")]
    NotFound(String),
    /// Backpressure or capacity limit rejected work before heavy compute.
    #[error("busy: {0}")]
    Busy(String),
    /// A bounded deadline expired.
    #[error("timeout: {0}")]
    Timeout(String),
    /// Downstream dependency is unavailable or degraded.
    #[error("dependency unavailable: {0}")]
    DependencyUnavailable(String),
    /// Configuration is invalid or could not be loaded.
    #[error("config: {0}")]
    Config(String),
    /// Internal failure without exposing implementation details to callers.
    #[error("internal: {0}")]
    Internal(String),
}

impl RewarderError {
    /// Low-cardinality reason label for metrics and error envelopes.
    #[must_use]
    pub fn reason(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::Unauthenticated(_) => "unauthenticated",
            Self::Unauthorized(_) => "unauthorized",
            Self::Conflict(_) => "conflict",
            Self::Quarantined(_) => "invariant",
            Self::NotFound(_) => "not_found",
            Self::Busy(_) => "busy",
            Self::Timeout(_) => "timeout",
            Self::DependencyUnavailable(_) => "dependency",
            Self::Config(_) => "config",
            Self::Internal(_) => "internal",
        }
    }

    /// Stable upper-case API code.
    #[must_use]
    pub fn api_code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Unauthenticated(_) => "UNAUTHENTICATED",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Conflict(_) => "CONFLICT",
            Self::Quarantined(_) => "QUARANTINED",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Busy(_) => "TOO_MANY_REQUESTS",
            Self::Timeout(_) => "TIMEOUT",
            Self::DependencyUnavailable(_) => "DEPENDENCY_UNAVAILABLE",
            Self::Config(_) => "CONFIG_INVALID",
            Self::Internal(_) => "INTERNAL",
        }
    }
}

impl From<toml::de::Error> for RewarderError {
    fn from(value: toml::de::Error) -> Self {
        Self::Config(value.to_string())
    }
}

impl From<std::io::Error> for RewarderError {
    fn from(value: std::io::Error) -> Self {
        Self::Config(value.to_string())
    }
}

impl From<serde_json::Error> for RewarderError {
    fn from(value: serde_json::Error) -> Self {
        Self::BadRequest(value.to_string())
    }
}
