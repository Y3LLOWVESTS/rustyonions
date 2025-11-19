//! RO:WHAT — Macronode stub for svc-storage.
//! RO:WHY  — Reserve a home for content-addressed blob storage workers.
//! RO:INVARIANTS —
//!   - Worker runs until process shutdown (infinite sleepy loop for now).

use std::time::Duration;

use tokio::time::sleep;
use tracing::info;

pub fn spawn() {
    tokio::spawn(async {
        info!("svc-storage: started (stub worker)");
        loop {
            sleep(Duration::from_secs(3600)).await;
        }
    });
}
