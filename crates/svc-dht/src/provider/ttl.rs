//! RO:WHAT — Background TTL pruning worker
//! RO:WHY — Keeps store clean over time without external triggers

use super::Store;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::debug;

pub fn spawn_pruner(store: Arc<Store>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Small initial delay to avoid racing immediately after a provide with very short TTLs.
        sleep(Duration::from_secs(2)).await;
        loop {
            let n = store.purge_expired();
            if n > 0 {
                debug!(purged = n, "provider TTL pruned");
            }
            sleep(Duration::from_secs(1)).await;
        }
    })
}
