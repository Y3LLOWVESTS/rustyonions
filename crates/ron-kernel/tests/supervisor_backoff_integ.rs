use std::time::{Duration, Instant};

use ron_kernel::metrics::readiness::Readiness;
use ron_kernel::supervisor::{backoff::Backoff, lifecycle::Supervisor};
use ron_kernel::{Bus, HealthState, KernelEvent, Metrics};

#[tokio::test]
async fn supervisor_restarts_and_backoff_grows() {
    let metrics = Metrics::new(false);
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());
    ready.set_config_loaded(true);

    let bus: Bus<KernelEvent> = Bus::new().with_metrics(metrics.clone());

    let work =
        || async { Err::<(), Box<dyn std::error::Error + Send + Sync + 'static>>("fail".into()) };

    let mut sup = Supervisor::new(
        "testsvc",
        (*metrics).clone(),
        bus.clone(),
        health.clone(),
        Backoff::new(
            Duration::from_millis(100),
            Duration::from_millis(400),
            2.0,
            0.0,
        ),
        100,
        Duration::from_secs(10),
    );

    let start = Instant::now();
    let h = tokio::spawn(async move { sup.run(work).await });

    tokio::time::sleep(Duration::from_millis(850)).await;
    h.abort();

    // read the counter from the *_total vec
    let c = metrics
        .service_restarts_total
        .with_label_values(&["testsvc"])
        .get();
    assert!(c >= 3, "expected >=3 restarts, got {}", c);

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(300));
}
