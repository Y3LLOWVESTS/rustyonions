//! RO:WHAT — Binary entrypoint: loads config, boots HTTP, exposes /metrics,/healthz,/readyz.
//! RO:WHY  — Service wrapper around library surfaces.
//! RO:INTERACTS — bootstrap::run, telemetry::tracing_init
//! RO:INVARIANTS — truthful readiness; graceful shutdown; no locks across .await

use std::net::SocketAddr;
use svc_passport::{bootstrap, telemetry::tracing_init, Config};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_init::init();
    let cfg = Config::load()?;
    let bind: SocketAddr = cfg.server.bind.parse()?;
    let admin: SocketAddr = cfg.server.admin_bind.parse()?;

    let (_http, http_addr) = bootstrap::run(bind, admin, cfg).await?;
    info!(%http_addr, "svc-passport: listening");
    // Block until Ctrl-C
    tokio::signal::ctrl_c().await?;
    Ok(())
}
