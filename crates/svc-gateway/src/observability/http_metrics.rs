//! HTTP metrics wiring + tiny middleware.
//! RO:WHAT   Record request totals and latency buckets with stable labels.
//! RO:WHY    Golden counters for SREs; cheap + predictable.
//! RO:LABELS route,method,status (counter) and route,method (histogram).
//! RO:SAFETY Registered once via `OnceCell`; no panics after success.
//! RO:NOTE   `prewarm()` creates child series so dashboards light up immediately.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use once_cell::sync::OnceCell;
use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts};
use std::time::Instant;

static HTTP_REQS: OnceCell<IntCounterVec> = OnceCell::new();
static LAT_HIST: OnceCell<HistogramVec> = OnceCell::new();

fn reqs() -> &'static IntCounterVec {
    HTTP_REQS.get_or_init(|| {
        let vec = IntCounterVec::new(
            Opts::new(
                "gateway_http_requests_total",
                "Total HTTP requests (svc-gateway middleware)",
            ),
            &["route", "method", "status"],
        )
        .expect("IntCounterVec");
        prometheus::register(Box::new(vec.clone())).expect("register gateway_http_requests_total");
        vec
    })
}

fn lats() -> &'static HistogramVec {
    LAT_HIST.get_or_init(|| {
        // Buckets chosen to match docs (ms in seconds representation).
        let buckets = vec![
            0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0,
        ];
        let opts = HistogramOpts::new(
            "gateway_request_latency_seconds",
            "Request latency in seconds (svc-gateway middleware)",
        )
        .buckets(buckets);
        let vec = HistogramVec::new(opts, &["route", "method"]).expect("HistogramVec");
        prometheus::register(Box::new(vec.clone()))
            .expect("register gateway_request_latency_seconds");
        vec
    })
}

/// Derive a compact, low-cardinality route label from the path.
/// We keep it stable for core endpoints; everything else falls back
/// to the first segment (or "root").
fn route_label(path: &str) -> &'static str {
    match path {
        "/healthz" => "healthz",
        "/readyz" => "readyz",
        "/metrics" => "metrics",
        "/version" => "version",
        "/dev/echo" => "dev_echo",
        "/dev/rl" => "dev_rl",
        _ => {
            if path == "/" {
                "root"
            } else {
                "other"
            }
        }
    }
}

/// Middleware: measure latency + count by labels.
/// Apply at route scope where appropriate.
///
/// # Errors
/// Never returns an error directly; upstream handler may.
pub async fn mw(req: Request<Body>, next: Next) -> Response {
    // Compute labels BEFORE moving `req` into `next.run(...)`.
    let route = route_label(req.uri().path());
    let method_owned = req.method().as_str().to_owned();

    let start = Instant::now();
    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();
    let secs = start.elapsed().as_secs_f64();

    reqs()
        .with_label_values(&[route, method_owned.as_str(), &status])
        .inc();
    lats()
        .with_label_values(&[route, method_owned.as_str()])
        .observe(secs);

    response
}

/// Pre-create common label series so dashboards don’t start “empty”.
/// Call this once during startup before serving traffic.
pub fn prewarm() {
    // Counters (route, method, status)
    for (route, method, statuses) in [
        ("healthz", "GET", &["200"][..]),
        ("readyz", "GET", &["200", "503"][..]),
        ("metrics", "GET", &["200"][..]),
        ("version", "GET", &["200"][..]),
        ("dev_echo", "POST", &["200", "413"][..]),
        ("dev_rl", "GET", &["200", "429"][..]),
    ] {
        for &st in statuses {
            let _ = reqs().get_metric_with_label_values(&[route, method, st]);
        }
        let _ = lats().get_metric_with_label_values(&[route, method]);
    }
}
