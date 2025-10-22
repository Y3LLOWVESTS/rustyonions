//! Loom interleavings for Bus subscribe/publish ensure no panics or deadlocks.
//! Run with: cargo test -p ron-kernel --features loom -- --nocapture

#![cfg(feature = "loom")]

use loom::thread;
use ron_kernel::{Bus, KernelEvent, Metrics};

#[test]
fn bus_publish_subscribe_concurrent() {
    loom::model(|| {
        let metrics = Metrics::new(false);
        let bus: Bus<KernelEvent> = metrics.make_bus(8);

        let bus_pub = bus.clone();
        let t_pub = thread::spawn(move || {
            let _ = bus_pub.publish(KernelEvent::Shutdown);
            let _ = bus_pub.publish(KernelEvent::ConfigUpdated { version: 1 });
        });

        let bus_sub = bus.clone();
        let t_sub = thread::spawn(move || {
            let mut rx = bus_sub.subscribe();
            let _ = rx.recv();
            let _ = rx.recv();
        });

        t_pub.join().unwrap();
        t_sub.join().unwrap();
    });
}
