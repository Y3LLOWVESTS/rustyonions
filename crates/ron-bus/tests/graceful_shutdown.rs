// RO:WHAT — Graceful shutdown convergence test.
// RO:WHY  — Final Shutdown should let receivers exit promptly (no hangs).
// RO:INTERACTS — Bus, BusConfig, Event.
// RO:INVARIANTS — Bounded, no background tasks; receivers exit on Shutdown.

use ron_bus::{Bus, BusConfig, Event};
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_shutdown_converges() {
    let bus = Bus::new(BusConfig::new().with_capacity(256)).unwrap();
    let tx = bus.sender();

    let mut rx1 = bus.subscribe();
    let t1 = tokio::spawn(async move {
        loop {
            match rx1.recv().await {
                Ok(Event::Shutdown) | Err(_) => break,
                _ => {}
            }
        }
    });

    let mut rx2 = bus.subscribe();
    let t2 = tokio::spawn(async move {
        loop {
            match rx2.recv().await {
                Ok(Event::Shutdown) | Err(_) => break,
                _ => {}
            }
        }
    });

    // Emit some traffic then Shutdown
    for i in 0..10 {
        let _ = tx.send(Event::ConfigUpdated { version: i });
    }
    let _ = tx.send(Event::Shutdown);

    // Must converge quickly
    timeout(Duration::from_secs(2), async { let _ = tokio::join!(t1, t2); })
        .await
        .expect("receivers failed to observe Shutdown in time");
}
