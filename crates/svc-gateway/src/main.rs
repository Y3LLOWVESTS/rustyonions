//! svc-gateway binary (stub bootstrap)

use axum::Router;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Minimal tracing; respects RUST_LOG if set.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .try_init();

    let cfg = Config::load()?;
    let metrics_handles = metrics::register()?;

    // App state requires both Config and MetricsHandles
    let state = AppState::new(cfg.clone(), metrics_handles.clone());

    // Build the router from crate routes
    let router: Router = routes::build_router(&state);

    // Bind and serve with graceful shutdown
    let listener = TcpListener::bind(cfg.server.bind_addr).await?;
    info!("svc-gateway listening on {}", cfg.server.bind_addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    // CTRL+C to stop
    let _ = tokio::signal::ctrl_c().await;
}
