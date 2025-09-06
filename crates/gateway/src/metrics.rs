// crates/gateway/src/metrics.rs
#![forbid(unsafe_code)]

//! Golden Prometheus metrics for the Gateway.
//!
//! Mount with:
//!   .route("/metrics", get(crate::metrics::metrics_handler))
//!   .layer(middleware::from_fn(crate::metrics::record_metrics))

use std::sync::OnceLock;
use std::time::Instant;

use axum::{
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
};
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry, TextEncoder};

struct GatewayMetrics {
    requests_total: IntCounterVec,              // labels: code
    bytes_out_total: IntCounter,
    request_latency_seconds: Histogram,
    cache_hits_total: IntCounter,               // 304s
    range_requests_total: IntCounter,           // 206s
    precompressed_served_total: IntCounterVec,  // labels: encoding
    quota_rejections_total: IntCounter,         // 429s
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();
fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::new)
}

static METRICS: OnceLock<GatewayMetrics> = OnceLock::new();
fn metrics() -> &'static GatewayMetrics {
    METRICS.get_or_init(|| {
        let r = registry();

        let requests_total = IntCounterVec::new(
            Opts::new("requests_total", "Total HTTP requests by status code"),
            &["code"],
        )
        .expect("requests_total");

        let bytes_out_total =
            IntCounter::with_opts(Opts::new("bytes_out_total", "Total response bytes (Content-Length)"))
                .expect("bytes_out_total");

        let request_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "request_latency_seconds",
            "Wall time from request to response",
        ))
        .expect("request_latency_seconds");

        let cache_hits_total =
            IntCounter::with_opts(Opts::new("cache_hits_total", "Conditional GET hits (304 Not Modified)"))
                .expect("cache_hits_total");

        let range_requests_total =
            IntCounter::with_opts(Opts::new("range_requests_total", "Byte-range responses (206)"))
                .expect("range_requests_total");

        let precompressed_served_total = IntCounterVec::new(
            Opts::new("precompressed_served_total", "Objects served from precompressed variants"),
            &["encoding"],
        )
        .expect("precompressed_served_total");

        let quota_rejections_total = IntCounter::with_opts(Opts::new(
            "quota_rejections_total",
            "Requests rejected due to quotas/overload (429)",
        ))
        .expect("quota_rejections_total");

        // Dedicated Registry to avoid accidental double-registrations from other crates.
        r.register(Box::new(requests_total.clone())).ok();
        r.register(Box::new(bytes_out_total.clone())).ok();
        r.register(Box::new(request_latency_seconds.clone())).ok();
        r.register(Box::new(cache_hits_total.clone())).ok();
        r.register(Box::new(range_requests_total.clone())).ok();
        r.register(Box::new(precompressed_served_total.clone())).ok();
        r.register(Box::new(quota_rejections_total.clone())).ok();

        GatewayMetrics {
            requests_total,
            bytes_out_total,
            request_latency_seconds,
            cache_hits_total,
            range_requests_total,
            precompressed_served_total,
            quota_rejections_total,
        }
    })
}

/// GET /metrics
pub async fn metrics_handler() -> impl IntoResponse {
    let mut buf = Vec::new();
    let enc = TextEncoder::new();
    if let Err(e) = enc.encode(&registry().gather(), &mut buf) {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("metrics encode error: {e}"),
        )
            .into_response();
    }
    let mut headers = HeaderMap::new();
    // prometheus::TextEncoder doesn't expose TEXT_FORMAT in this version.
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    (headers, buf).into_response()
}

/// Middleware that records request count, latency, and a best-effort byte count.
/// axum 0.7: Next has no generic; accept concrete Request<Body>.
pub async fn record_metrics(
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let mut resp = next.run(req).await;

    // Count by status code
    let code = resp.status().as_u16().to_string();
    metrics().requests_total.with_label_values(&[&code]).inc();

    // Latency
    metrics()
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    // Bytes (Content-Length only; if missing, skip)
    if let Some(len) = content_length(&resp) {
        metrics().bytes_out_total.inc_by(len as u64);
    }

    // Specialized counters
    match resp.status().as_u16() {
        206 => metrics().range_requests_total.inc(),
        304 => metrics().cache_hits_total.inc(),
        429 => metrics().quota_rejections_total.inc(),
        _ => {}
    }

    if let Some(enc) = content_encoding(&resp) {
        metrics()
            .precompressed_served_total
            .with_label_values(&[enc])
            .inc();
    }

    resp
}

fn content_length(resp: &Response) -> Option<u64> {
    resp.headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

fn content_encoding(resp: &Response) -> Option<&'static str> {
    resp.headers()
        .get(axum::http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| match s {
            "br" => Some("br"),
            "zstd" | "zst" => Some("zst"),
            "gzip" => Some("gzip"),
            "identity" => Some("identity"),
            _ => None,
        })
}

/// Optional helpers for handlers:
pub fn bump_precompressed_served(encoding: &str) {
    let enc = match encoding {
        "br" => "br",
        "zstd" | "zst" => "zst",
        "gzip" => "gzip",
        _ => "identity",
    };
    metrics()
        .precompressed_served_total
        .with_label_values(&[enc])
        .inc();
}
pub fn bump_cache_hit() {
    metrics().cache_hits_total.inc();
}
pub fn bump_quota_reject() {
    metrics().quota_rejections_total.inc();
}
