// crates/macronode/src/services/svc_storage.rs

//! RO:WHAT — Macronode embedded svc-storage HTTP server.
//! RO:WHY  — Run the CAS storage plane (svc-storage) in-process as part of the
//!           macronode profile, instead of a separate process.
//! RO:INVARIANTS —
//!   - Binds to 127.0.0.1:5303 by default (override via RON_STORAGE_ADDR).
//!   - Uses in-memory MemoryStorage backend only in this slice (no disk I/O).
//!   - No locks held across `.await`; storage crate owns all HTTP details.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use tokio::task;
use tracing::{error, info};

use crate::services::ports;
use crate::supervisor::{ManagedTask, ShutdownToken};
use svc_storage::http::{extractors::AppState, server::serve_http};
use svc_storage::storage::{MemoryStorage, Storage};

/// Resolve the bind address for the embedded storage HTTP server.
///
/// Default: `127.0.0.1:5303`
/// Override: `RON_STORAGE_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    match std::env::var("RON_STORAGE_ADDR") {
        Ok(raw) => match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-storage: using RON_STORAGE_ADDR={addr}");
                addr
            }
            Err(err) => {
                error!(
                    "svc-storage: invalid RON_STORAGE_ADDR={raw:?}, falling back to {}: {err}",
                    ports::DEFAULT_STORAGE_ADDR_STR
                );
                ports::default_storage_addr()
            }
        },
        Err(_) => ports::default_storage_addr(),
    }
}

/// Spawn the embedded svc-storage HTTP server.
///
/// NOTE: `shutdown` is still unused because `serve_http` does not yet accept a
/// shutdown signal; process exit tears it down.
pub fn spawn(_shutdown: ShutdownToken) -> ManagedTask {
    let handle = task::spawn(async move {
        let addr = resolve_bind_addr();
        info!("svc-storage: listening on {addr} (embedded in macronode)");

        // In-memory store (matches svc-storage/bin main wiring).
        let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
        let state = AppState { store };

        if let Err(err) = serve_http(addr, state).await {
            error!(?err, "svc-storage: server error");
        }
    });

    ManagedTask::new("svc-storage", handle)
}
