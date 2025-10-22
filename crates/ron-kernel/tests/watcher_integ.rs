//! Integration glue: exercise ConfigUpdated emission + amnesia gauge flip.

use std::fs;
use std::time::Duration;

use ron_kernel::{Bus, KernelEvent, Metrics};

#[tokio::test]
async fn amnesia_flip_emits_single_update() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("ron-kernel.toml");
    fs::write(
        &cfg_path,
        "amnesia = false\nhttp_port = 0\nrequest_timeout_ms = 1000\n",
    )
    .unwrap();

    let metrics = Metrics::new(false);
    let bus: Bus<KernelEvent> = Bus::new().with_metrics(metrics.clone());
    let mut rx = bus.subscribe();

    // Simulate watcher apply; replace with real watcher if present.
    tokio::spawn({
        let metrics = metrics.clone();
        let bus = bus.clone();
        async move {
            metrics.set_amnesia(true);
            bus.publish(KernelEvent::ConfigUpdated { version: 1 });
        }
    });

    let mut updates = 0u32;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);

    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline - now;

        // Bound this await so the loop can re-check the deadline.
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(ev)) => {
                if let KernelEvent::ConfigUpdated { .. } = ev {
                    updates += 1;
                }
            }
            Ok(Err(_lagged)) => {
                // Ignore lag for this test: we're only counting ConfigUpdated.
            }
            Err(_elapsed) => {
                // No message before deadline — exit loop.
                break;
            }
        }
    }

    assert_eq!(updates, 1, "expected exactly one ConfigUpdated");
    metrics.set_amnesia(false);
}
