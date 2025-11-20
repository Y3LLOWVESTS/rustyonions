//! RO:WHAT — Service startup wiring for Macronode.
//! RO:WHY  — Centralized place to start all internal services.

use std::sync::Arc;

use tracing::info;

use crate::{errors::Result, readiness::ReadyProbes, supervisor::ShutdownToken};

pub async fn spawn_all(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> Result<()> {
    info!("macronode supervisor: spawn_all (starting service workers)");

    // Now takes probes so it can mark gateway_bound = true.
    crate::services::svc_gateway::spawn(probes.clone());

    crate::services::svc_overlay::spawn(shutdown.clone());
    crate::services::svc_index::spawn(shutdown.clone());
    crate::services::svc_storage::spawn(shutdown.clone());
    crate::services::svc_mailbox::spawn(shutdown.clone());
    crate::services::svc_dht::spawn(shutdown);

    probes.set_deps_ok(true);

    Ok(())
}
