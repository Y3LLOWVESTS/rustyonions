// crates/macronode/src/services/spawn.rs

//! RO:WHAT — Single entrypoint to spawn all managed Macronode services.
//! RO:WHY  — Keep supervisor wiring centralized so we can:
//!           * track JoinHandles for crash logging,
//!           * thread readiness probes into services,
//!           * pass shutdown tokens for graceful drain later.
//!
//! RO:INVARIANTS —
//!   - This slice still runs services until process shutdown (no restarts).
//!   - `ReadyProbes::set_deps_ok(true)` is flipped once workers are spawned;
//!     per-service bits (index/overlay/mailbox/dht) are flipped by the
//!     individual service modules.
//!   - No service-specific logic leaks into the supervisor; this module
//!     just coordinates spawns.

use std::sync::Arc;

use tracing::info;

use crate::{
    errors::Result,
    readiness::ReadyProbes,
    supervisor::{ManagedTask, ShutdownToken},
};

/// Spawn all managed services.
///
/// Today this is still “fire-and-forget”: each service runs until process
/// shutdown. We collect the JoinHandles as `ManagedTask`s so the Supervisor
/// can monitor exits and log them. No restart policies wired yet.
pub async fn spawn_all(
    probes: Arc<ReadyProbes>,
    shutdown: ShutdownToken,
) -> Result<Vec<ManagedTask>> {
    info!("macronode supervisor: spawn_all (starting service workers)");

    let tasks: Vec<ManagedTask> = vec![
        // Gateway: real HTTP ingress, marks gateway_bound=true when listener binds.
        crate::services::svc_gateway::spawn(probes.clone()),
        // svc-index: real embedded HTTP server using svc-index crate.
        // Flips index_bound=true once its listener binds.
        crate::services::svc_index::spawn(probes.clone()),
        // Remaining services are still stub workers; they just loop until shutdown.
        // Each one flips its own per-service readiness bit when the worker starts.
        crate::services::svc_overlay::spawn(probes.clone(), shutdown.clone()),
        crate::services::svc_storage::spawn(shutdown.clone()),
        crate::services::svc_mailbox::spawn(probes.clone(), shutdown.clone()),
        crate::services::svc_dht::spawn(probes.clone(), shutdown),
    ];

    // All deps are considered "ok" once their workers have been spawned.
    // At this slice we don’t yet distinguish per-dep gating in `required_ready()`.
    probes.set_deps_ok(true);

    Ok(tasks)
}
