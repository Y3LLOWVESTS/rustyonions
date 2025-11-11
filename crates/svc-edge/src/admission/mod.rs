//! Admission chain builder (timeout → inflight → RPS → body cap).
//!
//! RO:WHAT
//! - Small helper to apply a standard set of hardening layers to a `Router`.
//!
//! RO:WHY
//! - Keep the main binary uncluttered; centralize the composition here.

use std::time::Duration;

use axum::{
    error_handling::HandleErrorLayer,
    response::IntoResponse,
    Router,
};
use http::StatusCode;
use tower::{
    limit::{ConcurrencyLimitLayer, RateLimitLayer},
    timeout::{error::Elapsed, TimeoutLayer},
    util::BoxCloneService,
    BoxError, ServiceBuilder,
};
use tower_http::limit::RequestBodyLimitLayer;

use crate::AppState;

/// Apply the default admission stack to a router.
///
/// Defaults (aligned with docs; adjustable later via `Config`):
/// - timeout: 5s  → 408 on expiry
/// - inflight cap: 256 → 503 when saturated
/// - RPS: 1000 req/s → 429 when exceeded
/// - body cap: 1 MiB → 413 handled by the layer
pub fn apply_defaults(router: Router<AppState>) -> Router<AppState> {
    let layers = ServiceBuilder::new()
        // 1) Map middleware errors → HTTP responses (makes Error=Infallible for Axum).
        .layer(HandleErrorLayer::new(|err: BoxError| async move {
            // Timeout → 408
            if err.is::<Elapsed>() {
                return (StatusCode::REQUEST_TIMEOUT, "request timed out").into_response();
            }
            // Rate or concurrency limiting → 429, otherwise treat as overload → 503.
            let msg = err.to_string();
            if msg.contains("rate limit") || msg.contains("RateLimit") {
                (StatusCode::TOO_MANY_REQUESTS, "rate limit").into_response()
            } else {
                (StatusCode::SERVICE_UNAVAILABLE, "overloaded").into_response()
            }
        }))
        // 2) Admission layers (cheap guards first is fine here).
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .layer(ConcurrencyLimitLayer::new(256))
        .layer(RateLimitLayer::new(1000, Duration::from_secs(1)))
        .layer(RequestBodyLimitLayer::new(1_048_576)) // 1 MiB
        // 3) Ensure the composed service is Clone for Router::layer().
        .layer(BoxCloneService::layer());

    router.layer(layers)
}
