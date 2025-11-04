//! HTTP metrics middleware (route-scoped).
//! RO:WHAT  Count requests and record latency with Prometheus labels.
//! RO:WHY   Operators get per-route visibility without global layering.
//! RO:NOTE  Axum 0.7: `Next` has no generics; `Request` is `Request<Body>`.
//! RO:HASH  No SHA anywhere. This middleware does not hash data.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts};
use std::{sync::OnceLock, time::Instant};

/// Use distinct, gateway-scoped metric names to avoid any collision with
/// central registries that may already expose similarly named series.
/// (We can alias/rename later when official getters are exposed.)
const REQS_NAME: &str = "gateway_http_requests_total";
const LAT_NAME: &str = "gateway_request_latency_seconds";

fn http_counters() -> &'static IntCounterVec {
    static CTR: OnceLock<IntCounterVec> = OnceLock::new();
    CTR.get_or_init(|| {
        let vec = IntCounterVec::new(
            Opts::new(REQS_NAME, "Total HTTP requests (svc-gateway middleware)"),
            &["route", "method", "status"],
        )
        .expect("IntCounterVec");
        // Register once; OnceLock ensures we don't double-register from this module.
        prometheus::register(Box::new(vec.clone())).expect("register gateway_http_requests_total");
        vec
    })
}

fn http_latency() -> &'static HistogramVec {
    static HIST: OnceLock<HistogramVec> = OnceLock::new();
    HIST.get_or_init(|| {
        let opts = HistogramOpts::new(
            LAT_NAME,
            "Request latency in seconds (svc-gateway middleware)",
        )
        .buckets(vec![
            0.000_5, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0,
        ]);
        let vec = HistogramVec::new(opts, &["route", "method"]).expect("HistogramVec");
        prometheus::register(Box::new(vec.clone()))
            .expect("register gateway_request_latency_seconds");
        vec
    })
}

/// Axum 0.7 middleware entry point.
/// Mounted (for now) only on `/healthz`.
///
/// # Behavior
/// - Increments `{route,method,status}`
/// - Observes latency `{route,method}`
pub async fn mw(req: Request<Body>, next: Next) -> Response {
    // Static route label since we mount this only on /healthz for now.
    let route_label = "healthz";
    let method_label = req.method().as_str().to_ascii_uppercase();

    let start = Instant::now();
    let resp = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();

    let status_label = resp.status().as_u16().to_string();

    http_counters()
        .with_label_values(&[route_label, &method_label, &status_label])
        .inc();

    http_latency()
        .with_label_values(&[route_label, &method_label])
        .observe(elapsed);

    resp
}
