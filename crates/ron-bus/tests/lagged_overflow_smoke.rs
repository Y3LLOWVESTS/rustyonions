// RO:WHAT — Force lag on a slow subscriber to observe Lagged(n)
// RO:WHY  — Demonstrate lossy semantics under bounded overflow (host would count metrics)
// RO:INTERACTS — Bus, BusConfig, Event
// RO:INVARIANTS — publisher never blocks; slow consumer gets Lagged(n) and eventually exits on Shutdown
//
// Behavior notes:
// - We intentionally make the consumer slow to trigger RecvError::Lagged(n).
// - We publish Shutdown as the *last* message so that after any lag, the next Ok()
//   should observe Shutdown and break cleanly.

use ron_bus::{Bus, BusConfig, Event};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Barrier;
use tokio::time::{sleep, timeout, Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lagged_overflow_smoke() {
    let cap = 8u32; // tiny to trigger lag
    let bus = Bus::new(BusConfig::new().with_capacity(cap)).unwrap();
    let tx = bus.sender();

    let barrier = Arc::new(Barrier::new(2));

    // Slow consumer
    let mut rx_slow = bus.subscribe();
    let slow = {
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            barrier.wait().await;
            let mut lagged_total = 0u64;
            loop {
                match rx_slow.recv().await {
                    Ok(Event::Shutdown) => break lagged_total,
                    Ok(_ev) => {
                        // Simulate work; large enough to induce lag with small capacity.
                        sleep(Duration::from_millis(2)).await;
                    }
                    Err(RecvError::Lagged(n)) => {
                        lagged_total += n;
                        // host would: metrics::bus_overflow_dropped_total().inc_by(n as u64);
                    }
                    Err(RecvError::Closed) => break lagged_total,
                }
            }
        })
    };

    // Fast publisher loop
    let pubber = {
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            barrier.wait().await;
            for i in 0..200u32 {
                let _ = tx.send(Event::ConfigUpdated { version: i as u64 });
            }
            // Place Shutdown as the final message so late receivers converge to it.
            let _ = tx.send(Event::Shutdown);
        })
    };

    // Add a timeout safety net so the test never hangs indefinitely.
    let result = timeout(Duration::from_secs(5), async {
        let (_pub_res, lagged_total) = tokio::join!(pubber, slow);
        lagged_total.unwrap()
    })
    .await;

    match result {
        Ok(lagged_total) => {
            assert!(
                lagged_total > 0,
                "expected Lagged(n) to occur for slow consumer"
            );
        }
        Err(_elapsed) => panic!("test timed out waiting for consumer to observe Shutdown"),
    }
}
