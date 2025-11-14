//! /metrics — Prometheus exposition.

use crate::state::AppState;
use axum::{extract::State, response::IntoResponse};
use http::{header, HeaderValue, StatusCode};
use prometheus::{Encoder, TextEncoder};

/// Render the Prometheus metrics exposition format from the default registry.
pub async fn metrics(State(_state): State<AppState>) -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let encoder = TextEncoder::new();

    let mut buf = Vec::new();
    if let Err(_e) = encoder.encode(&metric_families, &mut buf) {
        // Conservative fallback: don't leak internals; keep content-type text/plain.
        let mut res = (StatusCode::INTERNAL_SERVER_ERROR, "metrics encode error\n").into_response();
        res.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
        return res;
    }

    let body = String::from_utf8(buf).unwrap_or_default();

    let mut res = (StatusCode::OK, body).into_response();
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; version=0.0.4"));
    res
}
