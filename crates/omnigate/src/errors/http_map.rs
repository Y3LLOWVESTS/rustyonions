//! RO:WHAT   Map admission/policy errors to stable JSON problem docs + helpers.
//! RO:WHY    Clients/SREs need consistent, parseable error envelopes.
//! RO:INTERACTS middleware::{quotas,fair_queue,body_caps,decompress_guard,policy}, admin plane.
//! RO:INVARS  Always include x-request-id upstream; no secret leakage; status matches code.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use super::reasons::Reason;

/// Stable problem envelope for client-visible errors.
#[derive(Serialize)]
pub struct Problem<'a> {
    pub code: &'a str,
    pub message: &'a str,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'a str>,
}

/// Tiny helper used by tests and preflight guards: builds `{reason,message}` JSON with canonical status.
pub fn to_response(reason: Reason, message: &str) -> impl IntoResponse {
    let body = serde_json::json!({
        "reason": reason.key(),
        "message": message,
    });
    (reason.status(), Json(body))
}

/// Admission / policy / overload error space rendered as Problem JSON.
pub enum GateError<'a> {
    // Admission
    RateLimitedGlobal { retry_after_ms: u64 },
    RateLimitedIp { retry_after_ms: u64 },
    PayloadTooLarge { limit: u64 },
    UnsupportedEncoding { encoding: &'a str },
    StackedEncodings,
    // Policy
    PolicyDeny { reason: &'a str, status: StatusCode }, // 403 default, 451 for legal blocks
    PolicyError,                                        // 503 when evaluator fails/errors
    // Overload
    Degraded, // 503 when readiness gate is down
}

impl<'a> IntoResponse for GateError<'a> {
    fn into_response(self) -> Response {
        match self {
            GateError::RateLimitedGlobal { retry_after_ms } => {
                let body = Problem {
                    code: "RATE_LIMITED",
                    message: "Global rate limit",
                    retryable: true,
                    retry_after_ms: Some(retry_after_ms),
                    reason: Some(Reason::TooManyRequests.key()),
                };
                (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response()
            }
            GateError::RateLimitedIp { retry_after_ms } => {
                let body = Problem {
                    code: "RATE_LIMITED",
                    message: "IP quota exceeded",
                    retryable: true,
                    retry_after_ms: Some(retry_after_ms),
                    reason: Some(Reason::TooManyRequests.key()),
                };
                (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response()
            }
            GateError::PayloadTooLarge { .. } => {
                let body = Problem {
                    code: "PAYLOAD_TOO_LARGE",
                    message: "Body exceeds limit",
                    retryable: false,
                    retry_after_ms: None,
                    reason: Some(Reason::PayloadTooLarge.key()),
                };
                (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response()
            }
            GateError::UnsupportedEncoding { .. } | GateError::StackedEncodings => {
                let body = Problem {
                    code: "UNSUPPORTED_ENCODING",
                    message: "Encoding not allowed",
                    retryable: false,
                    retry_after_ms: None,
                    reason: Some(Reason::UnsupportedMediaType.key()),
                };
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, Json(body)).into_response()
            }
            GateError::PolicyDeny { reason, status } => {
                let body = Problem {
                    code: if status == StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS {
                        "LEGAL_RESTRICTION"
                    } else {
                        "POLICY_DENY"
                    },
                    message: "Access denied",
                    retryable: false,
                    retry_after_ms: None,
                    reason: Some(reason),
                };
                (status, Json(body)).into_response()
            }
            GateError::PolicyError => {
                let body = Problem {
                    code: "POLICY_ERROR",
                    message: "Policy evaluation failed",
                    retryable: true,
                    retry_after_ms: Some(250),
                    reason: Some(Reason::Unavailable.key()),
                };
                (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
            }
            GateError::Degraded => {
                let body = Problem {
                    code: "SERVICE_DEGRADED",
                    message: "Overload protection",
                    retryable: true,
                    retry_after_ms: Some(250),
                    reason: Some(Reason::Unavailable.key()),
                };
                (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
            }
        }
    }
}

/// Generic downstream error mapper used by v1 passthrough routes (no crate::downstream dependency).
pub fn map_ds_error<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, e.to_string())
}
