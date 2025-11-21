// crates/macronode/src/services/spawn.rs

//! RO:WHAT — Service startup wiring for Macronode.
//! RO:WHY  — Centralized place to start all internal services.
//! RO:INVARIANTS —
//!   - Called exactly once during supervisor startup.
//!   - Marks `deps_ok=true` once all service workers have been spawned.
//!   - Gateway is wired with `ReadyProbes` to flip `gateway_bound=true` on bind.
//!   - svc-index is now a *real* embedded HTTP service (not a sleep-loop stub).

use std::sync::Arc;

use tracing::info;

use crate::{errors::Result, readiness::ReadyProbes, supervisor::ShutdownToken};

/// Spawn all managed services.
///
/// Today this is still “fire-and-forget”: each service runs until process
/// shutdown. Future slices will return join handles and wire crash detection /
/// restart policies via the Supervisor.
pub async fn spawn_all(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> Result<()> {
    info!("macronode supervisor: spawn_all (starting service workers)");

    // Gateway: real HTTP ingress, marks gateway_bound=true when listener binds.
    crate::services::svc_gateway::spawn(probes.clone());

    // svc-index: now a real embedded HTTP server using svc-index crate.
    // We pass probes so we *can* hook readiness gates later; for now we rely
    // on the existing deps_ok flip below as the coarse “all deps spawned”.
    crate::services::svc_index::spawn(probes.clone());

    // Remaining services are still stub workers; they just loop until shutdown.
    crate::services::svc_overlay::spawn(shutdown.clone());
    crate::services::svc_storage::spawn(shutdown.clone());
    crate::services::svc_mailbox::spawn(shutdown.clone());
    crate::services::svc_dht::spawn(shutdown);

    // All deps are considered "ok" once their workers have been spawned.
    // At this slice we don’t yet distinguish per-dep readiness.
    probes.set_deps_ok(true);

    Ok(())
}
