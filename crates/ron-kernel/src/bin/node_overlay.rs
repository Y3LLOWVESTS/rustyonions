#![forbid(unsafe_code)]

use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use prometheus::HistogramTimer;
use ron_kernel::{wait_for_ctrl_c, Metrics};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, time::sleep};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    metrics: Arc<Metrics>,
}

#[derive(Debug, Deserialize)]
struct EchoReq {
    payload: String,
}

#[derive(Debug, Serialize)]
struct EchoResp {
    echo: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // === Metrics / Admin server (overlay uses 9090) ===
    let admin_addr: SocketAddr = "127.0.0.1:9090".parse()?;
    let metrics0 = Metrics::new();
    let (_admin_task, bound) = metrics0.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    let metrics = Arc::new(metrics0);
    metrics.health().set("node_overlay", true);

    // === App server (overlay API on 8071) ===
    let app_addr: SocketAddr = "127.0.0.1:8071".parse()?;
    let listener = TcpListener::bind(app_addr).await?;
    let state = AppState {
        metrics: metrics.clone(),
    };

    let app = Router::new().route("/echo", post(echo)).with_state(state);

    info!(
        "node_overlay listening on http://{}/ (POST /echo)",
        app_addr
    );

    // Graceful shutdown on Ctrl-C
    let shutdown = async {
        let _ = wait_for_ctrl_c().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    metrics.health().set("node_overlay", false);
    Ok(())
}

async fn echo(State(state): State<AppState>, Json(req): Json<EchoReq>) -> impl IntoResponse {
    // Record latency
    let _t: HistogramTimer = state.metrics.request_latency_seconds.start_timer();

    // Simulate small work so histogram isn't zero
    sleep(Duration::from_millis(2)).await;

    let resp = EchoResp { echo: req.payload };
    (StatusCode::OK, Json(resp)).into_response()
}
