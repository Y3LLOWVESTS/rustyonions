// crates/gateway/src/routes/errors.rs
//! Typed JSON error envelope and mappers for common HTTP errors.

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rand::Rng;
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    pub corr_id: String,
}

/// Internal helper to append/propagate a correlation id.
fn set_corr_id(mut headers: HeaderMap, corr_id: &str) -> HeaderMap {
    let key: HeaderName = HeaderName::from_static("x-corr-id");
    if let Ok(val) = HeaderValue::from_str(corr_id) {
        headers.insert(key, val);
    }
    headers
}

/// Accepts `u32` or `Option<u64>` for Retry-After.
pub(super) enum RetryAfter {
    Seconds(u32),
    None,
}
impl From<u32> for RetryAfter {
    fn from(v: u32) -> Self {
        RetryAfter::Seconds(v)
    }
}
impl From<Option<u64>> for RetryAfter {
    fn from(opt: Option<u64>) -> Self {
        match opt {
            Some(n) => RetryAfter::Seconds(n.min(u64::from(u32::MAX)) as u32),
            None => RetryAfter::None,
        }
    }
}
impl RetryAfter {
    fn write(self, headers: &mut HeaderMap) {
        if let RetryAfter::Seconds(secs) = self {
            let hv = HeaderValue::from_str(&secs.to_string())
                .unwrap_or(HeaderValue::from_static("60"));
            let _ = headers.insert(axum::http::header::RETRY_AFTER, hv);
        }
    }
}

fn build_response(
    status: StatusCode,
    code: &'static str,
    message: String,
    retryable: bool,
    retry_after: RetryAfter,
) -> Response {
    let corr_id = format!("{:016x}", rand::rng().random::<u64>());
    let body = ErrorBody {
        code,
        message,
        retryable,
        corr_id: corr_id.clone(),
    };

    let mut headers = HeaderMap::new();
    headers = set_corr_id(headers, &corr_id);
    retry_after.write(&mut headers);

    (status, headers, Json(body)).into_response()
}

/// 400 Bad Request
#[allow(dead_code)]
pub(super) fn bad_request(msg: impl Into<String>) -> Response {
    build_response(StatusCode::BAD_REQUEST, "bad_request", msg.into(), false, RetryAfter::None)
}

/// 404 Not Found
pub(super) fn not_found(msg: impl Into<String>) -> Response {
    build_response(StatusCode::NOT_FOUND, "not_found", msg.into(), false, RetryAfter::None)
}

/// 413 Payload Too Large
#[allow(dead_code)]
pub(super) fn payload_too_large(msg: impl Into<String>) -> Response {
    build_response(
        StatusCode::PAYLOAD_TOO_LARGE,
        "payload_too_large",
        msg.into(),
        false,
        RetryAfter::None,
    )
}

/// 429 Too Many Requests — accepts `u32` or `Option<u64>`
pub(super) fn too_many_requests(
    msg: impl Into<String>,
    retry_after_seconds: impl Into<RetryAfter>,
) -> Response {
    build_response(
        StatusCode::TOO_MANY_REQUESTS,
        "too_many_requests",
        msg.into(),
        true,
        retry_after_seconds.into(),
    )
}

/// 503 Service Unavailable — accepts `u32` or `Option<u64>`
pub(super) fn service_unavailable(
    msg: impl Into<String>,
    retry_after_seconds: impl Into<RetryAfter>,
) -> Response {
    build_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "service_unavailable",
        msg.into(),
        true,
        retry_after_seconds.into(),
    )
}

/// Back-compat alias for older call sites.
pub(super) fn unavailable(
    msg: impl Into<String>,
    retry_after_seconds: impl Into<RetryAfter>,
) -> Response {
    service_unavailable(msg, retry_after_seconds)
}

/// Fallback you can mount on the Router to ensure 404s are consistent.
#[allow(dead_code)]
pub async fn fallback_404() -> impl IntoResponse {
    not_found("route not found")
}

/// Map arbitrary error into a 503 envelope (e.g., for `.handle_error(...)`).
#[allow(dead_code)]
pub(super) fn map_into_503(err: impl std::fmt::Display) -> Response {
    service_unavailable(format!("temporary failure: {err}"), 30u32)
}
