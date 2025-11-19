//! RO:WHAT — Macronode stub for svc-gateway.
//! RO:WHY  — Reserve a home for HTTP ingress / edge plane integration.
//! RO:INVARIANTS —
//!   - Worker runs until process shutdown (infinite sleepy loop for now).

use std::time::Duration;

use tokio::time::sleep;
use tracing::info;

pub fn spawn() {
    tokio::spawn(async {
        info!("svc-gateway: started (stub worker)");
        loop {
            sleep(Duration::from_secs(3600)).await;
        }
    });
}
