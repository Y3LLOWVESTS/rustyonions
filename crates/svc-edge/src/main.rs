use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get},
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    service_name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    service: &'a str,
    version: &'a str,
    ok: bool,
    uptime_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let bind: SocketAddr = std::env::var("MICRONODE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3001".to_string())
        .parse()
        .expect("MICRONODE_ADDR must be host:port");

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        service_name: "micronode",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        // Service endpoints
        .route("/", get(root))
        .route("/status", get(status))
        .route("/version", get(version))
        // Ops endpoints
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("micronode listening on http://{bind}");

    // Mark ready after successful bind
    state.ready.store(true, Ordering::SeqCst);

    // Serve until Ctrl-C
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("micronode shutdown complete");
    Ok(())
}

fn init_tracing() {
    // Respect RUST_LOG if provided, default to info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .with_max_level(Level::INFO)
        .with_env_filter(env_filter)
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received Ctrl-C, shutting down…");
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: true,
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn status(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: st.ready.load(std::sync::atomic::Ordering::SeqCst),
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    let v = serde_json::json!({
        "service": st.service_name,
        "version": st.version
    });
    (StatusCode::OK, Json(v))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(std::sync::atomic::Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics() -> impl IntoResponse {
    // Use the default Prometheus registry; services can register counters/histograms elsewhere.
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        let body = format!("encode error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}
