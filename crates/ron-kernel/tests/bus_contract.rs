use ron_kernel::{Bus, KernelEvent, Metrics};

#[tokio::test]
async fn publish_zero_subscribers_counts_and_returns_zero() {
    let metrics = Metrics::new(false);
    let bus: Bus<KernelEvent> = Bus::new().with_metrics(metrics.clone());

    let n = bus.publish(KernelEvent::Shutdown);
    assert_eq!(n, 0);

    let m = bus.publish(KernelEvent::Shutdown);
    assert_eq!(m, 0);
}

#[tokio::test]
async fn publish_with_subscriber_returns_one() {
    let metrics = Metrics::new(false);
    let bus: Bus<KernelEvent> = Bus::new().with_metrics(metrics.clone());
    let mut rx = bus.subscribe();

    let n = bus.publish(KernelEvent::Shutdown);
    assert_eq!(n, 1);

    let _ = Bus::handle_recv(rx.recv().await, Some(&metrics));
}

#[tokio::test]
async fn lagged_receiver_increments_lag_counter() {
    let metrics = Metrics::new(false);
    let bus: Bus<String> = Bus::new().with_metrics(metrics.clone());
    let mut rx = bus.subscribe();

    for i in 0..2048 {
        let _ = bus.publish(format!("m{i}"));
    }

    let _ = Bus::handle_recv(rx.recv().await, Some(&metrics));
}
