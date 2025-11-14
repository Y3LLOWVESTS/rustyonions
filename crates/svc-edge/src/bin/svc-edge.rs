/// GROK WROTE THIS (adapted to use svc-edge lib surface)
// svc-edge — bin wiring for Axum 0.7 with stateful routers and the real library state.
// - Uses Router directly with `axum::serve` (no manual make_service).
// - Clean graceful shutdown with CancellationToken.
// - Two listeners: admin (health/ready/metrics) and api (edge surface).
// RO:WHAT — Binary entrypoint for svc-edge.
// RO:WHY — Boots admin + API planes; uses Config/AppState/EdgeMetrics/HealthState from the lib.
// RO:INTERACTS — axum::serve, tokio::net, ron_kernel::wait_for_ctrl_c; AppState (config/metrics/health).
// RO:INVARIANTS — No ambient auth; binds config-driven; readiness is degrade-first until gates flip.
// RO:METRICS — /metrics exposes Prometheus; admission layer will tick rejects/latency later.
// RO:CONFIG — Uses `Config::from_sources(None)` (env + defaults) + temp env knobs for admission/assets.
// RO:SECURITY — Amnesia posture exposed via metrics (future: enforced persistence rules).
// RO:TEST — Driven by http_contract, readiness_logic, i_1_hardening_ingress, etc.

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};
use tokio::{net::TcpListener, signal};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, EnvFilter};

use svc_edge::admission;
use svc_edge::metrics::seed_from_health;
use svc_edge::readiness::readiness_handler;
use svc_edge::routes::{assets, health, prometheus};
use svc_edge::{wait_for_ctrl_c, AppState, Config, EdgeMetrics, HealthState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    // --- Config (from svc-edge::Config, env + defaults) --------------------
    // Reads SVC_EDGE_BIND_ADDR, SVC_EDGE_METRICS_ADDR, SVC_EDGE_SECURITY__AMNESIA, etc.
    let cfg: Config = Config::from_sources(None).context("load svc-edge config")?;

    let admin_addr: SocketAddr = cfg.metrics_addr;
    let api_addr: SocketAddr = cfg.bind_addr;

    // Admission env overrides (temporary until wired into Config fully).
    let timeout_ms: u64 = env_var_parse("SVC_EDGE_ADMISSION_TIMEOUT_MS", 5_000);
    let max_inflight: usize = env_var_parse("SVC_EDGE_ADMISSION_MAX_INFLIGHT", 256);

    // Asset root (temporary; real path will come from Config.assets.root).
    // Default to ./assets relative to current working dir.
    let assets_root: String = env::var("SVC_EDGE_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());

    info!(
        %admin_addr, %api_addr, timeout_ms, max_inflight, assets_root,
        "svc-edge: resolved bind addresses and admission caps"
    );

    // --- Health + metrics + shared AppState --------------------------------
    let health: Arc<HealthState> = Arc::new(HealthState::new());
    let metrics = EdgeMetrics::new();

    // Seed metrics with amnesia posture (additional gates will be wired later).
    seed_from_health(health.clone(), &metrics, cfg.security.amnesia);

    // Flip readiness: config_loaded=true (we made it here).
    health.set("config_loaded", true);

    let state = AppState::new(cfg, metrics, health.clone());

    // --- Simple service readiness probe (assets dir exists) -----------------
    // For now, require that the assets root exists to consider "services_ok".
    let services_ok = std::path::Path::new(&assets_root).exists();
    health.set("services_ok", services_ok);

    // --- Bind listeners -----------------------------------------------------
    let admin_listener = TcpListener::bind(admin_addr)
        .await
        .with_context(|| format!("bind admin listener at {admin_addr}"))?;
    let api_listener = TcpListener::bind(api_addr)
        .await
        .with_context(|| format!("bind api listener at {api_addr}"))?;

    info!(%admin_addr, "svc-edge: admin plane listening");
    info!(%api_addr, "svc-edge: api plane listening");

    // --- Build routers (admin + api) ---------------------------------------

    // Admin plane: /healthz, /readyz, /metrics.
    let admin_app: Router = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(readiness_handler))
        .route("/metrics", get(prometheus::metrics))
        .with_state(state.clone());

    // API plane: assets + CAS + test endpoints.
    let api_router: Router = Router::new()
        // Assets and CAS (basic ETag + Range)
        .route("/edge/assets/*path", get(assets::get_asset))
        .route("/cas/:algo/:digest", get(assets::get_cas))
        // Test helpers
        .route("/echo", post(assets::echo))
        .route("/echo/slow/:ms", post(assets::echo_slow))
        .with_state(state.clone());

    // Apply the admission chain with env-driven caps.
    let api_app: Router = admission::apply_with(
        api_router,
        Duration::from_millis(timeout_ms),
        max_inflight,
    );

    // --- Graceful shutdown wiring ------------------------------------------
    let cancel = CancellationToken::new();
    let t_admin = cancel.clone();
    let t_api = cancel.clone();

    let admin_srv = async move {
        axum::serve(admin_listener, admin_app)
            .with_graceful_shutdown(t_admin.cancelled_owned())
            .await
            .context("admin server failed")
    };

    let api_srv = async move {
        axum::serve(api_listener, api_app)
            .with_graceful_shutdown(t_api.cancelled_owned())
            .await
            .context("api server failed")
    };

    info!("svc-edge: up; waiting for traffic or shutdown signal");

    tokio::select! {
        res = admin_srv => {
            if let Err(e) = res {
                error!(error=%e, "admin server error");
                return Err(e);
            }
        }
        res = api_srv => {
            if let Err(e) = res {
                error!(error=%e, "api server error");
                return Err(e);
            }
        }
        _ = wait_for_shutdown_signal() => {
            info!("shutdown signal received");
        }
    }

    cancel.cancel();
    tokio::time::sleep(Duration::from_millis(100)).await;
    info!("svc-edge exiting");
    Ok(())
}

fn env_var_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    match std::env::var(key) {
        Ok(v) => v.parse().unwrap_or(default),
        Err(_) => default,
    }
}

async fn wait_for_shutdown_signal() {
    // Prefer the kernel helper if available, otherwise fallback.
    wait_for_ctrl_c().await;
    // Fallback: tokio ctrl-c (no-op if already triggered).
    let _ = signal::ctrl_c().await;
}

fn init_tracing() {
    // Example: RUST_LOG=info,hyper=warn,axum::rejection=trace
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hyper=warn"));

    fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
