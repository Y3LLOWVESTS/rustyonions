// crates/macronode/src/services/svc_dht.rs

//! RO:WHAT — Macronode wrapper for the DHT/routing plane.
//! RO:WHY  — Reserve a config-aware home for DHT workers so macronode can
//!           coordinate them without owning DHT internals.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.
//!   - Bind address is resolved once and logged for operator introspection.
//!   - This module owns only host wiring; DHT semantics live in `svc-dht`.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    services::ports,
    supervisor::{ManagedTask, ShutdownToken},
};

/// Resolve the bind address for the DHT plane.
///
/// Env override:
///   - `RON_DHT_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    match std::env::var("RON_DHT_ADDR") {
        Ok(raw) => match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-dht: using RON_DHT_ADDR={addr}");
                addr
            }
            Err(err) => {
                error!(
                    "svc-dht: invalid RON_DHT_ADDR={raw:?}, falling back to {}: {err}",
                    ports::DEFAULT_DHT_ADDR_STR
                );
                ports::default_dht_addr()
            }
        },
        Err(_) => ports::default_dht_addr(),
    }
}

/// Spawn the DHT worker (stub).
pub fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();
        info!(%addr, "svc-dht: started (host shell, waiting for real svc-dht wiring)");

        probes.set_dht_bound(true);

        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }

        info!("svc-dht: shutdown requested, exiting worker");
    });

    ManagedTask::new("svc-dht", handle)
}
