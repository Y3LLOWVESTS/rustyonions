//! RO:WHAT — Prometheus exporter + metrics registry; mounts /metrics, /healthz, /readyz.
//! RO:WHY  — Observability pillar; RED metrics and kernel signals for ops; PERF/RES concerns.
//! RO:PERF — Optional thread-local metrics buffering (feature: `metrics_buf`) removes atomics from the hot path.
//! RO:MOG  — A1/A5 edge notify counters; A2 batch publish counters; TLS buffer flush counter.

use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get, Router};
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};
use tokio::{net::TcpListener, task::JoinHandle};

use crate::internal::types::{BoxError, ServiceName};
use crate::metrics::{health::HealthState, readiness::Readiness};
use crate::Bus;

#[cfg(feature = "metrics_buf")]
use crate::metrics::buffer::{BufferedSinks, FlushPump, HotCounters};

/// Kernel metrics registry and handles.
#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub request_latency_seconds: Histogram,

    pub service_restarts_total: IntCounterVec,

    // Bus counters/gauges
    pub bus_published_total: IntCounter,
    pub bus_no_receivers_total: IntCounter,
    pub bus_receiver_lag_total: IntCounter,
    pub bus_dropped_total: IntCounter,
    pub bus_topics_total: IntGauge,

    // MOG A1/A5 telemetry
    pub bus_notify_sends_total: IntCounter,
    pub bus_notify_suppressed_total: IntCounter,

    // MOG A2 telemetry
    pub bus_batch_publish_total: IntCounter,
    pub bus_batch_len_histogram: Histogram,

    // TLS metrics buffering visibility
    #[cfg(feature = "metrics_buf")]
    pub bus_metrics_tls_flush_total: IntCounter,

    // Expose configured TLS threshold as a gauge for easy verification
    #[cfg(feature = "metrics_buf")]
    pub bus_metrics_tls_threshold: IntGauge,

    pub amnesia_mode: IntGauge,

    // Hot-path counters facade (kept behind Arc to avoid per-call Drop)
    #[cfg(feature = "metrics_buf")]
    hot: Option<Arc<HotCounters>>,
}

impl Metrics {
    pub fn new(initial_amnesia: bool) -> Arc<Self> {
        let registry = Registry::new();

        let request_latency_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "request_latency_seconds",
                "Kernel request latency (seconds)",
            )
            .buckets(vec![
                0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
            ]),
        )
        .expect("histogram");

        let service_restarts_total = IntCounterVec::new(
            Opts::new(
                "service_restarts_total",
                "Total restarts of supervised services",
            ),
            &["service"],
        )
        .expect("counter vec");

        let bus_published_total = IntCounter::with_opts(Opts::new(
            "bus_published_total",
            "Total messages published on kernel bus",
        ))
        .unwrap();
        let bus_no_receivers_total = IntCounter::with_opts(Opts::new(
            "bus_no_receivers_total",
            "Publishes with zero receivers",
        ))
        .unwrap();
        let bus_receiver_lag_total = IntCounter::with_opts(Opts::new(
            "bus_receiver_lag_total",
            "Lagged/missed messages observed by receivers",
        ))
        .unwrap();
        let bus_dropped_total = IntCounter::with_opts(Opts::new(
            "bus_dropped_total",
            "Messages dropped due to closed/overrun channel",
        ))
        .unwrap();
        let bus_topics_total = IntGauge::with_opts(Opts::new(
            "bus_topics_total",
            "Number of distinct topic buses",
        ))
        .unwrap();

        // A1/A5
        let bus_notify_sends_total = IntCounter::with_opts(Opts::new(
            "bus_notify_sends_total",
            "Edge-triggered notifies sent to subscribers",
        ))
        .unwrap();
        let bus_notify_suppressed_total = IntCounter::with_opts(Opts::new(
            "bus_notify_suppressed_total",
            "Notifies suppressed by pending=true (coalesced)",
        ))
        .unwrap();

        // A2
        let bus_batch_publish_total = IntCounter::with_opts(Opts::new(
            "bus_batch_publish_total",
            "Calls to publish_many (A2)",
        ))
        .unwrap();
        let bus_batch_len_histogram = Histogram::with_opts(
            HistogramOpts::new("bus_batch_len_histogram", "publish_many batch sizes (A2)").buckets(
                vec![
                    1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0,
                ],
            ),
        )
        .expect("histogram");

        // Amnesia gauge
        let amnesia_mode =
            IntGauge::with_opts(Opts::new("amnesia_mode", "1 when amnesia mode is enabled"))
                .unwrap();

        // Register all
        registry
            .register(Box::new(request_latency_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(service_restarts_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_published_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_no_receivers_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_receiver_lag_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_dropped_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_topics_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_notify_sends_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_notify_suppressed_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_batch_publish_total.clone()))
            .unwrap();
        registry
            .register(Box::new(bus_batch_len_histogram.clone()))
            .unwrap();
        registry.register(Box::new(amnesia_mode.clone())).unwrap();

        // TLS buffering metrics
        #[cfg(feature = "metrics_buf")]
        let bus_metrics_tls_flush_total = {
            let c = IntCounter::with_opts(Opts::new(
                "bus_metrics_tls_flush_total",
                "TLS metrics buffer flushes (visibility when buffering is enabled)",
            ))
            .unwrap();
            registry.register(Box::new(c.clone())).ok();
            c
        };

        #[cfg(feature = "metrics_buf")]
        let bus_metrics_tls_threshold = {
            let g = IntGauge::with_opts(Opts::new(
                "bus_metrics_tls_threshold",
                "Configured TLS flush threshold",
            ))
            .unwrap();
            registry.register(Box::new(g.clone())).ok();
            g
        };

        // Choose and publish the threshold; construct sinks + hot facade.
        #[cfg(feature = "metrics_buf")]
        let (hot, chosen_threshold) = {
            let threshold: usize = 64; // tune under benches (64..512)
            let sinks = BufferedSinks::new(
                bus_published_total.clone(),
                bus_notify_sends_total.clone(),
                bus_metrics_tls_flush_total.clone(),
                threshold,
            );
            (Some(Arc::new(HotCounters::new(sinks))), threshold)
        };

        #[cfg(feature = "metrics_buf")]
        {
            bus_metrics_tls_threshold.set(chosen_threshold as i64);
            tracing::info!(
                threshold = chosen_threshold,
                "metrics_buf enabled; TLS flush threshold configured"
            );
        }

        let me = Arc::new(Self {
            registry,
            request_latency_seconds,
            service_restarts_total,
            bus_published_total,
            bus_no_receivers_total,
            bus_receiver_lag_total,
            bus_dropped_total,
            bus_topics_total,
            bus_notify_sends_total,
            bus_notify_suppressed_total,
            bus_batch_publish_total,
            bus_batch_len_histogram,
            #[cfg(feature = "metrics_buf")]
            bus_metrics_tls_flush_total,
            #[cfg(feature = "metrics_buf")]
            bus_metrics_tls_threshold,
            amnesia_mode,
            #[cfg(feature = "metrics_buf")]
            hot,
        });

        me.set_amnesia(initial_amnesia);
        me
    }

    pub fn set_amnesia(&self, on: bool) {
        self.amnesia_mode.set(if on { 1 } else { 0 });
    }

    /// Start HTTP server exposing /metrics, /healthz, /readyz.
    pub async fn serve(
        self: Arc<Self>,
        addr: SocketAddr,
        health: HealthState,
        ready: Readiness,
    ) -> Result<(JoinHandle<()>, SocketAddr), BoxError> {
        let listener = TcpListener::bind(addr).await?;
        let local = listener.local_addr()?;

        let registry = self.registry.clone();
        let app = Router::new()
            .route(
                "/metrics",
                get(move || {
                    let registry = registry.clone();
                    async move {
                        let mf = registry.gather();
                        let mut buf = Vec::new();
                        TextEncoder::new().encode(&mf, &mut buf).unwrap();
                        (axum::http::StatusCode::OK, buf)
                    }
                }),
            )
            .route(
                "/healthz",
                get({
                    let health = health.clone();
                    move || crate::metrics::health::healthz_handler(health.clone())
                }),
            )
            .route(
                "/readyz",
                get({
                    let ready = ready.clone();
                    move || crate::metrics::readiness::readyz_handler(ready.clone())
                }),
            );

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        #[cfg(feature = "metrics_buf")]
        if let Some(hot) = self.hot.clone() {
            let pump = FlushPump::new_from_hot(hot);
            tokio::spawn(async move { pump.run(200).await }); // ~200ms cadence
        }

        Ok((handle, local))
    }

    pub fn inc_restart(&self, service: ServiceName) {
        self.service_restarts_total
            .with_label_values(&[service])
            .inc();
    }

    pub fn make_bus<T: Clone + Send + 'static>(self: &Arc<Self>, capacity: usize) -> Bus<T> {
        use crate::bus::bounded::Bus;
        Bus::with_capacity(capacity).with_metrics(self.clone())
    }

    #[cfg(feature = "metrics_buf")]
    #[inline]
    pub fn hot(&self) -> Option<&HotCounters> {
        self.hot.as_deref()
    }
}
