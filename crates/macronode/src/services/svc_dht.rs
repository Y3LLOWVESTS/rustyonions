//! RO:WHAT — Macronode stub for svc-dht.
//! RO:WHY  — Reserve a home for DHT / routing table workers.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.

use std::time::Duration;

use tokio::time::sleep;
use tracing::info;

use crate::supervisor::ShutdownToken;

pub fn spawn(shutdown: ShutdownToken) {
    tokio::spawn(async move {
        info!("svc-dht: started (stub worker)");
        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }
        info!("svc-dht: shutdown requested, exiting worker");
    });
}
