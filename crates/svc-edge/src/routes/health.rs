//! /healthz — liveness endpoint.

use axum::response::IntoResponse;
use http::StatusCode;

/// Liveness probe.
///
/// Returns `200 OK` unconditionally if the process is alive.
pub async fn healthz() -> impl IntoResponse {
    StatusCode::OK
}
