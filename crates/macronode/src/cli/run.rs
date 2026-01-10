//! RO:WHAT — Implementation of the `run` subcommand.
//! RO:WHY  — Bridge between CLI surface and the existing runtime wiring
//!           (config, logging, readiness, admin HTTP, supervisor).
//! RO:INVARIANTS —
//!   - Config pipeline: defaults -> file (optional) -> env -> CLI overlays.
//!   - `RunOpts` is the only source of CLI overrides.
//!   - HTTP admin server uses graceful shutdown on Ctrl-C.
//!   - No locks held across .await.

use std::{sync::Arc, time::Instant};

use axum::Router;
use ron_kernel::wait_for_ctrl_c;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::{
    bench::BenchManager,
    bus::NodeBus,
    config::{
        cli_overlay::{apply_cli_overlays, CliOverlay},
        load_effective_config,
    },
    errors::Result,
    http_admin,
    observability::{logging, net_accounting},
    readiness::ReadyProbes,
    supervisor::{ShutdownToken, Supervisor},
    types::AppState,
};

use super::RunOpts;

/// Execute the `run` subcommand.
pub async fn run(opts: RunOpts) -> Result<()> {
    // 1) Load config (defaults + optional file from CLI/env + env).
    //
    // Precedence for file path:
    //   1) CLI --config
    //   2) RON_CONFIG / MACRO_CONFIG (inside load_effective_config)
    let base_cfg = load_effective_config(opts.config_path.as_deref())?;

    // 2) Build CLI overlay from RunOpts and apply it.
    let overlay = CliOverlay {
        http_addr: opts.http_addr.clone(),
        metrics_addr: opts.metrics_addr.clone(),
        log_level: opts.log_level.clone(),
    };
    let cfg = apply_cli_overlays(base_cfg, &overlay)?;

    // 3) Initialize logging with config log level (RUST_LOG can still override).
    logging::init(&cfg.log_level);

    // 4) Build shared readiness probes and shutdown token.
    let probes = Arc::new(ReadyProbes::new());
    let shutdown_token = ShutdownToken::new();

    // 5) Start supervised services. Successful spawn marks deps_ok.
    let supervisor = Supervisor::new(probes.clone(), shutdown_token.clone());
    supervisor.start().await?;

    // 6) Start node-local network + request accounting sampler (for svc-admin rollups/charts).
    net_accounting::ensure_started(shutdown_token.clone());

    // 7) Build intra-node event bus.
    let bus = NodeBus::new();

    // 8) Build node-executed benchmark manager (bounded, safe loadgen).
    // Uses the node's own admin plane as the primary workload target.
    let bench = Arc::new(BenchManager::new(format!("http://{}", cfg.http_addr)));

    // 9) Build shared application state for HTTP handlers.
    let state = AppState {
        cfg: Arc::new(cfg.clone()),
        probes: probes.clone(),
        bus,
        started_at: Instant::now(),
        bench,
    };

    // 10) Bind HTTP admin listener.
    let listener = TcpListener::bind(cfg.http_addr).await?;
    probes.set_listeners_bound(true);
    probes.set_cfg_loaded(true);

    // Metrics are served on the admin listener in this slice.
    // Mark metrics bound now that we have a live listener.
    probes.set_metrics_bound(true);

    let router: Router = http_admin::router::build_router(state);

    info!("macronode admin listening on {}", cfg.http_addr);

    // 11) Run HTTP admin server with graceful shutdown on Ctrl-C.
    let shutdown_signal = async move {
        wait_for_ctrl_c().await;
        info!("macronode: shutdown signal received, draining admin server");
        shutdown_token.trigger();
    };

    if let Err(err) = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await
    {
        error!("macronode admin server error: {err}");
    }

    info!("macronode: admin server exited, shutdown complete");

    Ok(())
}
