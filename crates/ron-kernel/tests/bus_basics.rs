use ron_kernel::{Bus, KernelEvent, Metrics};

#[tokio::test]
async fn zero_and_one_subscriber_paths() {
    let metrics = Metrics::new(false);
    let bus: Bus<KernelEvent> = Bus::new().with_metrics(metrics.clone());

    // 0 subscribers -> publish returns 0
    let delivered = bus.publish(KernelEvent::Shutdown);
    assert_eq!(delivered, 0, "no subscribers -> delivered count should be 0");

    // subscribe one receiver -> publish returns 1
    let mut rx = bus.subscribe();
    let delivered2 = bus.publish(KernelEvent::Shutdown);
    assert_eq!(delivered2, 1, "one subscriber -> delivered should be 1");

    // drain without lag (and exercise helper)
    let _ = Bus::handle_recv(rx.recv().await, Some(&metrics));
}
