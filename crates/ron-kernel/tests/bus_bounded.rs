//! Bounded bus: lag accounting and publish semantics.

use ron_kernel::{Bus, KernelEvent, Metrics};

#[tokio::test]
async fn lagged_receiver_increments_lag_counter_and_publish_counts() {
    let metrics = Metrics::new(false);
    // small capacity to force lag quickly
    let bus: Bus<KernelEvent> = Bus::with_capacity(8).with_metrics(metrics.clone());
    let mut rx = bus.subscribe();

    // With one subscriber, publish returns 1
    let n = bus.publish(KernelEvent::ConfigUpdated { version: 1 });
    assert_eq!(n, 1);

    // Overflow receiver by sending many without reading
    for i in 0..2048usize {
        let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
    }

    // Receiver sees either a value or Lagged; account via helper
    let _ = Bus::handle_recv(rx.recv().await, Some(&metrics));

    // We should have observed some lag
    assert!(
        metrics.bus_receiver_lag_total.get() > 0,
        "expected bus_receiver_lag_total to increase"
    );
}
