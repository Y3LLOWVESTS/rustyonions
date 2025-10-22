//! /readyz returns 503 until both gates (config_loaded & services_ok) are true.

use axum::http::StatusCode;
use ron_kernel::Metrics;
use ron_kernel::metrics::health::HealthState;
use ron_kernel::metrics::readiness::{Readiness, readyz_handler};

#[tokio::test]
async fn readiness_degrades_then_ok() {
    let _metrics = Metrics::new(false); // ensures registry initialized, but not required here
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());

    // Initially: both gates false -> 503
    let resp = readyz_handler(ready.clone()).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Flip just one gate -> still 503
    ready.set_config_loaded(true);
    let resp = readyz_handler(ready.clone()).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Flip services_ok via HealthState -> 200
    health.set("kernel", true);
    let resp = readyz_handler(ready.clone()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
