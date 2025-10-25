use std::net::SocketAddr;
use std::time::Duration;

use ron_kernel::metrics::readiness::Readiness;
use ron_kernel::{HealthState, KernelEvent, Metrics};

#[tokio::main]
async fn main() {
    // Metrics + readiness demo server
    let metrics = Metrics::new(false);
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());
    ready.set_config_loaded(true);

    let (handle, local) = metrics
        .clone()
        .serve(
            "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            health.clone(),
            ready.clone(),
        )
        .await
        .unwrap();
    println!("metrics at http://{}/metrics", local);

    // Build a generic bus and attach metrics
    let bus = metrics.make_bus::<KernelEvent>(1024);

    // === Subscriber task =====================================================
    #[cfg(feature = "bus_edge_notify")]
    {
        // Edge-aware subscriber: disciplined drain loop (A5)
        let mut sub = bus.subscribe_edge();
        tokio::spawn(async move {
            // sub_index is for labeling; not used by the inlined helper today
            sub.run_drain_loop(0).await;
        });
    }

    #[cfg(not(feature = "bus_edge_notify"))]
    {
        // Classic subscriber: just recv in a loop
        let mut rx = bus.subscribe();
        tokio::spawn(async move { while rx.recv().await.is_ok() {} });
    }
    // ========================================================================

    // === Publisher demo workload ============================================
    // If `bus_batch` is enabled, publish in bursts via publish_many (A2).
    // Otherwise fall back to single-message publishes.
    #[cfg(feature = "bus_batch")]
    {
        // Replace the publisher loop (demo) to exercise batches
        let bus2 = bus.clone();
        tokio::spawn(async move {
            let mut v = 0u64;
            let mut scratch = Vec::with_capacity(256);
            loop {
                scratch.clear();
                for _ in 0..128 {
                    scratch.push(KernelEvent::ConfigUpdated { version: v });
                    v = v.wrapping_add(1);
                }
                // A2: single-sweep publish
                let _ = bus2.publish_many(&scratch);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        });
    }

    #[cfg(not(feature = "bus_batch"))]
    {
        // One-at-a-time publisher (original behavior)
        let bus2 = bus.clone();
        tokio::spawn(async move {
            let mut v = 0u64;
            loop {
                let _ = bus2.publish(KernelEvent::ConfigUpdated { version: v });
                v = v.wrapping_add(1);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }
    // ========================================================================

    tokio::signal::ctrl_c().await.unwrap();
    handle.abort();
}
