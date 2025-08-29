//! Prometheus metrics + simple health endpoints (Axum 0.7).
//! Routes:
//!   GET /metrics  -> Prometheus metrics
//!   GET /healthz  -> 200 if process up
//!   GET /readyz   -> 200 if all registered services are healthy

#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Registry, TextEncoder,
};
use serde::Serialize;
use tokio::{net::TcpListener, task::JoinHandle};

#[derive(Default, Clone)]
pub struct HealthState {
    inner: Arc<RwLock<HashMap<String, bool>>>,
}

impl HealthState {
    pub fn set(&self, service: impl Into<String>, ok: bool) {
        let mut m = self.inner.write().expect("health rwlock poisoned");
        m.insert(service.into(), ok);
    }
    pub fn set_all(&self, services: &[&str], ok: bool) {
        let mut m = self.inner.write().expect("health rwlock poisoned");
        for s in services {
            m.insert((*s).to_owned(), ok);
        }
    }
    pub fn map(&self) -> HashMap<String, bool> {
        self.inner.read().expect("health rwlock poisoned").clone()
    }
    pub fn is_ready(&self) -> bool {
        let m = self.inner.read().expect("health rwlock poisoned");
        !m.is_empty() && m.values().all(|v| *v)
    }
}

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    pub bytes_in: IntCounter,
    pub bytes_out: IntCounter,
    pub conns_gauge: IntGauge,
    pub restarts: IntCounterVec,
    pub req_latency: Histogram,
    health: HealthState,
}

/// JSON body for /readyz
#[derive(Serialize)]
struct ReadyReport {
    ready: bool,
    services: HashMap<String, bool>,
}

impl Metrics {
    /// Build a metrics registry with common series pre-registered.
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();

        let bytes_in = IntCounter::new("ron_bytes_in_total", "Total bytes received").unwrap();
        let bytes_out = IntCounter::new("ron_bytes_out_total", "Total bytes sent").unwrap();
        let conns_gauge = IntGauge::new("ron_active_connections", "Active transport connections")
            .unwrap();
        let restarts = IntCounterVec::new(
            prometheus::Opts::new("ron_service_restarts_total", "Service restarts by name"),
            &["service"],
        )
        .unwrap();
        let req_latency = Histogram::with_opts(
            HistogramOpts::new("ron_request_latency_seconds", "Request latency")
                .buckets(prometheus::exponential_buckets(0.001, 2.0, 16).unwrap()),
        )
        .unwrap();

        registry.register(Box::new(bytes_in.clone())).unwrap();
        registry.register(Box::new(bytes_out.clone())).unwrap();
        registry.register(Box::new(conns_gauge.clone())).unwrap();
        registry.register(Box::new(restarts.clone())).unwrap();
        registry.register(Box::new(req_latency.clone())).unwrap();

        Arc::new(Self {
            registry,
            bytes_in,
            bytes_out,
            conns_gauge,
            restarts,
            req_latency,
            health: HealthState::default(),
        })
    }

    pub fn health(&self) -> &HealthState {
        &self.health
    }

    /// Build a router exposing /metrics, /healthz, /readyz from this Metrics instance.
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/metrics", get(Self::metrics_handler))
            .route("/healthz", get(Self::healthz_handler))
            .route("/readyz", get(Self::readyz_handler))
            .with_state(self)
    }

    /// Spawn an Axum server on `addr` (use 127.0.0.1:0 for an ephemeral port).
    /// Returns a join handle and the bound local address.
    pub async fn serve(self: Arc<Self>, addr: SocketAddr) -> (JoinHandle<()>, SocketAddr) {
        let listener = TcpListener::bind(addr).await.expect("bind /metrics");
        let local = listener.local_addr().expect("local_addr");
        let app = self.router();
        let handle = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app).await {
                tracing::error!(%err, "metrics server error");
            }
        });
        (handle, local)
    }

    async fn metrics_handler(State(me): State<Arc<Metrics>>) -> impl IntoResponse {
        let metric_families = me.registry.gather();
        let mut buf = Vec::new();
        let encoder = TextEncoder::new();
        if let Err(err) = encoder.encode(&metric_families, &mut buf) {
            tracing::error!(%err, "encode prometheus");
        }
        (axum::http::StatusCode::OK, buf)
    }

    async fn healthz_handler() -> impl IntoResponse {
        axum::http::StatusCode::OK
    }

    async fn readyz_handler(State(me): State<Arc<Metrics>>) -> impl IntoResponse {
        let ready = me.health.is_ready();
        let body = ReadyReport {
            ready,
            services: me.health.map(),
        };
        let status = if ready {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        };
        (status, Json(body))
    }
}
