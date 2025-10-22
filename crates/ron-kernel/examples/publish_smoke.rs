//! RO:WHAT
//! Minimal smoke to exercise the bus and expose counters over /metrics.
//!
//! RO:WHY
//! Validate A2 (bus_batch) in a live process and make it trivial to curl the
//! exporter and confirm notify/batch/publish counters move.
//!
//! RO:INTERACTS
//! - ron_kernel::Metrics (serves /metrics on ephemeral port)
//! - ron_kernel::bus::bounded::{Bus, EdgeReceiver} (feature-gated)
//!
//! RO:INVARIANTS
//! - Runs indefinitely until Ctrl-C.
//! - Env-driven config for subs/cap/burst/tick to avoid code edits.

use std::{env, time::Duration};
use tokio::time::{interval, sleep};

use ron_kernel::Metrics;

fn getenv<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // --- Metrics + HTTP on an ephemeral port (":0") ---
    let metrics = Metrics::new(false);
    let (_server, addr) = {
        use ron_kernel::metrics::{health::HealthState, readiness::Readiness};
        let health = HealthState::new();
        let ready = Readiness::new(health.clone());
        // bind to :0 so OS picks a free port
        metrics
            .clone()
            .serve(([127, 0, 0, 1], 0).into(), health, ready)
            .await?
    };
    println!("metrics at http://{}/metrics", addr);
    println!("curl it in another terminal; press Ctrl-C here to stop …");

    // --- Config via env (defaults are sane for laptops) ---
    let cap = getenv::<usize>("RON_BENCH_CAP", 4096);
    let subs = getenv::<usize>("RON_BENCH_FANOUT", 4);
    let burst = getenv::<usize>("RON_BENCH_BURST", 256);
    let tick_ms = getenv::<u64>("RON_TICK_MS", 1000);
    println!(
        "[example cfg] subs={}, cap={}, burst={}, tick_ms={}",
        subs, cap, burst, tick_ms
    );

    // --- Bus + subscribers (edge receivers if feature enabled) ---
    let bus = metrics.make_bus::<u64>(cap);

    #[cfg(feature = "bus_edge_notify")]
    {
        use ron_kernel::bus::bounded::EdgeReceiver;
        for i in 0..subs {
            let mut edge: EdgeReceiver<u64> = bus.subscribe_edge();
            tokio::spawn(async move {
                edge.run_drain_loop(i).await;
            });
        }
    }
    #[cfg(not(feature = "bus_edge_notify"))]
    {
        let mut rxs: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();
        for mut rx in rxs {
            let m = metrics.clone();
            tokio::spawn(async move {
                loop {
                    // Drain as events arrive; account lag/drops via handle_recv if provided.
                    let _ = ron_kernel::bus::bounded::Bus::<u64>::handle_recv(rx.recv().await, Some(&m));
                }
            });
        }
    }

    // --- Periodic publisher ---
    // Aim for "burst" elements per tick. With bus_batch ON, this is one publish_many per burst.
    let mut tick = interval(Duration::from_millis(tick_ms));
    let mut next = 0u64;

    loop {
        tick.tick().await;

        #[cfg(feature = "bus_batch")]
        {
            // One batch per tick; adjust tick_ms to control overall rate
            let mut buf = Vec::with_capacity(burst);
            buf.clear();
            for _ in 0..burst {
                buf.push(next);
                next = next.wrapping_add(1);
            }
            let _ = bus.publish_many(&buf);
        }

        #[cfg(not(feature = "bus_batch"))]
        {
            for _ in 0..burst {
                let _ = bus.publish(next);
                next = next.wrapping_add(1);
            }
        }

        // Give TLS flusher a beat between bursts (if metrics_buf is on).
        sleep(Duration::from_millis(50)).await;
    }

    #[allow(unreachable_code)]
    {
        Ok(())
    }
}
