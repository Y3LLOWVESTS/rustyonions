#![forbid(unsafe_code)]
use std::time::Duration;
use ron_kernel::bus::{sub, Bus};
use ron_kernel::KernelEvent;

#[tokio::test]
async fn bus_reports_lag() {
    // Tiny capacity to force lag on slow consumer.
    let bus = Bus::new(2);
    let mut rx = bus.subscribe();

    // Publish a burst larger than capacity.
    for i in 0..16u32 {
        bus.publish_lossy(KernelEvent::ConfigUpdated { version: i as u64 });
    }

    // Drain some messages slowly so we trigger "lagged".
    let mut seen = 0;
    for _ in 0..8 {
        if sub::recv_with_timeout(&bus, &mut rx, Duration::from_millis(50)).await.is_some() {
            seen += 1;
        }
    }

    // We should have recorded some drop.
    assert!(bus.dropped_total() > 0, "expected dropped_total > 0, seen={seen}");
}
