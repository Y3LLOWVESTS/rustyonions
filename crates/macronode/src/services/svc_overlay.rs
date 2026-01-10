// crates/macronode/src/services/svc_overlay.rs

//! RO:WHAT — Macronode wrapper for the overlay plane.
//! RO:WHY  — Provide a config-aware shell for overlay / gossip / connection
//!           management so we can later drop in the real `svc-overlay` crate
//!           without touching the supervisor surface.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.
//!   - Bind address is resolved once at startup and logged.
//!   - No locks held across `.await` in this slice.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    services::ports,
    supervisor::{ManagedTask, ShutdownToken},
};

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
                    "svc-overlay: invalid RON_OVERLAY_ADDR={raw:?}, falling back to {}: {err}",
                    ports::DEFAULT_OVERLAY_ADDR_STR
                );
                ports::default_overlay_addr()
            }
        },
        Err(_) => ports::default_overlay_addr(),
    }
}

/// Spawn the overlay worker (stub).
pub fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();
        info!(
            %addr,
            "svc-overlay: started (host shell, waiting for real svc-overlay wiring)"
        );

        probes.set_overlay_bound(true);

        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }

        info!("svc-overlay: shutdown requested, exiting worker");
    });

    ManagedTask::new("svc-overlay", handle)
}
