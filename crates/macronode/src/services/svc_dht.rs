// crates/macronode/src/services/svc_dht.rs

//! RO:WHAT — Macronode wrapper for the DHT/routing plane.
//! RO:WHY  — Reserve a config-aware home for DHT workers so macronode can
//!           coordinate them without owning DHT internals.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.
//!   - Bind address is resolved once and logged for operator introspection.
//!   - This module owns only host wiring; DHT semantics live in `svc-dht`.
//!
//! RO:FUTURE —
//!   - Call into `svc-dht` lib entrypoint with:
//!       * A transport handle (ron-transport).
//!       * Bus handle for overlay/DHT events.
//!       * ShutdownToken.
//!   - Expose routing health and table stats via metrics and `/status`.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    supervisor::{ManagedTask, ShutdownToken},
};

/// Default bind for the DHT plane (local-only in this slice).
const DEFAULT_DHT_ADDR: &str = "127.0.0.1:5302";

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
                    "svc-dht: invalid RON_DHT_ADDR={raw:?}, \
                     falling back to {DEFAULT_DHT_ADDR}: {err}"
                );
                DEFAULT_DHT_ADDR
                    .parse()
                    .expect("DEFAULT_DHT_ADDR must be a valid SocketAddr")
            }
        },
        Err(_) => DEFAULT_DHT_ADDR
            .parse()
            .expect("DEFAULT_DHT_ADDR must be a valid SocketAddr"),
    }
}

/// Spawn the DHT worker.
///
/// For now this is a stub loop that just logs the resolved address and
/// waits on the shutdown token.
pub fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();
        info!(
            %addr,
            "svc-dht: started (host shell, waiting for real svc-dht wiring)"
        );

        // Flip the per-service readiness bit once the worker has started.
        probes.set_dht_bound(true);

        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }

        info!("svc-dht: shutdown requested, exiting worker");
    });

    ManagedTask::new("svc-dht", handle)
}
