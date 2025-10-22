//! Smoke test: Metrics::serve mounts routes and binds successfully.

use std::net::SocketAddr;

use ron_kernel::{HealthState, Metrics};
use ron_kernel::metrics::readiness::Readiness;

#[tokio::test]
async fn metrics_server_binds_and_runs() {
    let metrics = Metrics::new(false);
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());

    // Toggle minimal readiness so /readyz can return 200 once needed
    ready.set_config_loaded(true);
    health.set("kernel", true);

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (handle, bound) = metrics.clone().serve(addr, health.clone(), ready.clone()).await.unwrap();

    // bound should be a real ephemeral port
    assert_ne!(bound.port(), 0);

    // shut it down
    handle.abort();
}
