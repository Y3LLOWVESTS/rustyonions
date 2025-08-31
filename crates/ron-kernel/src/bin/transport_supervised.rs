#![forbid(unsafe_code)]

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use ron_kernel::{wait_for_ctrl_c, Bus, Metrics, HealthState};
use ron_kernel::cancel::Shutdown;
use ron_kernel::supervisor::Supervisor;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    crash: Arc<tokio::sync::Notify>,
    metrics: Arc<Metrics>,
}

#[derive(Clone)]
struct AdminState {
    health: Arc<HealthState>,
    metrics: Arc<Metrics>,
}

#[derive(Serialize)]
struct OkMsg {
    ok: bool,
    msg: &'static str,
}

/* =========================  Service #1: Demo HTTP  ========================= */

async fn run_http_service(sdn: Shutdown, state: AppState) -> anyhow::Result<()> {
    let addr: SocketAddr = "127.0.0.1:8088".parse().expect("socket addr");
    let app = Router::new()
        .route("/", get(root))
        .route("/crash", post(crash))
        .with_state(state.clone());

    let listener = TcpListener::bind(addr).await?;
    info!("demo HTTP listening on http://{}", addr);

    let serve_fut = async {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { sdn.cancelled().await })
            .await
            .map_err(|e| anyhow::anyhow!(e))
    };

    tokio::pin!(serve_fut);

    tokio::select! {
        res = &mut serve_fut => {
            res?;
            Ok(())
        }
        _ = state.crash.notified() => {
            warn!("Crash requested; stopping service with error to trigger restart");
            sleep(Duration::from_millis(200)).await;
            Err(anyhow::anyhow!("intentional crash requested by /crash"))
        }
    }
}

async fn root(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    let resp = (StatusCode::OK, Json(OkMsg { ok: true, msg: "hello from supervised transport" }));

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

async fn crash(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    state.crash.notify_waiters();
    let resp = (StatusCode::OK, Json(OkMsg { ok: true, msg: "crash requested; service will restart" }));

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

/* =======================  Service #2: Admin HTTP  ========================== */

async fn run_admin_service(sdn: Shutdown, state: AdminState) -> anyhow::Result<()> {
    let addr: SocketAddr = "127.0.0.1:9096".parse().expect("socket addr");
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_route))
        .with_state(state.clone());

    let listener = TcpListener::bind(addr).await?;
    info!("admin HTTP listening on http://{} (endpoints: /healthz /readyz /metrics)", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { sdn.cancelled().await })
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

async fn healthz(State(state): State<AdminState>) -> impl IntoResponse {
    let start = Instant::now();

    let resp = if state.health.all_ready() {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    };

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

async fn readyz(State(state): State<AdminState>) -> impl IntoResponse {
    let start = Instant::now();

    let resp = if state.health.all_ready() {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    };

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

async fn metrics_route(State(state): State<AdminState>) -> impl IntoResponse {
    let start = Instant::now();

    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let _ = encoder.encode(&metric_families, &mut buf);

    // Own the content-type string so we don't return a borrow tied to `encoder`.
    let ct: String = encoder.format_type().to_string();

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    (StatusCode::OK, [(CONTENT_TYPE, ct)], buf)
}

/* ================================  main  =================================== */

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).pretty().init();

    info!("Starting transport_supervised demo…");

    // Shared infra (Bus/Metrics/Health)
    let metrics = Arc::new(Metrics::new());
    let health  = Arc::new(HealthState::new());
    let bus     = Bus::new(1024);
    let sdn     = Shutdown::new();

    // Start config watcher (publishes KernelEvent::ConfigUpdated on change)
    let _cfg_watch = ron_kernel::config::spawn_config_watcher("config.toml", bus.clone(), health.clone());

    // Supervisor
    let mut sup = Supervisor::new(bus.clone(), metrics.clone(), health.clone(), sdn.clone());

    // Service #1: demo HTTP
    let state = AppState {
        crash: Arc::new(tokio::sync::Notify::new()),
        metrics: metrics.clone(),
    };
    sup.add_service("demo_http", move |sdn| {
        let state = state.clone();
        async move { run_http_service(sdn, state).await }
    });

    // Service #2: admin HTTP
    let admin_state = AdminState {
        health: health.clone(),
        metrics: metrics.clone(),
    };
    sup.add_service("admin_http", move |sdn| {
        let st = admin_state.clone();
        async move { run_admin_service(sdn, st).await }
    });

    let handle = sup.spawn();

    // Split the long message into multiple lines to avoid any truncation problems.
    info!("Try it:");
    info!("  curl -s http://127.0.0.1:8088/");
    info!("  curl -s -X POST http://127.0.0.1:8088/crash");
    info!("  curl -s http://127.0.0.1:9096/healthz");
    info!("  curl -s http://127.0.0.1:9096/readyz");
    info!("  curl -s http://127.0.0.1:9096/metrics | head -n 20");

    // Wait for Ctrl-C, then shut down gracefully
    let _ = wait_for_ctrl_c().await;
    info!("Ctrl-C received; shutting down…");
    handle.shutdown();
    handle.join().await?;

    Ok(())
}
