//! RO:WHAT — Prometheus metrics for svc-storage.

use once_cell::sync::Lazy;
use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};

pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

pub static REQUEST_LATENCY_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let o = HistogramOpts::new("storage_request_latency_seconds", "HTTP request latency");
    let h = Histogram::with_opts(o).unwrap();
    REGISTRY.register(Box::new(h.clone())).ok();
    h
});

pub static REJECTED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let o = Opts::new("storage_rejected_total", "Rejected requests by reason");
    let c = IntCounterVec::new(o, &["reason"]).unwrap();
    REGISTRY.register(Box::new(c.clone())).ok();
    c
});

pub static INTEGRITY_FAIL_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let o = Opts::new("storage_integrity_fail_total", "Integrity check failures");
    let c = IntCounterVec::new(o, &["path"]).unwrap();
    REGISTRY.register(Box::new(c.clone())).ok();
    c
});
