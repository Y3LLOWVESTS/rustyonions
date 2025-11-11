//! /metrics — Prometheus exposition.

use crate::metrics::EdgeMetrics;
use crate::state::AppState;
use axum::{extract::State, response::IntoResponse};
use http::{header, HeaderValue, StatusCode};

/// Render the Prometheus metrics exposition format.
///
/// Uses the global registry populated by the service.
pub async fn metrics(State(_state): State<AppState>) -> impl IntoResponse {
    let body = EdgeMetrics::gather();
    let mut res = (StatusCode::OK, body).into_response();
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; version=0.0.4"));
    res
}
