use crate::config::Config;
use crate::error::{Error, Result};
use crate::observability;
use crate::router::build_router;
use crate::state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{net::TcpListener, signal};

/// Main entrypoint used by tests and the binary.
///
/// Starts the UI/API listener and a secondary metrics/health listener.
pub async fn run(config: Config) -> Result<()> {
    observability::init_tracing();

    let state = Arc::new(AppState::new(config.clone()));
    let app = build_router(state.clone());

    let bind_addr: SocketAddr = config
        .server
        .bind_addr
        .parse()
        .map_err(|e| Error::Config(format!("invalid bind_addr: {e}")))?;

    let metrics_addr: SocketAddr = config
        .server
        .metrics_addr
        .parse()
        .map_err(|e| Error::Config(format!("invalid metrics_addr: {e}")))?;

    let main_listener = TcpListener::bind(bind_addr).await?;
    let metrics_listener = TcpListener::bind(metrics_addr).await?;

    tracing::info!(%bind_addr, "svc-admin listening for UI/API");
    tracing::info!(%metrics_addr, "svc-admin listening for health/metrics");

    // Spawn secondary listener on metrics_addr. For now it reuses the same
    // router so `/healthz`, `/readyz`, and `/metrics` are available there.
    let metrics_app = app.clone();
    let metrics_task = tokio::spawn(async move {
        if let Err(err) = axum::serve(metrics_listener, metrics_app).await {
            tracing::error!(%err, "metrics/health listener crashed");
        }
    });

    axum::serve(main_listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| Error::Other(e.to_string()))?;

    // Allow the metrics task to wind down; we don't treat failure here as fatal.
    let _ = metrics_task.await;

    Ok(())
}

/// Public alias matching the README docs (`server::run_server(cfg)`).
pub async fn run_server(config: Config) -> Result<()> {
    run(config).await
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
