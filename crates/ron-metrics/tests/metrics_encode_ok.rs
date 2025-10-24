use ron_metrics::{BaseLabels, HealthState, Metrics};
use ron_metrics::build_info::build_version;
use prometheus::{Encoder, TextEncoder};

#[test]
fn metrics_encode_ok() {
    let base = BaseLabels {
        service: "test-svc".into(),
        instance: "test-1".into(),
        build_version: build_version(),
        amnesia: "off".into(),
    };
    let health = HealthState::new();
    health.set("config_loaded".into(), true);

    let metrics = Metrics::new(base, health).expect("metrics new");
    metrics.inc_service_restart("worker-A");
    metrics.add_bus_lag("kernel", 2);
    metrics.observe_request(0.002);

    let mf = metrics.registry().gather();
    assert!(!mf.is_empty(), "registry should have some families");

    let mut buf = Vec::new();
    let enc = TextEncoder::new();
    enc.encode(&mf, &mut buf).expect("encode");
    assert!(!buf.is_empty(), "prometheus text should be non-empty");
}
