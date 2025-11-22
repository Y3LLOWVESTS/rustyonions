// crates/omnigate/src/errors/http_map.rs
//! RO:WHAT   Map admission/policy errors to stable JSON problem docs + helpers.
//! RO:WHY    Clients/SREs need consistent, parseable error envelopes.
//! RO:INTERACTS middleware::{quotas,fair_queue,body_caps,decompress_guard,policy}, admin plane.
//! RO:INVARS  Always include x-request-id upstream; no secret leakage; status matches code.

use axum::{
    http::{header::RETRY_AFTER, HeaderMap, HeaderValue, StatusCode},
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
    /// Optional free-form reason (e.g., policy reason like "put blocked").
    /// For guard helpers (body_caps, decompress_guard, etc.) this is a
    /// lowercase/snake-case code derived from `Reason`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'a str>,
}

/// Convert a millisecond budget into a Retry-After header value in SECONDS (ceil).
#[inline]
fn retry_after_header_secs(ms: u64) -> String {
    ms.div_ceil(1000).to_string()
}

/// Tiny helper used by tests and preflight guards: returns a canonical Problem JSON with the
/// HTTP status derived from `Reason`.
pub fn to_response(reason: Reason, message: &str) -> Response {
    let body = Problem {
        code: reason.code_str(),
        message,
        retryable: reason.retryable(),
        retry_after_ms: None,
        // IMPORTANT: tests like middleware_contract assert on this field
        // being a non-null snake_case string (e.g. "payload_too_large").
        reason: Some(reason.reason_str()),
    };
    (reason.status(), Json(body)).into_response()
}

/// Admission / policy / overload error space rendered as Problem JSON.
pub enum GateError<'a> {
    // Admission
    RateLimitedGlobal {
        retry_after_ms: u64,
    },
    RateLimitedIp {
        retry_after_ms: u64,
    },
    PayloadTooLarge {
        limit: u64,
    },
    UnsupportedEncoding {
        encoding: &'a str,
    },
    StackedEncodings,

    // Policy
    /// 403 default, 451 for legal blocks (status provided by caller)
    PolicyDeny {
        reason: &'a str,
        status: StatusCode,
    },
    /// 503 when evaluator fails/errors
    PolicyError,

    // Overload
    /// 503 when readiness gate is down
    Degraded,
}

impl<'a> IntoResponse for GateError<'a> {
    fn into_response(self) -> Response {
        match self {
            GateError::RateLimitedGlobal { retry_after_ms } => {
                let mut headers = HeaderMap::new();
                let hv = HeaderValue::from_str(&retry_after_header_secs(retry_after_ms))
                    .unwrap_or_else(|_| HeaderValue::from_static("1"));
                headers.insert(RETRY_AFTER, hv);
                let body = Problem {
                    code: Reason::TooManyRequests.code_str(),
                    message: "Global rate limit",
                    retryable: true,
                    retry_after_ms: Some(retry_after_ms),
                    reason: None,
                };
                (StatusCode::TOO_MANY_REQUESTS, headers, Json(body)).into_response()
            }
            GateError::RateLimitedIp { retry_after_ms } => {
                let mut headers = HeaderMap::new();
                let hv = HeaderValue::from_str(&retry_after_header_secs(retry_after_ms))
                    .unwrap_or_else(|_| HeaderValue::from_static("1"));
                headers.insert(RETRY_AFTER, hv);
                let body = Problem {
                    code: Reason::TooManyRequests.code_str(),
                    message: "IP quota exceeded",
                    retryable: true,
                    retry_after_ms: Some(retry_after_ms),
                    reason: None,
                };
                (StatusCode::TOO_MANY_REQUESTS, headers, Json(body)).into_response()
            }
            GateError::PayloadTooLarge { .. } => {
                let body = Problem {
                    code: Reason::PayloadTooLarge.code_str(),
                    message: "Body exceeds limit",
                    retryable: false,
                    retry_after_ms: None,
                    reason: None,
                };
                (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response()
            }
            GateError::UnsupportedEncoding { .. } | GateError::StackedEncodings => {
                let body = Problem {
                    code: Reason::UnsupportedMediaType.code_str(),
                    message: "Encoding not allowed",
                    retryable: false,
                    retry_after_ms: None,
                    reason: None,
                };
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, Json(body)).into_response()
            }
            GateError::PolicyDeny { reason, status } => {
                let code = if status == StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS {
                    "LEGAL_RESTRICTION"
                } else {
                    "POLICY_DENY"
                };
                let body = Problem {
                    code,
                    message: "Access denied",
                    retryable: false,
                    retry_after_ms: None,
                    // For policy we preserve the free-form reason provided by caller.
                    reason: Some(reason),
                };
                (status, Json(body)).into_response()
            }
            GateError::PolicyError => {
                // Small backoff hint (250ms) + header
                let retry_after_ms = 250u64;
                let mut headers = HeaderMap::new();
                let hv = HeaderValue::from_str(&retry_after_header_secs(retry_after_ms))
                    .unwrap_or_else(|_| HeaderValue::from_static("1"));
                headers.insert(RETRY_AFTER, hv);
                let body = Problem {
                    code: "POLICY_ERROR",
                    message: "Policy evaluation failed",
                    retryable: true,
                    retry_after_ms: Some(retry_after_ms),
                    reason: None,
                };
                (StatusCode::SERVICE_UNAVAILABLE, headers, Json(body)).into_response()
            }
            GateError::Degraded => {
                // Backoff hint (250ms) + header
                let retry_after_ms = 250u64;
                let mut headers = HeaderMap::new();
                let hv = HeaderValue::from_str(&retry_after_header_secs(retry_after_ms))
                    .unwrap_or_else(|_| HeaderValue::from_static("1"));
                headers.insert(RETRY_AFTER, hv);
                let body = Problem {
                    code: "SERVICE_DEGRADED",
                    message: "Overload protection",
                    retryable: true,
                    retry_after_ms: Some(retry_after_ms),
                    reason: None,
                };
                (StatusCode::SERVICE_UNAVAILABLE, headers, Json(body)).into_response()
            }
        }
    }
}

/// Generic downstream error mapper used by v1 passthrough routes (no crate::downstream dependency).
pub fn map_ds_error<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, e.to_string())
}
