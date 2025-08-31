#![forbid(unsafe_code)]

use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::HistogramTimer;
use ron_kernel::{wait_for_ctrl_c, Metrics};
use tokio::{net::TcpListener, time::sleep};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // === Metrics / Admin server ===
    let admin_addr: SocketAddr = "127.0.0.1:9096".parse()?;
    let metrics0 = Metrics::new();

    // Metrics::serve(self, ...) consumes self; call on a clone so we can still use metrics.
    let (_admin_task, bound) = metrics0.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    // Share Metrics via Arc
    let metrics = Arc::new(metrics0);

    // Mark this demo healthy so /readyz returns 200
    metrics.health().set("metrics_demo", true);
    info!("metrics_demo marked healthy");

    // === App server with an instrumented handler ===
    let app_addr: SocketAddr = "127.0.0.1:9091".parse()?;
    let listener = TcpListener::bind(app_addr).await?;
    let app = Router::new()
        .route("/ping", get(ping))
        .with_state(metrics.clone());

    info!(
        "App server (metrics_demo) listening on http://{}/ (GET /ping)",
        app_addr
    );

    // Graceful shutdown on Ctrl-C
    let shutdown = async {
        let _ = wait_for_ctrl_c().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    // Flip health to false on shutdown (optional)
    metrics.health().set("metrics_demo", false);
    Ok(())
}

async fn ping(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    // Record request latency via RAII timer (observes on drop)
    let _t: HistogramTimer = metrics.request_latency_seconds.start_timer();

    // Simulate a tiny bit of work so we see non-zero latency
    sleep(Duration::from_millis(2)).await;

    (StatusCode::OK, "pong").into_response()
}
