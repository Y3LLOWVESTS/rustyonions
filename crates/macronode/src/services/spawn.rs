// crates/macronode/src/services/spawn.rs

//! RO:WHAT — Single entrypoint to spawn all managed Macronode services.
//! RO:WHY  — Keep supervisor wiring centralized so we can:
//!           * track JoinHandles for crash logging,
//!           * thread readiness probes into services,
//!           * pass shutdown tokens for graceful drain later.
//!
//! RO:INVARIANTS —
//!   - This slice still runs services until process shutdown (no restarts).
//!   - `ReadyProbes::set_deps_ok(true)` records successful worker scheduling.
//!   - Gateway, storage, and index readiness flip only after real listener binds.
//!   - No service-specific logic leaks into the supervisor; this module
//!     just coordinates spawns.

use std::sync::Arc;

use tracing::info;

use crate::{
    errors::Result,
    readiness::ReadyProbes,
    supervisor::{ManagedTask, ShutdownToken},
    types::RuntimeStatus,
};

/// Spawn all managed services.
///
/// Today this is still “fire-and-forget”: each service runs until process
/// shutdown. We collect the JoinHandles as `ManagedTask`s so the Supervisor
/// can monitor exits and log them. No restart policies wired yet.
pub async fn spawn_all(
    probes: Arc<ReadyProbes>,
    shutdown: ShutdownToken,
    runtime: Arc<RuntimeStatus>,
) -> Result<Vec<ManagedTask>> {
    info!("macronode supervisor: spawn_all (starting service workers)");

    let tasks: Vec<ManagedTask> = vec![
        // Gateway: real HTTP ingress, marks gateway_bound=true when listener binds.
        crate::services::svc_gateway::spawn(probes.clone()),
        // svc-index: real embedded HTTP server using svc-index crate.
        // Registers its authoritative provider cache before readiness.
        crate::services::svc_index::spawn(probes.clone(), runtime.clone()),
        // These workers retain their existing lifecycle wiring.
        crate::services::svc_overlay::spawn(probes.clone(), shutdown.clone()),
        crate::services::svc_storage::spawn(probes.clone(), shutdown.clone(), runtime.clone()),
        crate::services::svc_mailbox::spawn(probes.clone(), shutdown.clone()),
        // DHT embeds svc-dht's canonical router and registers the exact local
        // provider store used by the prune coordinator.
        crate::services::svc_dht::spawn(probes.clone(), shutdown, runtime),
    ];

    // Workers were scheduled successfully. Truthful readiness still waits
    // for gateway, storage, and index to bind their real listeners.
    probes.set_deps_ok(true);

    Ok(tasks)
}
