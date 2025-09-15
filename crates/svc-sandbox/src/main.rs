mod hardening;
mod metrics;
mod decoy;
mod oap_stub;
mod tarpit;
mod router;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
    middleware,
};
use bytes::Bytes;
use metrics::SandboxMetrics;
use parking_lot::RwLock;
use rand::{rngs::StdRng, SeedableRng};
use serde::Serialize;
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tokio::signal;
use tracing::{info, warn, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Redirect,
    Mirror,
    Tarpit,
}

impl Mode {
    fn from_env() -> Self {
        match std::env::var("SANDBOX_MODE").unwrap_or_else(|_| "redirect".into()).to_lowercase().as_str() {
            "mirror" => Mode::Mirror,
            "tarpit" => Mode::Tarpit,
            _ => Mode::Redirect,
        }
    }
}

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    mode: Mode,
    max_body: usize,
    tarpit_min_ms: u64,
    tarpit_max_ms: u64,
    metrics: Arc<SandboxMetrics>,
    decoys: Arc<RwLock<decoy::DecoyCatalog>>,
    /// Sticky diversion fingerprints for telemetry/demo
    sticky: Arc<RwLock<HashSet<String>>>,
    service_name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct RootStatus<'a> {
    service: &'a str,
    version: &'a str,
    mode: &'a str,
    decoy_assets: usize,
    uptime_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let bind: SocketAddr = std::env::var("SANDBOX_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3005".to_string())
        .parse()
        .expect("SANDBOX_ADDR host:port");

    let mode = Mode::from_env();
    let max_body: usize = std::env::var("SANDBOX_MAX_BODY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);

    let tarpit_min_ms: u64 = std::env::var("SANDBOX_TARPIT_MS_MIN").ok().and_then(|s| s.parse().ok()).unwrap_or(250);
    let tarpit_max_ms: u64 = std::env::var("SANDBOX_TARPIT_MS_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(2_000);

    let seed: u64 = std::env::var("SANDBOX_DECOY_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5EED_5EED);

    let mut rng = StdRng::seed_from_u64(seed);
    let decoys = decoy::DecoyCatalog::generate(&mut rng, 64);

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        mode,
        max_body,
        tarpit_min_ms,
        tarpit_max_ms,
        metrics: Arc::new(SandboxMetrics::new()),
        decoys: Arc::new(RwLock::new(decoys)),
        sticky: Arc::new(RwLock::new(Default::default())),
        service_name: "svc-sandbox",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/version", get(version))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_endpoint))
        // deception API (looks plausible)
        .route("/assets/:id", get(get_asset))
        .route("/oap/v1/handshake", post(oap_handshake))
        // utility
        .route("/whoami", get(|| async { "sandbox\n" }))
        .with_state(state.clone());

    // Apply deception router middleware (fingerprint + sticky diversion telemetry)
    let app = app.layer(middleware::from_fn_with_state(state.clone(), router::deception_middleware));

    // Apply hardening limits (timeouts, concurrency, rate, body limit)
    let app = hardening::layer(state.max_body).layer(app);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("svc-sandbox listening on http://{bind} mode={:?}", state.mode);

    state.ready.store(true, Ordering::SeqCst);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = signal::ctrl_c().await;
            info!("shutdown signal received");
        })
        .await?;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_target(false).with_timer(fmt::time::uptime()).with_max_level(Level::INFO).with_env_filter(env_filter).init();
}

// ------------------- Handlers -------------------

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::ZERO).as_secs();
    let count = st.decoys.read().len();
    let mode = match st.mode { Mode::Redirect => "redirect", Mode::Mirror => "mirror", Mode::Tarpit => "tarpit" };

    Json(RootStatus {
        service: st.service_name,
        version: st.version,
        mode,
        decoy_assets: count,
        uptime_secs: up,
    })
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({ "service": st.service_name, "version": st.version }))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics_endpoint() -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = prometheus::TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("encode error: {e}")).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}

async fn get_asset(State(st): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    // Record stickiness on the asset id too (telemetry/demo)
    st.sticky.write().insert(id.clone());

    // Tar-pit if enabled
    tarpit::maybe_tarpit(st.mode, st.tarpit_min_ms, st.tarpit_max_ms, st.metrics.as_ref()).await;

    let cat = st.decoys.read();
    if let Some(asset) = cat.get(&id) {
        st.metrics.token_trip_total.inc();
        // stream in chunks; no unbounded Vec copies in hot path
        let bytes = Bytes::from(asset.bytes.clone());
        let chunk = 64 * 1024;
        let total = bytes.len();
        let stream = futures_util::stream::unfold(0usize, move |offset| {
            let b = bytes.clone();
            async move {
                if offset >= total {
                    None
                } else {
                    let end = (offset + chunk).min(total);
                    let slice = b.slice(offset..end);
                    Some((Ok::<Bytes, std::io::Error>(slice), end))
                }
            }
        });
        let body = axum::body::Body::from_stream(stream);
        let mut resp = axum::response::Response::new(body);
        resp.headers_mut().insert(axum::http::header::CONTENT_TYPE, asset.content_type.parse().unwrap());
        return resp;
    }

    (StatusCode::NOT_FOUND, "no such asset\n").into_response()
}

async fn oap_handshake(State(st): State<AppState>, axum::extract::Bytes payload: axum::extract::Bytes) -> impl IntoResponse {
    // enforce strict frame size
    if payload.len() > oap_stub::OAP1_MAX_FRAME {
        st.metrics.rejected_total.with_label_values(&["frame_too_large"]).inc();
        return (StatusCode::PAYLOAD_TOO_LARGE, Json(serde_json::json!({"error":"frame_too_large"})));
    }

    // Tar-pit if enabled
    tarpit::maybe_tarpit(st.mode, st.tarpit_min_ms, st.tarpit_max_ms, st.metrics.as_ref()).await;

    match oap_stub::handshake_stub(&payload) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(kind) => {
            st.metrics.rejected_total.with_label_values(&[kind.code()]).inc();
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": kind.code()}))).into_response()
        }
    }
}
