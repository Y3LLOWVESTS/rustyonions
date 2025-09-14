#![forbid(unsafe_code)]

use ron_kernel::bus::Bus;
use ron_kernel::KernelEvent;
use std::error::Error;

#[tokio::test]
async fn bus_basic_pubsub() -> Result<(), Box<dyn Error>> {
    let bus = Bus::new(8);
    let mut rx = bus.subscribe();

    bus.publish(KernelEvent::Health {
        service: "svc-a".into(),
        ok: true,
    })?;

    let ev = rx.recv().await?;
    match ev {
        KernelEvent::Health { service, ok } => {
            assert_eq!(service, "svc-a");
            assert!(ok);
        }
        other => panic!("unexpected event: {:?}", other),
    }

    Ok(())
}
