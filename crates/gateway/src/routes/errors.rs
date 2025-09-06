// crates/gateway/src/routes/errors.rs
#![forbid(unsafe_code)]

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Debug, Clone)]
struct ErrorEnvelope {
    code: &'static str,  // "bad_request" | "not_found" | "payload_too_large" | "quota_exhausted" | "unavailable"
    message: String,
    retryable: bool,
    corr_id: String,
}

fn corr_id() -> String {
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

fn respond_json(status: StatusCode, env: ErrorEnvelope, headers: HeaderMap) -> Response {
    (status, headers, Json(env)).into_response()
}

pub fn not_found(message: impl Into<String>) -> Response {
    respond_json(
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

pub fn too_many_requests(message: impl Into<String>, retry_after_secs: Option<u64>) -> Response {
    respond_json(
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

pub fn unavailable(message: impl Into<String>, retry_after_secs: Option<u64>) -> Response {
    respond_json(
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
