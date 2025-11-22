// crates/macronode/src/services/svc_overlay.rs

//! RO:WHAT — Macronode wrapper for the overlay plane.
//! RO:WHY  — Provide a config-aware shell for overlay / gossip / connection
//!           management so we can later drop in the real `svc-overlay` crate
//!           without touching the supervisor surface.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.
//!   - Bind address is resolved once at startup and logged.
//!   - No locks held across `.await` in this slice.
//!
//! RO:FUTURE —
//!   - Call into `svc-overlay` lib entrypoint with:
//!       * TransportConfig (from ron-transport)
//!       * Bus handle (for health/crash events)
//!       * ShutdownToken
//!   - Surface overlay health into `/api/v1/status` and Prometheus.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    supervisor::{ManagedTask, ShutdownToken},
};

/// Default bind for the overlay plane (local-only in this slice).
const DEFAULT_OVERLAY_ADDR: &str = "127.0.0.1:5301";

/// Resolve the bind address for the overlay plane.
///
/// Env override:
///   - `RON_OVERLAY_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    match std::env::var("RON_OVERLAY_ADDR") {
        Ok(raw) => match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-overlay: using RON_OVERLAY_ADDR={addr}");
                addr
            }
            Err(err) => {
                error!(
                    "svc-overlay: invalid RON_OVERLAY_ADDR={raw:?}, \
                     falling back to {DEFAULT_OVERLAY_ADDR}: {err}"
                );
                DEFAULT_OVERLAY_ADDR
                    .parse()
                    .expect("DEFAULT_OVERLAY_ADDR must be a valid SocketAddr")
            }
        },
        Err(_) => DEFAULT_OVERLAY_ADDR
            .parse()
            .expect("DEFAULT_OVERLAY_ADDR must be a valid SocketAddr"),
    }
}

/// Spawn the overlay worker.
///
/// This keeps behavior simple and test-friendly while giving us a stable
/// attach point for the future `svc-overlay` integration.
pub fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();
        info!(
            %addr,
            "svc-overlay: started (host shell, waiting for real svc-overlay wiring)"
        );

        // Flip the per-service readiness bit once the worker has started.
        probes.set_overlay_bound(true);

        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }

        info!("svc-overlay: shutdown requested, exiting worker");
    });

    ManagedTask::new("svc-overlay", handle)
}
