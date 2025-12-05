use crate::config::Config;
use crate::observability;
use crate::router::build_router;
use crate::state::AppState;
use crate::error::Result;
use axum::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;

pub async fn run(config: Config) -> Result<()> {
    observability::init_tracing();

    let state = Arc::new(AppState::new(config.clone()));
    let app = build_router(state.clone());

    let addr: SocketAddr = config
        .server
        .bind_addr
        .parse()
        .expect("invalid bind_addr in config");

    tracing::info!(%addr, "svc-admin listening");

    Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
