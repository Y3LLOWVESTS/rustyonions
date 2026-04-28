// crates/micronode/src/observability/metrics.rs
//! RO:WHAT — Micronode observability helpers that match svc-admin expectations.
//! RO:WHY  — svc-admin expects `ron_facet_requests_total{facet,result}` for facet freshness,
//!           plus a couple of simple node gauges.
//! RO:NOTE — These metrics are registered into the ron-kernel Metrics registry (not the default registry).

use prometheus::{Error as PromError, Gauge, IntCounterVec, Opts, Registry};
use std::sync::OnceLock;
use std::time::Duration;

const PROM_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

#[derive(Clone)]
struct Obs {
    facet_requests_total: IntCounterVec,
    node_uptime_seconds: Gauge,
    node_ready: Gauge,
}

static OBS: OnceLock<Obs> = OnceLock::new();

fn ignore_already_reg(e: PromError) {
    // prometheus::Error has an AlreadyReg variant; treat it as harmless for dev reloads/tests.
    // Anything else is worth surfacing via logs later (we keep it silent here to stay dependency-light).
    let _ = e;
}

pub fn prometheus_content_type() -> &'static str {
    PROM_CONTENT_TYPE
}

/// Register micronode-specific metrics into the provided Registry (ron-kernel Metrics registry).
pub fn init(registry: &Registry) {
    if OBS.get().is_some() {
        return;
    }

    let facet_requests_total = IntCounterVec::new(
        Opts::new("ron_facet_requests_total", "Total requests per facet (svc-admin expects this)."),
        &["facet", "result"],
    )
    .expect("ron_facet_requests_total must construct");

    let node_uptime_seconds =
        Gauge::with_opts(Opts::new("ron_node_uptime_seconds", "Node process uptime in seconds."))
            .expect("ron_node_uptime_seconds must construct");

    let node_ready =
        Gauge::with_opts(Opts::new("ron_node_ready", "Node readiness (1=ready, 0=not ready)."))
            .expect("ron_node_ready must construct");

    // Register into the provided registry.
    if let Err(e) = registry.register(Box::new(facet_requests_total.clone())) {
        ignore_already_reg(e);
    }
    if let Err(e) = registry.register(Box::new(node_uptime_seconds.clone())) {
        ignore_already_reg(e);
    }
    if let Err(e) = registry.register(Box::new(node_ready.clone())) {
        ignore_already_reg(e);
    }

    let obs = Obs { facet_requests_total, node_uptime_seconds, node_ready };

    // Set OnceLock (ignore if someone raced us).
    let _ = OBS.set(obs);

    // Prewarm common facets so they appear even before traffic.
    prewarm_facet("admin.healthz");
    prewarm_facet("admin.readyz");
    prewarm_facet("admin.version");
    prewarm_facet("admin.metrics");
    prewarm_facet("admin.status");
    prewarm_facet("admin.system_summary");
    prewarm_facet("admin.storage_summary");
}

fn prewarm_facet(facet: &str) {
    if let Some(o) = OBS.get() {
        // Inc-by-zero pattern (Prometheus client will still materialize the series after with_label_values).
        let c_ok = o.facet_requests_total.with_label_values(&[facet, "ok"]);
        c_ok.inc_by(0);
        let c_err = o.facet_requests_total.with_label_values(&[facet, "err"]);
        c_err.inc_by(0);
    }
}

pub fn observe_facet_ok(facet: &str) {
    if let Some(o) = OBS.get() {
        o.facet_requests_total.with_label_values(&[facet, "ok"]).inc();
    }
}

pub fn observe_facet_err(facet: &str) {
    if let Some(o) = OBS.get() {
        o.facet_requests_total.with_label_values(&[facet, "err"]).inc();
    }
}

pub fn update_micronode_metrics(uptime: Duration, ready: bool) {
    if let Some(o) = OBS.get() {
        o.node_uptime_seconds.set(uptime.as_secs_f64());
        o.node_ready.set(if ready { 1.0 } else { 0.0 });
    }
}
