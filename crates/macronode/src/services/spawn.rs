//! RO:WHAT — Single entrypoint for services composed by one CrabNode/macronode.
//! RO:WHY — CN-3 establishes profile identity, Omnigate, then canonical svc-gateway before remaining workers.
//! RO:INTERACTS — svc-passport, Omnigate, svc-gateway, storage, index, overlay, mailbox, DHT, readiness.
//! RO:INVARIANTS — no ping-only ingress; no copied routes; failed required startup aborts earlier required tasks.
//! RO:METRICS — canonical service owners retain their own metrics.
//! RO:CONFIG — service wrappers consume the canonical CrabNode profile/environment.
//! RO:SECURITY — identity uses a constrained router; public CrabNode injects durable KMS custody and Native Passport recovery without exposing KMS/admin routes; startup fails closed on required composition failure.
//! RO:TEST — crabnode_cn3_ingress.rs plus lifecycle/operator regressions.

use std::sync::Arc;

use tracing::info;

use crate::{
    errors::Result,
    readiness::ReadyProbes,
    supervisor::{ManagedTask, ShutdownToken},
    types::RuntimeStatus,
};

pub async fn spawn_all(
    probes: Arc<ReadyProbes>,
    shutdown: ShutdownToken,
    runtime: Arc<RuntimeStatus>,
) -> Result<Vec<ManagedTask>> {
    info!("macronode supervisor: spawn_all starting canonical CrabNode services");

    let passport = crate::services::svc_passport::spawn(probes.clone(), shutdown.clone()).await?;

    let omnigate = match crate::services::svc_omnigate::spawn(probes.clone()).await {
        Ok(task) => task,

        Err(err) => {
            probes.mark_service_unready("svc-passport");

            passport.handle.abort();

            return Err(err);
        }
    };

    let gateway = match crate::services::svc_gateway::spawn(probes.clone()).await {
        Ok(task) => task,

        Err(err) => {
            probes.mark_service_unready("omnigate");

            probes.mark_service_unready("svc-passport");

            omnigate.handle.abort();
            passport.handle.abort();

            return Err(err);
        }
    };

    let tasks: Vec<ManagedTask> = vec![
        passport,
        omnigate,
        gateway,
        crate::services::svc_index::spawn(probes.clone(), runtime.clone()),
        crate::services::svc_overlay::spawn(probes.clone(), shutdown.clone()),
        crate::services::svc_storage::spawn(probes.clone(), shutdown.clone(), runtime.clone()),
        crate::services::svc_mailbox::spawn(probes.clone(), shutdown.clone()),
        crate::services::svc_dht::spawn(probes.clone(), shutdown, runtime),
    ];

    probes.set_deps_ok(true);

    Ok(tasks)
}
