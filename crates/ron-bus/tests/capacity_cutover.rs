// RO:WHAT — Capacity cutover test (A -> drop -> B).
// RO:WHY  — Capacity is fixed; resizing is done by constructing a new Bus.
// RO:INTERACTS — Bus, BusConfig, Event.
// RO:INVARIANTS — No background tasks; old bus is dropped before new is created.

use ron_bus::{Bus, BusConfig, Event};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capacity_cutover_recreate_bus() {
    // Bus A
    let bus_a = Bus::new(BusConfig::new().with_capacity(64)).expect("bus A");
    assert_eq!(bus_a.capacity(), 64);

    // Prove basic send/recv works
    let tx_a = bus_a.sender();
    let mut rx_a = bus_a.subscribe();
    tx_a.send(Event::ConfigUpdated { version: 100 }).unwrap();
    let ev = rx_a.recv().await.unwrap();
    match ev {
        Event::ConfigUpdated { version } => assert_eq!(version, 100),
        _ => panic!("unexpected event on bus A: {:?}", ev),
    }

    // Drop A, construct B with different capacity
    drop(rx_a);
    drop(tx_a);
    drop(bus_a);

    // Bus B
    let bus_b = Bus::new(BusConfig::new().with_capacity(128)).expect("bus B");
    assert_eq!(bus_b.capacity(), 128);

    let tx_b = bus_b.sender();
    let mut rx_b = bus_b.subscribe();
    tx_b.send(Event::Shutdown).unwrap();
    let ev = rx_b.recv().await.unwrap();
    matches!(ev, Event::Shutdown);
}
