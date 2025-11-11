//! Binary entrypoint for svc-edge.

use std::sync::Arc;

use axum::{routing::{get, post}, Router};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{fmt, EnvFilter};

use svc_edge::{
    admission,
    cli::Cli,
    readiness::readiness_handler,
    routes::{assets::echo, health::healthz, prometheus::metrics},
    wait_for_ctrl_c, AppState, Config, EdgeMetrics, HealthState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hyper=warn"));
    fmt().with_env_filter(env_filter).compact().init();

    // Parse CLI / config
    #[cfg(feature = "cli")]
    let cli = Cli::parse_from_env();
    #[cfg(not(feature = "cli"))]
    let cli = Cli::default();
    let cfg = Config::from_sources(cli.config_path.as_deref().and_then(|p| p.to_str()))?;

    // Health/metrics
    let health: Arc<HealthState> = Arc::new(HealthState::new());
    let edge_metrics = EdgeMetrics::new();
    // seed amnesia gauge from config
    edge_metrics.set_amnesia(cfg.security.amnesia);
    let state = AppState::new(cfg.clone(), edge_metrics.clone(), health.clone());

    // ----- Admin router (health/ready/metrics) -----
    let admin = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readiness_handler))
        .route("/metrics", get(metrics))
        .with_state(state.clone());

    // ----- API router (temporary: /echo) with admission layers -----
    let api = Router::new()
        .route("/echo", post(echo))
        .with_state(state.clone());
    let api = admission::apply_defaults(api);

    // Bind listeners
    let api_addr = cfg.bind_addr;
    let metrics_addr = cfg.metrics_addr;

    let admin_listener = TcpListener::bind(metrics_addr).await?;
    tracing::info!(%metrics_addr, "svc-edge: admin plane bound (health/ready/metrics)");

    let api_listener = TcpListener::bind(api_addr).await?;
    tracing::info!(%api_addr, "svc-edge: api plane bound");

    // Flip initial health gates after config loaded
    health.set("services_ok", true);
    health.set("config_loaded", true);

    // Shared shutdown token
    let token = CancellationToken::new();
    let t_admin = token.clone();
    let t_api = token.clone();

    // Task: wait for Ctrl-C / SIGTERM then cancel token
    let cancel_task = tokio::spawn({
        let token = token.clone();
        async move {
            shutdown_signal().await;
            token.cancel();
        }
    });

    // Serve both planes with graceful shutdown
    let admin_srv = axum::serve(admin_listener, admin)
        .with_graceful_shutdown(t_admin.cancelled());
    let api_srv = axum::serve(api_listener, api)
        .with_graceful_shutdown(t_api.cancelled());

    tracing::info!("svc-edge: starting (admin + api planes)");

    // Join servers first (concrete types inferred), then await canceller.
    let (r1, r2) = tokio::join!(admin_srv, api_srv);
    r1?;
    r2?;
    let _ = cancel_task.await;

    tracing::info!("svc-edge: shutdown complete");
    Ok(())
}

/// Unified shutdown signal: Ctrl-C on all platforms, plus SIGTERM on Unix.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = wait_for_ctrl_c() => {
                tracing::info!("shutdown: received Ctrl-C");
            }
            _ = term.recv() => {
                tracing::info!("shutdown: received SIGTERM");
            }
        }
    }
    #[cfg(not(unix))]
    {
        wait_for_ctrl_c().await;
        tracing::info!("shutdown: received Ctrl-C");
    }
}
