#![forbid(unsafe_code)]
use std::time::Duration;
use ron_kernel::bus::{sub, Bus};
use ron_kernel::KernelEvent;

#[tokio::test]
async fn bus_topic_filtering() {
    let bus = Bus::new(8);
    let mut rx = bus.subscribe();

    // noise
    bus.publish_lossy(KernelEvent::Health { service: "svc-a".into(), ok: true });

    // target
    bus.publish_lossy(KernelEvent::ConfigUpdated { version: 42 });

    // more noise
    bus.publish_lossy(KernelEvent::Health { service: "svc-b".into(), ok: false });

    let got = sub::recv_matching(&bus, &mut rx, Duration::from_millis(250), |ev| {
        matches!(ev, KernelEvent::ConfigUpdated { version } if *version == 42)
    }).await;

    assert!(matches!(got, Some(KernelEvent::ConfigUpdated { version: 42 })));
}
