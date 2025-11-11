#![deny(clippy::unwrap_used, clippy::expect_used, clippy::await_holding_lock)]

mod build_info;
mod config;
mod http;
mod observability;
mod storage;

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use ron_kernel::{wait_for_ctrl_c, HealthState};
use tokio::net::TcpListener;
use tracing::info;

use crate::config::load::load_config;
use crate::http::routes::registry_routes_with_cfg;
use crate::observability::endpoints::{admin_router, set_queues_ok, set_services_ok, AdminState};
use crate::observability::tracing::SERVICE_NAME;
use crate::storage::inmem::InMemoryStore;
use crate::storage::RegistryStore; // bring trait into scope for .subscribe()

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use the shared logger in observability::logging to avoid dead-code.
    crate::observability::logging::init_tracing();

    // Load config (env/file precedence handled in load_config)
    let cfg = load_config(None)?;
    let metrics = crate::observability::metrics::RegistryMetrics::new();
    let health = Arc::new(HealthState::default());

    // Spawn (stub) config reloader so the function isn't dead code.
    crate::config::reload::spawn_reloader();

    // Storage (in-memory for beta); after created, flip services_ok.
    let store = Arc::new(InMemoryStore::new());
    set_services_ok(&health, true);

    // Probe queues by trying a subscribe; on success flip queues_ok.
    {
        let _rx = store.subscribe();
        set_queues_ok(&health, true);
    }

    // Build routers
    let admin = admin_router(AdminState {
        health: health.clone(),
        build: build_info::build_info(),
        metrics: metrics.clone(),
    });
    let api: Router = registry_routes_with_cfg(metrics.clone(), store.clone(), &cfg);

    // Bind (cfg.metrics_addr == admin plane; cfg.bind_addr == public API)
    let admin_addr: SocketAddr = cfg.metrics_addr.parse()?;
    let api_addr: SocketAddr = cfg.bind_addr.parse()?;

    info!(
        service = SERVICE_NAME,
        admin_addr = %admin_addr,
        api_addr = %api_addr,
        "listening"
    );

    let admin_listener = TcpListener::bind(admin_addr).await?;
    let api_listener = TcpListener::bind(api_addr).await?;

    let admin_task = tokio::spawn(async move {
        axum::serve(admin_listener, admin.into_make_service())
            .with_graceful_shutdown(wait_for_ctrl_c())
            .await
            .ok();
    });

    let api_task = tokio::spawn(async move {
        axum::serve(api_listener, api.into_make_service())
            .with_graceful_shutdown(wait_for_ctrl_c())
            .await
            .ok();
    });

    // Wait for either to finish (Ctrl-C triggers graceful shutdown)
    let _ = tokio::join!(admin_task, api_task);

    Ok(())
}
