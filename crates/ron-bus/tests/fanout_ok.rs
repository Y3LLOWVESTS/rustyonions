// RO:WHAT — Happy-path fanout: N subscribers receive all events without lag
// RO:WHY  — Proves bounded bus works for steady load; publishers non-blocking
// RO:INTERACTS — Bus, BusConfig, Event
// RO:INVARIANTS — no deadlocks; all receivers get events when not lagging

use ron_bus::{Bus, BusConfig, Event};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fanout_ok() {
    // Use builder because BusConfig is #[non_exhaustive]
    let cfg = BusConfig::new().with_capacity(256);
    let bus = Bus::new(cfg).unwrap();

    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    // Publish a couple of events
    let tx = bus.sender();
    tx.send(Event::Health { service: "svc.a".into(), ok: true }).unwrap();
    tx.send(Event::Shutdown).unwrap();

    let a1 = rx1.recv().await.unwrap();
    let a2 = rx1.recv().await.unwrap();
    let b1 = rx2.recv().await.unwrap();
    let b2 = rx2.recv().await.unwrap();

    assert_eq!(a1, Event::Health { service: "svc.a".into(), ok: true });
    assert_eq!(a2, Event::Shutdown);
    assert_eq!(b1, Event::Health { service: "svc.a".into(), ok: true });
    assert_eq!(b2, Event::Shutdown);
}
