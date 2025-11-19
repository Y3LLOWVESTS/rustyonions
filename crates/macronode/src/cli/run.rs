//! RO:WHAT — Implementation of the `run` subcommand.
//! RO:WHY  — Bridge between CLI surface and the existing runtime wiring
//!           (config, logging, readiness, admin HTTP, supervisor).
//! RO:INVARIANTS —
//!   - Config pipeline: defaults -> env -> CLI overlays -> validate.
//!   - `RunOpts` is the only source of CLI overrides.

use std::{sync::Arc, time::Instant};

use axum::Router;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::{
    config::cli_overlay::{apply_cli_overlays, CliOverlay},
    config::load_config,
    errors::Result,
    http_admin,
    observability::logging,
    readiness::ReadyProbes,
    supervisor::Supervisor,
    types::AppState,
};

use super::RunOpts;

/// Execute the `run` subcommand.
pub async fn run(opts: RunOpts) -> Result<()> {
    // 1) Load config (defaults + env).
    let base_cfg = load_config()?;

    // 2) Build CLI overlay from RunOpts and apply it.
    let overlay = CliOverlay {
        http_addr: opts.http_addr.clone(),
        log_level: opts.log_level.clone(),
    };
    let cfg = apply_cli_overlays(base_cfg, &overlay)?;

    // 3) Initialize logging with config log level (RUST_LOG can still override).
    logging::init(&cfg.log_level);

    // 4) Build shared state.
    let probes = Arc::new(ReadyProbes::new());
    let state = AppState {
        cfg: Arc::new(cfg.clone()),
        probes: probes.clone(),
        started_at: Instant::now(),
    };

    // Mark metrics + deps as "bound/ok" for now (we already serve /metrics and
    // have no external deps wired yet).
    probes.set_metrics_bound(true);
    probes.set_deps_ok(true);

    // 4.5) Start supervised services (stub v1).
    let supervisor = Supervisor::new();
    supervisor.start().await?;

    // 5) Bind HTTP admin listener.
    let listener = TcpListener::bind(cfg.http_addr).await?;
    probes.set_listeners_bound(true);
    probes.set_cfg_loaded(true);

    let router: Router = http_admin::router::build_router(state);

    info!("macronode admin listening on {}", cfg.http_addr);

    if let Err(err) = axum::serve(listener, router).await {
        error!("macronode admin server error: {err}");
    }

    Ok(())
}
