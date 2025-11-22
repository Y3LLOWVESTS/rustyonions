//! RO:WHAT — Macronode embedded svc-storage HTTP server.
//! RO:WHY  — Run the CAS storage plane (svc-storage) in-process as part of the
//!           macronode profile, instead of a separate process.
//! RO:INVARIANTS —
//!   - Binds to 127.0.0.1:5303 by default (override via RON_STORAGE_ADDR).
//!   - Uses in-memory MemoryStorage backend only in this slice (no disk I/O).
//!   - No locks held across `.await`; storage crate owns all HTTP details.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::task;
use tracing::{error, info};

use crate::supervisor::{ManagedTask, ShutdownToken};
use svc_storage::http::{extractors::AppState, server::serve_http};
use svc_storage::storage::{MemoryStorage, Storage};

/// Resolve the bind address for the embedded storage HTTP server.
///
/// Default: `127.0.0.1:5303`  
/// Override: `RON_STORAGE_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    const DEFAULT_ADDR: &str = "127.0.0.1:5303";

    match std::env::var("RON_STORAGE_ADDR") {
        Ok(raw) => match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-storage: using RON_STORAGE_ADDR={addr}");
                addr
            }
            Err(err) => {
                error!(
                    "svc-storage: invalid RON_STORAGE_ADDR={raw:?}, \
                     falling back to {DEFAULT_ADDR}: {err}"
                );
                DEFAULT_ADDR
                    .parse()
                    .expect("DEFAULT_ADDR must be a valid SocketAddr")
            }
        },
        Err(_) => DEFAULT_ADDR
            .parse()
            .expect("DEFAULT_ADDR must be a valid SocketAddr"),
    }
}

/// Spawn the embedded svc-storage HTTP server.
///
/// We now return a `ManagedTask` so the supervisor can watch the JoinHandle.
/// `shutdown` is still unused because `serve_http` does not yet accept a
/// shutdown signal; process exit tears it down.
pub fn spawn(_shutdown: ShutdownToken) -> ManagedTask {
    let handle = task::spawn(async move {
        let addr = resolve_bind_addr();

        info!("svc-storage: listening on {addr} (embedded in macronode)");

        // In-memory store (matches svc-storage/bin main wiring).
        let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
        let state = AppState { store };

        // Delegate to svc-storage's HTTP server.
        match serve_http(addr, state).await {
            Ok(()) => {
                info!("svc-storage: server exited cleanly");
            }
            Err(err) => {
                error!("svc-storage: server error: {err:#}");
            }
        }
    });

    ManagedTask::new("svc-storage", handle)
}
