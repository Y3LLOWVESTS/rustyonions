//! Route-scoped timeout middleware.
//! RO:WHAT  Enforce a simple request timeout using Tokio's `timeout`.
//! RO:WHY   Prevents hung readiness checks from stalling callers.
//! RO:NOTE  Axum 0.7 middleware shape (`Next` has no generics).
//! RO:CONF  Duration via `SVC_GATEWAY_READY_TIMEOUT_MS` (default 200ms).

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Duration;
use tokio::time::timeout;

/// Timeout wrapper for `/readyz`.
/// - Reads `SVC_GATEWAY_READY_TIMEOUT_MS` (u64) from env (default 200ms).
/// - On timeout, returns `504 Gateway Timeout` with "timeout".
pub async fn ready_timeout_mw(req: Request<Body>, next: Next) -> Response {
    let ms = std::env::var("SVC_GATEWAY_READY_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(200);

    match timeout(Duration::from_millis(ms), next.run(req)).await {
        Ok(resp) => resp,
        Err(_) => (axum::http::StatusCode::GATEWAY_TIMEOUT, "timeout").into_response(),
    }
}
