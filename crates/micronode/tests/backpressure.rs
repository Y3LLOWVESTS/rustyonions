//! RO:WHAT — Integration-style tests for Micronode backpressure behavior.
//! RO:WHY  — Assert that the concurrency layer sheds overload with 429 and that
//!           the concurrency registry builds distinct, bounded pools.
//!
//! These tests operate entirely in-process (no real TCP sockets), driving the
//! Axum `Router` directly.

use std::sync::Arc;

use axum::{body::Body, routing::get, Router};
use http::{Request, StatusCode};
use micronode::{
    concurrency::{ConcurrencyConfig, ConcurrencyRegistry},
    layers::concurrency::ConcurrencyLayer,
};
use tokio::sync::Semaphore;
use tower::ServiceExt as _; // for `Router::oneshot`

#[tokio::test]
async fn concurrency_layer_sheds_with_429_when_pool_is_exhausted() {
    // Limit=1 and we hold the only permit up front so the layer sees a
    // saturated pool and must respond with 429 immediately.
    let sema = Arc::new(Semaphore::new(1));
    let _held = sema.clone().acquire_owned().await.expect("failed to acquire initial permit");

    let app =
        Router::new().route("/hot", get(|| async { "ok" })).layer(ConcurrencyLayer::new(sema));

    let req = Request::builder().uri("/hot").body(Body::empty()).expect("build request");

    let resp = app.oneshot(req).await.expect("router call failed");

    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "expected 429 from ConcurrencyLayer when pool is exhausted"
    );
}

#[tokio::test]
async fn concurrency_registry_builds_distinct_pools_per_class() {
    let cfg = ConcurrencyConfig::default();
    let registry = ConcurrencyRegistry::from_config(&cfg);

    let admin = registry.get("http_admin");
    let kv = registry.get("http_kv");

    // Pointers must differ; these are distinct semaphores for distinct budgets.
    let admin_ptr: *const Semaphore = &*admin;
    let kv_ptr: *const Semaphore = &*kv;

    assert_ne!(admin_ptr, kv_ptr, "admin and kv should have distinct concurrency pools");

    // Both pools should honour their configured caps.
    assert_eq!(
        cfg.http_admin.max_inflight,
        admin.available_permits(),
        "admin pool should be initialized with the configured max_inflight"
    );
    assert_eq!(
        cfg.http_kv.max_inflight,
        kv.available_permits(),
        "kv pool should be initialized with the configured max_inflight"
    );
}
