//! RO:WHAT — Metrics facade and registry wiring for ron-metrics.
//! RO:WHY  — Single place to define/own metric families and expose helpers.
//! RO:INVARIANTS — no locks across .await; single registry instance; stable names.

use std::sync::Arc;

use crate::exposer::http::make_router;
use crate::health::HealthState;
use crate::readiness::ReadyPolicy; // <- import ReadyPolicy from the public module
use crate::registry::SafeRegistry;

use prometheus::{Histogram, HistogramOpts, IntCounterVec, IntGaugeVec, Opts};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::errors::MetricsError;
use crate::BaseLabels;

#[derive(Clone)]
pub struct Metrics {
    inner: Arc<Inner>,
}

struct Inner {
    registry: SafeRegistry,
    // Golden families
    pub service_restarts_total: IntCounterVec,
    pub bus_lagged_total: IntCounterVec,
    pub request_latency_seconds: Histogram,
    pub exposition_latency_seconds: Histogram,
    pub health_ready: IntGaugeVec,
    pub request_status_total: IntCounterVec,
    // Health state used by /healthz,/readyz
    pub health: HealthState,
}

impl Metrics {
    pub fn new(_base: BaseLabels, health: HealthState) -> Result<Self, MetricsError> {
        // Current SafeRegistry only exposes `new()`
        let registry = SafeRegistry::new();

        // ---- metric families ----
        let service_restarts_total = IntCounterVec::new(
            Opts::new("service_restarts_total", "Total restarts per component"),
            &["component"],
        )?;

        let bus_lagged_total = IntCounterVec::new(
            Opts::new("bus_lagged_total", "Overwrites due to lag/drop on bus"),
            &["bus"],
        )?;

        let request_latency_seconds = Histogram::with_opts(
            HistogramOpts::new("request_latency_seconds", "Request latency")
                .buckets(buckets::pow2_1ms_to_512ms()),
        )?;

        let exposition_latency_seconds = Histogram::with_opts(
            HistogramOpts::new("exposition_latency_seconds", "Latency to expose endpoints")
                .buckets(buckets::pow2_1ms_to_512ms()),
        )?;

        let health_ready =
            IntGaugeVec::new(Opts::new("health_ready", "Readiness (0/1)"), &["check"])?;

        let request_status_total = IntCounterVec::new(
            Opts::new("request_status_total", "Responses by status class"),
            &["status_class"],
        )?;

        // ---- register once with stable names ----
        registry.register("service_restarts_total", |r| {
            r.register(Box::new(service_restarts_total.clone()))
        })?;
        registry.register("bus_lagged_total", |r| {
            r.register(Box::new(bus_lagged_total.clone()))
        })?;
        registry.register("request_latency_seconds", |r| {
            r.register(Box::new(request_latency_seconds.clone()))
        })?;
        registry.register("exposition_latency_seconds", |r| {
            r.register(Box::new(exposition_latency_seconds.clone()))
        })?;
        registry.register("health_ready", |r| {
            r.register(Box::new(health_ready.clone()))
        })?;
        registry.register("request_status_total", |r| {
            r.register(Box::new(request_status_total.clone()))
        })?;

        Ok(Self {
            inner: Arc::new(Inner {
                registry,
                service_restarts_total,
                bus_lagged_total,
                request_latency_seconds,
                exposition_latency_seconds,
                health_ready,
                request_status_total,
                health,
            }),
        })
    }

    /// Exposer uses this to call `.gather()`.
    /// We return the wrapper so `exposer/http.rs` can do: `metrics.registry().gather()`.
    pub fn registry(&self) -> &SafeRegistry {
        &self.inner.registry
    }

    pub fn health(&self) -> &HealthState {
        &self.inner.health
    }

    // ---------- public helpers ----------

    pub fn inc_service_restart(&self, component: &str) {
        let _ = self
            .inner
            .service_restarts_total
            .with_label_values(&[component])
            .inc();
    }

    pub fn add_bus_lag(&self, bus: &str, overwritten: u64) {
        let _ = self
            .inner
            .bus_lagged_total
            .with_label_values(&[bus])
            .inc_by(overwritten);
    }

    pub fn observe_request(&self, seconds: f64) {
        self.inner.request_latency_seconds.observe(seconds);
    }

    /// Record status by class ("2xx", "4xx", ...)
    pub fn observe_status_class(&self, class: &str) {
        let _ = self
            .inner
            .request_status_total
            .with_label_values(&[class])
            .inc();
    }

    pub fn set_ready<S: Into<String>>(&self, check: S, ok: bool) {
        // avoid moving `check` twice
        let check_s: String = check.into();
        let n = if ok { 1 } else { 0 };
        let _ = self
            .inner
            .health_ready
            .with_label_values(&[&check_s])
            .set(n);
        self.inner.health.set(check_s, ok);
    }

    /// Spawn the HTTP server exposing /metrics,/healthz,/readyz
    pub async fn serve(
        self,
        addr: std::net::SocketAddr,
    ) -> Result<(JoinHandle<()>, std::net::SocketAddr), MetricsError> {
        let router = make_router(self.clone());
        let listener = TcpListener::bind(addr).await?;
        let local = listener.local_addr()?;
        let jh = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!(error=?e, "metrics HTTP server exited");
            }
        });
        Ok((jh, local))
    }

    /// Keep signature compatible with `exposer/http.rs` (it passes endpoint string)
    pub(crate) fn observe_exposition(&self, seconds: f64, _endpoint: &'static str) {
        self.inner.exposition_latency_seconds.observe(seconds);
    }

    /// Exposer expects a ready policy; keep default semantics
    pub fn ready_policy(&self) -> ReadyPolicy {
        ReadyPolicy::default()
    }
}

// Local buckets helper inside this module
pub mod buckets {
    pub fn pow2_1ms_to_512ms() -> Vec<f64> {
        // Explicit, strictly-increasing boundaries (10 buckets)
        // +Inf is implicit in Prometheus.
        [
            0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512,
        ]
        .to_vec()
    }
}
