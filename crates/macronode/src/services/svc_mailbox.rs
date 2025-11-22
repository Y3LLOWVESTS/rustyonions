// crates/macronode/src/services/svc_mailbox.rs

//! RO:WHAT — Macronode wrapper for the mailbox plane.
//! RO:WHY  — Reserve a real home for queued message delivery / mailbox semantics
//!           with config-aware bind addresses and clean shutdown wiring.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.
//!   - No locks are held across `.await` (once we add async work here).
//!   - This module owns *only* host-level wiring; the real mailbox logic will
//!     live in `svc-mailbox` crate once that lib surface is ready.
//!
//! RO:FUTURE —
//!   - Swap the internal loop to call into `svc-mailbox` crate (HTTP/transport)
//!     without changing the supervisor or `spawn_all` signatures.
//!   - Emit health/metrics via ron-metrics once mailbox has a proper surface.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    supervisor::{ManagedTask, ShutdownToken},
};

/// Default bind for the mailbox plane (local-only in this slice).
const DEFAULT_MAILBOX_ADDR: &str = "127.0.0.1:5304";

/// Resolve the bind address for the mailbox plane.
///
/// Today this is purely informational: we log the resolved address so that
/// when `svc-mailbox` is wired in, it already has a stable config surface.
///
/// Env override:
///   - `RON_MAILBOX_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    match std::env::var("RON_MAILBOX_ADDR") {
        Ok(raw) => match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-mailbox: using RON_MAILBOX_ADDR={addr}");
                addr
            }
            Err(err) => {
                error!(
                    "svc-mailbox: invalid RON_MAILBOX_ADDR={raw:?}, \
                     falling back to {DEFAULT_MAILBOX_ADDR}: {err}"
                );
                DEFAULT_MAILBOX_ADDR
                    .parse()
                    .expect("DEFAULT_MAILBOX_ADDR must be a valid SocketAddr")
            }
        },
        Err(_) => DEFAULT_MAILBOX_ADDR
            .parse()
            .expect("DEFAULT_MAILBOX_ADDR must be a valid SocketAddr"),
    }
}

/// Spawn the mailbox worker.
///
/// This keeps behavior simple and test-friendly while giving us a stable
/// attach point for the future `svc-mailbox` integration.
pub fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();
        info!(
            %addr,
            "svc-mailbox: started (host shell, waiting for real svc-mailbox wiring)"
        );

        // Flip the per-service readiness bit once the worker has started.
        probes.set_mailbox_bound(true);

        // Lightweight wait loop; no busy-spin.
        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }

        info!("svc-mailbox: shutdown requested, exiting worker");
    });

    ManagedTask::new("svc-mailbox", handle)
}
