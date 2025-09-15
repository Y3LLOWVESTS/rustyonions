use std::time::Duration;
use axum::{extract::DefaultBodyLimit, Router};
use tower::{Layer, ServiceBuilder};
use tower::limit::{ConcurrencyLimitLayer, RateLimitLayer};
use tower::timeout::TimeoutLayer;

/// Standard hardening stack:
/// - 5s handler timeout
/// - 512 in-flight requests
/// - 500 rps rate limit (tune per service)
/// - Request body cap (default ~1MiB, configurable)
pub fn layer(max_body: usize) -> impl Layer<Router> + Clone {
    ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .layer(ConcurrencyLimitLayer::new(512))
        .layer(RateLimitLayer::new(500, Duration::from_secs(1)))
        .layer(DefaultBodyLimit::max(max_body))
        .into_inner()
}
