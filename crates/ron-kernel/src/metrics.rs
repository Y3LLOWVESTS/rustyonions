#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use prometheus::{
    self as prom, Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, TextEncoder, register,
};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::info;

/// Shared health state exposed via /healthz and used by /readyz.
#[derive(Default)]
pub struct HealthState {
    inner: parking_lot::RwLock<HashMap<String, bool>>,
}

impl HealthState {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }

    /// Mark a service as healthy/unhealthy.
    pub fn set(&self, service: impl Into<String>, ok: bool) {
        let mut g = self.inner.write();
        g.insert(service.into(), ok);
    }

    /// Take a snapshot for JSON responses or checks.
    pub fn snapshot(&self) -> HashMap<String, bool> {
        self.inner.read().clone()
    }

    /// Ready if all tracked services are healthy. If none tracked yet, not ready.
    pub fn all_ready(&self) -> bool {
        let g = self.inner.read();
        !g.is_empty() && g.values().all(|v| *v)
    }
}

// ---- Global, process-wide collectors registered exactly once ----

fn bus_lagged_total_static() -> &'static IntCounterVec {
    static V: OnceLock<IntCounterVec> = OnceLock::new();
    V.get_or_init(|| {
        let v = IntCounterVec::new(
            Opts::new("bus_lagged_total", "Number of lagged events observed by receivers"),
            &["service"],
        )
        .expect("new IntCounterVec(bus_lagged_total)");
        register(Box::new(v.clone())).expect("register bus_lagged_total");
        v
    })
}

fn service_restarts_total_static() -> &'static IntCounterVec {
    static V: OnceLock<IntCounterVec> = OnceLock::new();
    V.get_or_init(|| {
        let v = IntCounterVec::new(
            Opts::new("service_restarts_total", "Count of service restarts"),
            &["service"],
        )
        .expect("new IntCounterVec(service_restarts_total)");
        register(Box::new(v.clone())).expect("register service_restarts_total");
        v
    })
}

fn request_latency_seconds_static() -> &'static Histogram {
    static H: OnceLock<Histogram> = OnceLock::new();
    H.get_or_init(|| {
        // Default buckets are fine for tests; customize later if needed.
        let h = Histogram::with_opts(HistogramOpts::new("request_latency_seconds", "HTTP request latency"))
            .expect("new Histogram(request_latency_seconds)");
        register(Box::new(h.clone())).expect("register request_latency_seconds");
        h
    })
}

/// Metrics registry & HTTP admin server (/metrics, /healthz, /readyz).
#[derive(Clone)]
pub struct Metrics {
    health: Arc<HealthState>,

    // Example metrics registered to the default registry per blueprint.
    pub bus_lagged_total: IntCounterVec,
    pub service_restarts_total: IntCounterVec,
    pub request_latency_seconds: Histogram,
}

impl Metrics {
    /// Create Metrics and clone the globally-registered collectors.
    pub fn new() -> Self {
        Self {
            health: Arc::new(HealthState::new()),
            bus_lagged_total: bus_lagged_total_static().clone(),
            service_restarts_total: service_restarts_total_static().clone(),
            request_latency_seconds: request_latency_seconds_static().clone(),
        }
    }

    /// Expose a reference to health state (matches blueprint).
    pub fn health(&self) -> &HealthState {
        &self.health
    }

    /// Start the admin HTTP server. Returns a JoinHandle and the bound address.
    ///
    /// Endpoints:
    /// - GET /metrics  -> Prometheus text format
    /// - GET /healthz  -> JSON map of service->bool (liveness)
    /// - GET /readyz   -> 200 if all services are healthy; else 503
    pub async fn serve(self, addr: SocketAddr) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
        let health = self.health.clone();

        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/healthz", get(healthz_handler))
            .route("/readyz", get(readyz_handler))
            .with_state(AppState { health });

        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        info!(
            "Admin endpoints: /metrics /healthz /readyz at http://{}/",
            local_addr
        );

        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("metrics admin server error: {e}");
            }
        });

        Ok((handle, local_addr))
    }
}

#[derive(Clone)]
struct AppState {
    health: Arc<HealthState>,
}

async fn metrics_handler() -> impl IntoResponse {
    let metric_families = prom::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("encode error: {e}"),
        )
            .into_response();
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, encoder.format_type())
        .body(axum::body::Body::from(buf))
        .unwrap()
}

async fn healthz_handler(State(state): State<AppState>) -> impl IntoResponse {
    let snap = state.health.snapshot();
    Json(snap).into_response()
}

async fn readyz_handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.health.all_ready() {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}
