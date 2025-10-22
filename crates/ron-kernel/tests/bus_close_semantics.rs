//! When the sender is dropped, broadcast receivers observe `Closed`.

use ron_kernel::Bus;

#[tokio::test]
async fn receiver_observes_closed_on_sender_drop() {
    let bus: Bus<String> = Bus::with_capacity(8);
    let mut rx = bus.subscribe();

    // Drop all senders (cloned senders would need dropping too; we have only one)
    drop(bus);

    // Receiver should now get Err(Closed)
    let res = rx.recv().await;
    assert!(matches!(
        res,
        Err(tokio::sync::broadcast::error::RecvError::Closed)
    ));
}
