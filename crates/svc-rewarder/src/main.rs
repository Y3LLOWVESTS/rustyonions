//! RO:WHAT — Binary bootstrap for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF/DX. Keeps process setup thin and all logic in library modules.
//! RO:INTERACTS — config load, telemetry, http router, tokio signal shutdown.
//! RO:INVARIANTS — validated config before bind; no app logic in main; graceful Ctrl-C shutdown.
//! RO:METRICS — exposes /metrics through the HTTP router.
//! RO:CONFIG — reads --config, SVC_REWARDER_CONFIG, and SVC_REWARDER_* env overlays.
//! RO:SECURITY — does not log secrets; auth handled by route handlers.
//! RO:TEST — manual cargo run smoke plus integration tests against router.

use svc_rewarder::config::load_config_from_env;
use svc_rewarder::http::{routes::router, RewarderState};
use svc_rewarder::telemetry::init_tracing;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config_from_env()?;
    init_tracing(&cfg.log);
    svc_rewarder::security::tls::validate_tls_runtime(&cfg.tls)?;

    let bind_addr = cfg.bind_addr;
    let state = RewarderState::new(cfg)?;
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    info!(%local_addr, "svc-rewarder listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
