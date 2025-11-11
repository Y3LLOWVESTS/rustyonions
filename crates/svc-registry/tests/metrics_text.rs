use std::sync::Arc;

use axum::{body, http::Request, Router};
use ron_kernel::HealthState;
use svc_registry::{
    build_info,
    observability::{
        endpoints::{admin_router, AdminState},
        metrics::RegistryMetrics,
    },
};
use tower::util::ServiceExt; // for .oneshot

#[tokio::test]
async fn metrics_text_includes_registry_counters() {
    // Build admin router with current state.
    let metrics = RegistryMetrics::default();
    let health = Arc::new(HealthState::default());
    let admin: Router = admin_router(AdminState {
        health,
        build: build_info::build_info(),
        metrics: metrics.clone(),
    });

    // GET /metrics
    let res = admin
        .clone()
        .oneshot(Request::get("/metrics").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(res.status().is_success());

    // axum::body::to_bytes requires a limit param (bytes cap).
    let bytes = body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let s = String::from_utf8(bytes.to_vec()).unwrap();

    // “Must-have” metric names:
    assert!(s.contains("registry_head_version"));
    assert!(s.contains("registry_sse_clients_connected_total"));
    assert!(s.contains("registry_sse_clients_disconnected_total"));
}
