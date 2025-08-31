#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use prometheus::{
    self as prom, Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, TextEncoder,
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
        Self {
            inner: Default::default(),
        }
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

/// Metrics registry & HTTP admin server (/metrics, /healthz, /readyz).
#[derive(Clone)]
pub struct Metrics {
    health: Arc<HealthState>,

    // Example metrics registered to the default registry per blueprint.
    pub bus_lagged_total: prom::IntCounterVec,
    pub service_restarts_total: prom::IntCounterVec,
    pub request_latency_seconds: Histogram,
}

impl Metrics {
    /// Create Metrics and register a baseline set to the default registry.
    pub fn new() -> Self {
        let bus_lagged_total = IntCounterVec::new(
            Opts::new(
                "bus_lagged_total",
                "Number of lagged events observed by receivers",
            ),
            &["service"],
        )
        .expect("new IntCounterVec");
        prom::register(Box::new(bus_lagged_total.clone())).expect("register bus_lagged_total");

        let service_restarts_total = IntCounterVec::new(
            Opts::new("service_restarts_total", "Count of service restarts"),
            &["service"],
        )
        .expect("new IntCounterVec");
        prom::register(Box::new(service_restarts_total.clone()))
            .expect("register service_restarts_total");

        let request_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "request_latency_seconds",
            "HTTP request latency",
        ))
        .expect("new Histogram");
        prom::register(Box::new(request_latency_seconds.clone()))
            .expect("register request_latency_seconds");

        Self {
            health: Arc::new(HealthState::new()),
            bus_lagged_total,
            service_restarts_total,
            request_latency_seconds,
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
            // Axum 0.7 server
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
