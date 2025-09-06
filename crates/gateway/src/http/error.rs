//! Canonical ErrorEnvelope for HTTP + (optionally) OAP surfaces.
//! Maps to 400/404/413/429/503 and sets Retry-After when provided.

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Debug, Clone)]
pub struct ErrorEnvelope {
    pub code: &'static str,     // e.g., "bad_request", "not_found", "payload_too_large", "quota_exhausted", "unavailable"
    pub message: String,        // short human message
    pub retryable: bool,        // whether client may retry
    pub corr_id: String,        // correlation id for tracing/logs
}

fn corr_id() -> String {
    // no extra deps: nanos since epoch as hex
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos:x}")
}

fn with_retry_after(mut headers: HeaderMap, retry_after_secs: Option<u64>) -> HeaderMap {
    if let Some(secs) = retry_after_secs {
        if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
            headers.insert("Retry-After", v);
        }
    }
    headers
}

fn respond(status: StatusCode, env: ErrorEnvelope, headers: HeaderMap) -> Response {
    (status, headers, Json(env)).into_response()
}

/// 400 Bad Request
pub fn bad_request(message: impl Into<String>) -> Response {
    respond(
        StatusCode::BAD_REQUEST,
        ErrorEnvelope {
            code: "bad_request",
            message: message.into(),
            retryable: false,
            corr_id: corr_id(),
        },
        HeaderMap::new(),
    )
}

/// 404 Not Found
pub fn not_found(message: impl Into<String>) -> Response {
    respond(
        StatusCode::NOT_FOUND,
        ErrorEnvelope {
            code: "not_found",
            message: message.into(),
            retryable: false,
            corr_id: corr_id(),
        },
        HeaderMap::new(),
    )
}

/// 413 Payload Too Large
pub fn payload_too_large(message: impl Into<String>) -> Response {
    respond(
        StatusCode::PAYLOAD_TOO_LARGE,
        ErrorEnvelope {
            code: "payload_too_large",
            message: message.into(),
            retryable: false,
            corr_id: corr_id(),
        },
        HeaderMap::new(),
    )
}

/// 429 Too Many Requests (quota exhausted) — optional Retry-After
pub fn too_many_requests(message: impl Into<String>, retry_after_secs: Option<u64>) -> Response {
    respond(
        StatusCode::TOO_MANY_REQUESTS,
        ErrorEnvelope {
            code: "quota_exhausted",
            message: message.into(),
            retryable: true,
            corr_id: corr_id(),
        },
        with_retry_after(HeaderMap::new(), retry_after_secs),
    )
}

/// 503 Service Unavailable — optional Retry-After
pub fn unavailable(message: impl Into<String>, retry_after_secs: Option<u64>) -> Response {
    respond(
        StatusCode::SERVICE_UNAVAILABLE,
        ErrorEnvelope {
            code: "unavailable",
            message: message.into(),
            retryable: true,
            corr_id: corr_id(),
        },
        with_retry_after(HeaderMap::new(), retry_after_secs),
    )
}
