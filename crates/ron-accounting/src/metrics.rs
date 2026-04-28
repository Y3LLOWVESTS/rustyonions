//! RO:WHAT — Prometheus metric handles for ron-accounting core operations.
//! RO:WHY — Pillar 12; Concerns: PERF/RES/GOV. Usage metering must be observable.
//! RO:INTERACTS — recorder, rollover, exporter, readiness, future HTTP /metrics adapter.
//! RO:INVARIANTS — register once per host registry; clone handles; no high-cardinality labels.
//! RO:METRICS — accounting_recorded_total, accounting_rows_current, accounting_slices_sealed_total.
//! RO:CONFIG — metrics feature gate; no runtime config in Batch 1.
//! RO:SECURITY — metrics must not expose PII; labels are normalized before recording.
//! RO:TEST — metrics contract tests in later batches.

#[cfg(feature = "metrics")]
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, IntGauge, Registry, TextEncoder};

use crate::errors::{Error, Result};

/// Metric handles. When `metrics` is disabled this is a zero-sized no-op handle.
#[derive(Debug, Clone)]
pub struct Metrics {
    #[cfg(feature = "metrics")]
    recorded_total: IntCounter,
    #[cfg(feature = "metrics")]
    rows_current: IntGauge,
    #[cfg(feature = "metrics")]
    slices_sealed_total: IntCounter,
    #[cfg(feature = "metrics")]
    export_fail_total: IntCounter,
    #[cfg(feature = "metrics")]
    export_latency_seconds: Histogram,
}

impl Metrics {
    /// Register metrics into a caller-owned registry.
    #[cfg(feature = "metrics")]
    pub fn new(registry: &Registry) -> Result<Self> {
        let recorded_total = IntCounter::new(
            "accounting_recorded_total",
            "Total usage increments accepted by ron-accounting.",
        )
        .map_err(|err| Error::other(err.to_string()))?;
        let rows_current = IntGauge::new(
            "accounting_rows_current",
            "Current distinct in-memory accounting rows.",
        )
        .map_err(|err| Error::other(err.to_string()))?;
        let slices_sealed_total = IntCounter::new(
            "accounting_slices_sealed_total",
            "Total sealed accounting slices.",
        )
        .map_err(|err| Error::other(err.to_string()))?;
        let export_fail_total = IntCounter::new(
            "accounting_export_fail_total",
            "Total accounting export failures.",
        )
        .map_err(|err| Error::other(err.to_string()))?;
        let export_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "accounting_export_latency_seconds",
            "Latency of accounting exporter put operations.",
        ))
        .map_err(|err| Error::other(err.to_string()))?;

        register(registry, recorded_total.clone())?;
        register(registry, rows_current.clone())?;
        register(registry, slices_sealed_total.clone())?;
        register(registry, export_fail_total.clone())?;
        register(registry, export_latency_seconds.clone())?;

        Ok(Self {
            recorded_total,
            rows_current,
            slices_sealed_total,
            export_fail_total,
            export_latency_seconds,
        })
    }

    /// Construct a no-op metrics handle when metrics are disabled.
    #[cfg(not(feature = "metrics"))]
    pub fn new_noop() -> Self {
        Self {}
    }

    /// Increment accepted usage records.
    pub fn inc_recorded(&self) {
        #[cfg(feature = "metrics")]
        self.recorded_total.inc();
    }

    /// Set current row count.
    pub fn set_rows_current(&self, rows: usize) {
        #[cfg(feature = "metrics")]
        self.rows_current.set(rows as i64);

        #[cfg(not(feature = "metrics"))]
        let _ = rows;
    }

    /// Increment sealed-slice count.
    pub fn inc_slices_sealed(&self) {
        #[cfg(feature = "metrics")]
        self.slices_sealed_total.inc();
    }

    /// Increment export failure count.
    pub fn inc_export_fail(&self) {
        #[cfg(feature = "metrics")]
        self.export_fail_total.inc();
    }

    /// Observe exporter latency in seconds.
    pub fn observe_export_latency(&self, seconds: f64) {
        #[cfg(feature = "metrics")]
        self.export_latency_seconds.observe(seconds);

        #[cfg(not(feature = "metrics"))]
        let _ = seconds;
    }

    /// Gather a registry into Prometheus text format.
    #[cfg(feature = "metrics")]
    pub fn gather_to_text(registry: &Registry) -> Result<String> {
        let families = registry.gather();
        let mut buf = Vec::new();
        TextEncoder::new()
            .encode(&families, &mut buf)
            .map_err(|err| Error::other(err.to_string()))?;
        String::from_utf8(buf).map_err(|err| Error::other(err.to_string()))
    }
}

#[cfg(feature = "metrics")]
fn register<C>(registry: &Registry, collector: C) -> Result<()>
where
    C: prometheus::core::Collector + Clone + 'static,
{
    registry
        .register(Box::new(collector))
        .map_err(|err| Error::other(err.to_string()))
}
