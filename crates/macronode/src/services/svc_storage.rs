//! RO:WHAT — Macronode stub for svc-storage.
//! RO:WHY  — Reserve a home for content-addressed blob storage workers.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.

use std::time::Duration;

use tokio::time::sleep;
use tracing::info;

use crate::supervisor::ShutdownToken;

pub fn spawn(shutdown: ShutdownToken) {
    tokio::spawn(async move {
        info!("svc-storage: started (stub worker)");
        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }
        info!("svc-storage: shutdown requested, exiting worker");
    });
}
