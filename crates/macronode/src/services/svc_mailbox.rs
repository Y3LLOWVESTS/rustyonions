// crates/macronode/src/services/svc_mailbox.rs

//! RO:WHAT — Macronode wrapper for the mailbox plane.
//! RO:WHY  — Reserve a real home for queued message delivery / mailbox semantics
//!           with config-aware bind addresses and clean shutdown wiring.
//! RO:INVARIANTS —
//!   - Worker runs until shutdown is requested via `ShutdownToken`.
//!   - This module owns *only* host-level wiring; real mailbox logic lives elsewhere.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    services::ports,
    supervisor::{ManagedTask, ShutdownToken},
};

/// Resolve the bind address for the mailbox plane.
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
                    "svc-mailbox: invalid RON_MAILBOX_ADDR={raw:?}, falling back to {}: {err}",
                    ports::DEFAULT_MAILBOX_ADDR_STR
                );
                ports::default_mailbox_addr()
            }
        },
        Err(_) => ports::default_mailbox_addr(),
    }
}

/// Spawn the mailbox worker (stub).
pub fn spawn(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();
        info!(
            %addr,
            "svc-mailbox: started (host shell, waiting for real svc-mailbox wiring)"
        );

        probes.set_mailbox_bound(true);

        while !shutdown.is_triggered() {
            sleep(Duration::from_secs(5)).await;
        }

        info!("svc-mailbox: shutdown requested, exiting worker");
    });

    ManagedTask::new("svc-mailbox", handle)
}
