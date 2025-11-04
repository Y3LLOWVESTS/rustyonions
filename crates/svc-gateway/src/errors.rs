//! Carry-over: `Problem{code,message,retryable,retry_after_ms?,reason?}`.

use axum::{
    http::{self, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

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

impl Problem<'_> {
    #[must_use]
    pub fn into_response_with(self, status: StatusCode) -> Response {
        (status, Json(self)).into_response()
    }
}

/// 429 Too Many Requests with Retry-After (seconds). Never panics.
pub fn rate_limited_retry_after(ms: u64) -> Response {
    let mut resp = Problem {
        code: "rate_limited",
        message: "Too many requests",
        retryable: true,
        retry_after_ms: Some(ms),
        reason: None,
    }
    .into_response_with(StatusCode::TOO_MANY_REQUESTS);

    // Best-effort Retry-After header; if conversion fails, omit it.
    if let Ok(v) = HeaderValue::from_str(&(ms / 1000).to_string()) {
        let headers = resp.headers_mut();
        headers.insert(http::header::RETRY_AFTER, v);
    }
    resp
}

/// 503 Busy with Retry-After (seconds). Never panics.
pub fn too_busy_retry_after(ms: u64) -> Response {
    let mut resp = Problem {
        code: "too_busy",
        message: "Server busy",
        retryable: true,
        retry_after_ms: Some(ms),
        reason: None,
    }
    .into_response_with(StatusCode::SERVICE_UNAVAILABLE);

    if let Ok(v) = HeaderValue::from_str(&(ms / 1000).to_string()) {
        let headers = resp.headers_mut();
        headers.insert(http::header::RETRY_AFTER, v);
    }
    resp
}
