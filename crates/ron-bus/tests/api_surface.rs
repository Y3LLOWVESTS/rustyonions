// RO:WHAT — API surface smoke tests for ron-bus.
// RO:WHY  — Lock in the basic constructors and methods to catch accidental drift.
// RO:INTERACTS — Bus, BusConfig, Event.
// RO:INVARIANTS — Monomorphic Bus; capacity fixed at construction; one receiver per task.

use ron_bus::{Bus, BusConfig, Event};
use tokio::sync::broadcast::error::RecvError;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_surface_basic() {
    // Default config path
    let bus = Bus::new(BusConfig::new()).expect("bus");
    assert!(bus.capacity() >= 2);

    // Builder path
    let bus = Bus::new(BusConfig::new().with_capacity(512)).expect("bus");
    assert_eq!(bus.capacity(), 512);

    // Sender / subscribe / send / recv
    let tx = bus.sender();
    let mut rx = bus.subscribe();

    tx.send(Event::ConfigUpdated { version: 1 }).unwrap();
    match rx.recv().await {
        Ok(Event::ConfigUpdated { version }) => assert_eq!(version, 1),
        other => panic!("unexpected recv: {:?}", other),
    }

    // Shutdown should be observable by the same receiver.
    tx.send(Event::Shutdown).unwrap();
    match rx.recv().await {
        Ok(Event::Shutdown) => {}
        Err(RecvError::Closed) => panic!("channel unexpectedly closed"),
        other => panic!("unexpected recv: {:?}", other),
    }
}
