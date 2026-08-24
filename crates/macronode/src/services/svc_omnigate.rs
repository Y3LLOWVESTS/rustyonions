//! RO:WHAT — Compose canonical Omnigate inside the CrabNode/macronode runtime.
//! RO:WHY — CN-3 requires one real BFF behind canonical svc-gateway ingress.
//! RO:INTERACTS — omnigate::Config, omnigate::App, bootstrap server, ReadyProbes, spawn_all.
//! RO:INVARIANTS — route logic stays owned by Omnigate; successful bind opens only the Omnigate probe.
//! RO:METRICS — Omnigate retains its canonical metrics/admin exporter.
//! RO:CONFIG — OMNIGATE_BIND and OMNIGATE_METRICS_ADDR remain Omnigate-owned.
//! RO:SECURITY — canonical private-beta defaults remain loopback; no economic authority is added.
//! RO:TEST — crabnode_cn3_ingress.rs plus existing Omnigate tests.

#![forbid(unsafe_code)]

use std::sync::Arc;

use tracing::info;

use crate::{
    errors::{Error, Result},
    readiness::ReadyProbes,
    supervisor::ManagedTask,
};

pub async fn spawn(probes: Arc<ReadyProbes>) -> Result<ManagedTask> {
    probes.set_omnigate_bound(false);

    let cfg = omnigate::config::Config::load().map_err(Error::Other)?;

    let expected_bind = cfg.server.bind;

    let app = omnigate::App::build(cfg.clone())
        .await
        .map_err(Error::Other)?;

    let admin_addr = app.admin_addr;

    let (handle, actual_bind) = omnigate::bootstrap::server::serve(cfg.server, app.router)
        .await
        .map_err(Error::Other)?;

    if actual_bind != expected_bind {
        handle.abort();

        return Err(Error::config(format!(
            "Omnigate canonical bind mismatch: expected {expected_bind}, got {actual_bind}"
        )));
    }

    probes.set_omnigate_bound(true);

    info!(
        %actual_bind,
        %admin_addr,
        "CrabNode canonical Omnigate composed"
    );

    Ok(ManagedTask::new("omnigate", handle))
}
