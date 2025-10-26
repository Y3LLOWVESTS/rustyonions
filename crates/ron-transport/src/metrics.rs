//! RO:WHAT — Prometheus counters/histograms for transport.
//! RO:WHY  — Golden metrics surface; avoid duplicate registers.

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry};

#[derive(Clone)]
pub struct TransportMetrics {
    pub registry: Registry,
    pub connections: IntCounterVec,
    pub bytes_in: IntCounterVec,
    pub bytes_out: IntCounterVec,
    pub rejected_total: IntCounterVec,
    pub latency_seconds: HistogramVec,
}

impl TransportMetrics {
    pub fn new(namespace: &str) -> Self {
        let registry = Registry::new();
        let connections = IntCounterVec::new(
            Opts::new("transport_connections_total", "Accepted connections").namespace(namespace),
            &["name"],
        )
        .unwrap();
        let bytes_in = IntCounterVec::new(
            Opts::new("transport_bytes_in_total", "Bytes received").namespace(namespace),
            &["name"],
        )
        .unwrap();
        let bytes_out = IntCounterVec::new(
            Opts::new("transport_bytes_out_total", "Bytes sent").namespace(namespace),
            &["name"],
        )
        .unwrap();
        let rejected_total = IntCounterVec::new(
            Opts::new("transport_rejected_total", "Rejected connections/frames")
                .namespace(namespace),
            &["name", "reason"],
        )
        .unwrap();
        let latency_seconds = HistogramVec::new(
            HistogramOpts::new("transport_latency_seconds", "End-to-end per-conn lifetime")
                .namespace(namespace),
            &["name"],
        )
        .unwrap();

        registry.register(Box::new(connections.clone())).ok();
        registry.register(Box::new(bytes_in.clone())).ok();
        registry.register(Box::new(bytes_out.clone())).ok();
        registry.register(Box::new(rejected_total.clone())).ok();
        registry.register(Box::new(latency_seconds.clone())).ok();

        Self {
            registry,
            connections,
            bytes_in,
            bytes_out,
            rejected_total,
            latency_seconds,
        }
    }
}
