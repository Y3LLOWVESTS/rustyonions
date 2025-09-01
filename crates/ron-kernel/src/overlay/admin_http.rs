#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use tokio::net::TcpListener;
use tracing::info;

use crate::{cancel::Shutdown, metrics::HealthState, Metrics};

pub async fn run(
    sdn: Shutdown,
    health: Arc<HealthState>,
    _metrics: Arc<Metrics>,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    #[derive(Clone)]
    struct AdminState { health: Arc<HealthState> }

    async fn healthz(State(st): State<AdminState>) -> impl IntoResponse {
        if st.health.all_ready() { (StatusCode::OK, "ok") } else { (StatusCode::SERVICE_UNAVAILABLE, "not ready") }
    }
    async fn readyz (State(st): State<AdminState>) -> impl IntoResponse {
        if st.health.all_ready() { (StatusCode::OK, "ready") } else { (StatusCode::SERVICE_UNAVAILABLE, "not ready") }
    }
    async fn metrics_route() -> impl IntoResponse {
        let mf = prometheus::gather(); let mut buf = Vec::new(); let enc = TextEncoder::new(); let _ = enc.encode(&mf, &mut buf);
        (StatusCode::OK, [("Content-Type", enc.format_type().to_string())], buf)
    }

    let state = AdminState { health };
    let app = Router::new().route("/healthz", get(healthz)).route("/readyz", get(readyz)).route("/metrics", get(metrics_route)).with_state(state);
    let listener = TcpListener::bind(addr).await?; info!("admin HTTP listening on http://{addr}");
    axum::serve(listener, app).with_graceful_shutdown(async move { sdn.cancelled().await }).await.map_err(|e| anyhow::anyhow!(e))
}
