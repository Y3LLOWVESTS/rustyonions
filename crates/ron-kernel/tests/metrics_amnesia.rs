//! Ensures amnesia_mode gauge flips 0 <-> 1 and is exposed by the exporter.

use prometheus::Encoder; // brings TextEncoder::encode into scope
use ron_kernel::Metrics;
use ron_kernel::metrics::health::HealthState;
use ron_kernel::metrics::readiness::Readiness;

#[tokio::test]
async fn amnesia_mode_gauge_flips_and_exports() {
    let metrics = Metrics::new(false);

    // Sanity: initial seed should be 0
    {
        // NOTE: 'registry' is a field on Metrics; metrics is Arc<Metrics>
        let families = (*metrics).registry.gather();
        let mut buf = Vec::new();
        prometheus::TextEncoder::new()
            .encode(&families, &mut buf)
            .unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("amnesia_mode 0"),
            "expected amnesia_mode 0 at start, got:\n{text}"
        );
    }

    metrics.set_amnesia(true);
    {
        let families = (*metrics).registry.gather();
        let mut buf = Vec::new();
        prometheus::TextEncoder::new()
            .encode(&families, &mut buf)
            .unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("amnesia_mode 1"),
            "expected amnesia_mode 1 after flip, got:\n{text}"
        );
    }

    // Boot the HTTP exporter quickly to catch regressions in wiring.
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());
    ready.set_config_loaded(true);
    health.set("kernel", true);

    let (_handle, bound) =
        metrics.clone().serve("127.0.0.1:0".parse().unwrap(), health, ready).await.unwrap();
    assert_ne!(bound.port(), 0);
}
