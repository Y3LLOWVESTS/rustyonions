//! Readiness and health endpoints: handler semantics (no actual HTTP client).
use std::net::SocketAddr;

use axum::http::StatusCode;
use ron_kernel::{HealthState, Metrics};
use ron_kernel::metrics::readiness::{readyz_handler, Readiness};

#[tokio::test]
async fn readiness_transitions_to_ok_when_config_and_services_ready() {
    let metrics = Metrics::new(false);
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());

    // Start server just to ensure the router builds; we won't fetch it here.
    let (_handle, _addr) = metrics
        .clone()
        .serve("127.0.0.1:0".parse::<SocketAddr>().unwrap(), health.clone(), ready.clone())
        .await
        .unwrap();

    // Initially not ready (config not loaded)
    let resp = readyz_handler(ready.clone()).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Make both gates true
    ready.set_config_loaded(true);
    health.set("svc", true);

    // Now the handler should return 200 OK
    let ok_resp = readyz_handler(ready.clone()).await;
    assert_eq!(ok_resp.status(), StatusCode::OK);
}
