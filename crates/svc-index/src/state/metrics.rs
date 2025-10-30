//! RO:WHAT — Prometheus metrics registry and golden histograms.
//! RO:WHY  — Observability; consistent metric names.

use prometheus::{Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, Registry, TextEncoder};

pub struct Metrics {
    pub registry: Registry,
    pub http_requests_total: IntCounterVec,
    pub request_latency_seconds: Histogram,
    pub rejected_total: IntCounterVec,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();
        let http_requests_total = IntCounterVec::new(
            Opts::new("http_requests_total", "HTTP request count"),
            &["route", "method", "status"],
        )?;
        let request_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "request_latency_seconds",
            "Request latency",
        ))?;
        let rejected_total = IntCounterVec::new(
            Opts::new("rejected_total", "Rejected requests by reason"),
            &["reason"],
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(request_latency_seconds.clone()))?;
        registry.register(Box::new(rejected_total.clone()))?;
        Ok(Self {
            registry,
            http_requests_total,
            request_latency_seconds,
            rejected_total,
        })
    }

    pub fn render(&self) -> anyhow::Result<String> {
        let mut buf = Vec::new();
        let enc = TextEncoder::new();
        enc.encode(&self.registry.gather(), &mut buf)?;
        Ok(String::from_utf8(buf).unwrap_or_default())
    }
}
