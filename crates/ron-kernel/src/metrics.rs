// crates/ron-kernel/src/metrics.rs

#![forbid(unsafe_code)]

use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{
    default_registry, Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Opts,
    TextEncoder,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::task::JoinHandle;

/// Simple multi-service health map. Thread-safe and Clone via Arc.
#[derive(Clone, Default)]
pub struct HealthState {
    inner: Arc<RwLock<HashMap<String, bool>>>,
}

impl HealthState {
    /// Set health for a single service.
    pub fn set(&self, service: impl Into<String>, ok: bool) {
        let mut g = self.inner.write().unwrap();
        g.insert(service.into(), ok);
    }

    /// Compatibility helper expected by some demo bins:
    /// bulk-set several services to the same health value.
    pub fn set_all<I, S>(&self, services: I, ok: bool)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut g = self.inner.write().unwrap();
        for s in services {
            g.insert(s.into(), ok);
        }
    }

    /// True if every tracked service reports OK.
    pub fn all_ok(&self) -> bool {
        let g = self.inner.read().unwrap();
        g.values().all(|v| *v)
    }
}

/// Kernel metrics (Prometheus).
#[derive(Clone)]
pub struct Metrics {
    /// Current active connections (canonical).
    pub connections_active: IntGauge,

    /// Compatibility alias used by older demo bins.
    /// This references the same underlying gauge as `connections_active`.
    pub conns_gauge: IntGauge,

    /// Total bytes counters.
    pub bytes_in: IntCounter,
    pub bytes_out: IntCounter,

    /// Request latency histogram.
    pub req_latency: Histogram,

    /// Error counter labeled by `service`.
    pub error_counter: IntCounterVec,

    /// Compatibility counter used by demo supervisor to track restarts.
    /// Labeled by `service`.
    pub restarts: IntCounterVec,

    /// Shared health state.
    health: HealthState,
}

impl Metrics {
    pub fn new() -> Self {
        // Build core metrics
        let gauge = IntGauge::new("connections_active", "Active connections").unwrap();
        let bytes_in = IntCounter::new("bytes_in_total", "Total bytes received").unwrap();
        let bytes_out = IntCounter::new("bytes_out_total", "Total bytes sent").unwrap();
        let req_latency =
            Histogram::with_opts(HistogramOpts::new("request_latency_seconds", "Request latency"))
                .unwrap();

        let error_counter = IntCounterVec::new(
            Opts::new("errors_total", "Error counter").const_label("component", "kernel"),
            &["service"],
        )
        .unwrap();

        let restarts = IntCounterVec::new(
            Opts::new("service_restarts_total", "Service restarts").const_label("component", "kernel"),
            &["service"],
        )
        .unwrap();

        // Register in default registry exactly once per metric.
        let r = default_registry();
        r.register(Box::new(gauge.clone())).ok();
        r.register(Box::new(bytes_in.clone())).ok();
        r.register(Box::new(bytes_out.clone())).ok();
        r.register(Box::new(req_latency.clone())).ok();
        r.register(Box::new(error_counter.clone())).ok();
        r.register(Box::new(restarts.clone())).ok();

        // Provide both canonical and compatibility handles to the SAME gauge.
        let connections_active = gauge.clone();
        let conns_gauge = gauge;

        Self {
            connections_active,
            conns_gauge,
            bytes_in,
            bytes_out,
            req_latency,
            error_counter,
            restarts,
            health: HealthState::default(),
        }
    }

    /// Borrow the shared health map.
    pub fn health(&self) -> &HealthState {
        &self.health
    }

    /// Serve /metrics, /healthz, /readyz on the given addr.
    ///
    /// NOTE: This returns a bare tuple, not a Result, to match older demo bins
    /// that destructure as `(handle, addr)` without `Ok(...)`.
    pub async fn serve(self, addr: SocketAddr) -> (JoinHandle<()>, SocketAddr) {
        let router = Router::new()
            .route("/metrics", get(|| async {
                let metric_families = default_registry().gather();
                let mut buffer = Vec::<u8>::new();
                TextEncoder::new()
                    .encode(&metric_families, &mut buffer)
                    .unwrap();
                String::from_utf8_lossy(&buffer).to_string()
            }))
            .route(
                "/healthz",
                get({
                    let health = self.health.clone();
                    move || {
                        let health = health.clone();
                        async move {
                            if health.all_ok() {
                                StatusCode::OK.into_response()
                            } else {
                                StatusCode::SERVICE_UNAVAILABLE.into_response()
                            }
                        }
                    }
                }),
            )
            .route("/readyz", get(|| async { StatusCode::OK.into_response() }));

        // Bind (panic on failure to match older demos' expectations).
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("metrics: failed to bind listener");
        let actual = listener
            .local_addr()
            .expect("metrics: failed to read local addr");

        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                eprintln!("metrics server error: {e}");
            }
        });

        (handle, actual)
    }
}
