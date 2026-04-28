//! RO:WHAT — HTTP error envelope and status mapping for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: DX/GOV/SEC. Clients need deterministic structured failures.
//! RO:INTERACTS — errors::RewarderError, axum responses, metrics reject labels.
//! RO:INVARIANTS — no secrets in message/details; stable error.code and reason.
//! RO:METRICS — handlers increment rejected_total before returning errors.
//! RO:CONFIG — none.
//! RO:SECURITY — authorization failures are distinguished without leaking token data.
//! RO:TEST — integration tests assert status behavior.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::RewarderError;

/// Top-level error envelope.
#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    /// Error body.
    pub error: ErrorBody,
}

/// Stable error fields.
#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    /// Stable upper-case code.
    pub code: &'static str,
    /// Human-readable summary.
    pub message: String,
    /// Correlation id.
    pub corr_id: String,
    /// Machine details.
    pub details: ErrorDetails,
}

/// Error details object.
#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorDetails {
    /// Low-cardinality reason.
    pub reason: &'static str,
}

/// Axum response wrapper.
#[derive(Debug, Clone)]
pub struct HttpError {
    err: RewarderError,
    corr_id: String,
}

impl HttpError {
    /// Create error with corr id.
    #[must_use]
    pub fn new(err: RewarderError, corr_id: impl Into<String>) -> Self {
        Self {
            err,
            corr_id: corr_id.into(),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = status_for(&self.err);
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.err.api_code(),
                message: self.err.to_string(),
                corr_id: self.corr_id,
                details: ErrorDetails {
                    reason: self.err.reason(),
                },
            },
        };
        (status, Json(body)).into_response()
    }
}

/// HTTP status mapping.
#[must_use]
pub fn status_for(err: &RewarderError) -> StatusCode {
    match err {
        RewarderError::BadRequest(_) | RewarderError::Config(_) => StatusCode::BAD_REQUEST,
        RewarderError::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
        RewarderError::Unauthorized(_) => StatusCode::FORBIDDEN,
        RewarderError::Conflict(_) | RewarderError::Quarantined(_) => StatusCode::CONFLICT,
        RewarderError::NotFound(_) => StatusCode::NOT_FOUND,
        RewarderError::Busy(_) => StatusCode::TOO_MANY_REQUESTS,
        RewarderError::Timeout(_) | RewarderError::DependencyUnavailable(_) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        RewarderError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
