//! RO:WHAT — Service startup wiring for Macronode (MVP).
//! RO:WHY  — Give the supervisor a single async entrypoint to start all
//!           managed services.
//! RO:INVARIANTS —
//!   - This module is the single place where we decide which services to start.
//!   - Later: consult Config, register health reporters, and wire crash policy.

use tracing::info;

use crate::errors::Result;

/// Spawn all Macronode-managed services.
///
/// Today this spins up stub workers for:
///   - svc-gateway
///   - svc-overlay
///   - svc-index
///   - svc-storage
///   - svc-mailbox
///   - svc-dht
///
/// Each worker just logs a startup message and then sleeps forever. Future
/// slices will replace these with real service wiring.
pub async fn spawn_all() -> Result<()> {
    info!("macronode supervisor: spawn_all (starting service stubs)");

    crate::services::svc_gateway::spawn();
    crate::services::svc_overlay::spawn();
    crate::services::svc_index::spawn();
    crate::services::svc_storage::spawn();
    crate::services::svc_mailbox::spawn();
    crate::services::svc_dht::spawn();

    Ok(())
}
