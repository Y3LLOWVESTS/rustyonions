//! Decode guard: reject stacked encodings and enforce decoded absolute cap.
//!
//! MVP behavior (no on-the-fly decompression yet):
//! - If `Content-Encoding` has multiple values (comma-separated) → 415 `stacked_encoding`
//! - If `Content-Encoding` is one of {gzip, deflate, br, zstd} → 415 `encoded_body_unsupported`
//! - If `Content-Length` > `SVC_GATEWAY_DECODE_ABS_CAP_BYTES` (or config default) → 413 `decoded_cap`
//!
//! Metrics: increments `gateway_rejections_total{reason=...}`
//!
//! Future: replace with a true streaming decode stage, enforcing `ratio_max`.

use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::consts::DEFAULT_DECODE_ABS_CAP_BYTES;
use crate::errors::Problem;
use crate::observability::rejects::counter as rejects;

fn parse_len(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

fn content_encoding(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_lowercase())
}

fn abs_cap_from_env() -> u64 {
    std::env::var("SVC_GATEWAY_DECODE_ABS_CAP_BYTES")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DECODE_ABS_CAP_BYTES as u64)
}

fn encoded_kind(enc: &str) -> Option<&'static str> {
    match enc {
        "gzip" => Some("gzip"),
        "deflate" => Some("deflate"),
        "br" => Some("br"),
        "zstd" => Some("zstd"),
        _ => None,
    }
}

pub async fn decode_guard_mw(req: Request<Body>, next: Next) -> Response {
    let headers = req.headers();
    let cap = abs_cap_from_env();

    // 1) Absolute cap on declared length
    if let Some(n) = parse_len(headers) {
        if n > cap {
            rejects().with_label_values(&["decode_cap"]).inc();
            let body = Problem {
                code: "decoded_cap",
                message: "decoded size exceeds cap",
                retryable: false,
                retry_after_ms: None,
                reason: None,
            };
            return (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response();
        }
    }

    // 2) Content-Encoding checks
    if let Some(enc) = content_encoding(headers) {
        if enc.contains(',') {
            rejects().with_label_values(&["stacked_encoding"]).inc();
            let body = Problem {
                code: "stacked_encoding",
                message: "stacked content-encoding not supported",
                retryable: false,
                retry_after_ms: None,
                reason: None,
            };
            return (StatusCode::UNSUPPORTED_MEDIA_TYPE, Json(body)).into_response();
        }
        if encoded_kind(&enc).is_some() {
            rejects().with_label_values(&["encoded_body"]).inc();
            let body = Problem {
                code: "encoded_body_unsupported",
                message: "encoded request bodies are not supported",
                retryable: false,
                retry_after_ms: None,
                reason: None,
            };
            return (StatusCode::UNSUPPORTED_MEDIA_TYPE, Json(body)).into_response();
        }
    }

    next.run(req).await
}
