// crates/gateway/src/metrics.rs
#![forbid(unsafe_code)]

use std::sync::OnceLock;
use std::time::Instant;

use axum::{
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
};
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry, TextEncoder,
};

struct GatewayMetrics {
    // Store Option<T> so we can avoid unwrap/expect and gracefully no-op if construction fails.
    requests_total: Option<IntCounterVec>, // labels: code
    bytes_out_total: Option<IntCounter>,
    request_latency_seconds: Option<Histogram>,
    cache_hits_total: Option<IntCounter>,              // 304s
    range_requests_total: Option<IntCounter>,          // 206s
    precompressed_served_total: Option<IntCounterVec>, // labels: encoding
    quota_rejections_total: Option<IntCounter>,        // 429s
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
        .ok();
        if let Some(m) = &requests_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let bytes_out_total = IntCounter::with_opts(Opts::new(
            "bytes_out_total",
            "Total response bytes (Content-Length)",
        ))
        .ok();
        if let Some(m) = &bytes_out_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let request_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "request_latency_seconds",
            "Wall time from request to response",
        ))
        .ok();
        if let Some(m) = &request_latency_seconds {
            let _ = r.register(Box::new(m.clone()));
        }

        let cache_hits_total = IntCounter::with_opts(Opts::new(
            "cache_hits_total",
            "Conditional GET hits (304 Not Modified)",
        ))
        .ok();
        if let Some(m) = &cache_hits_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let range_requests_total = IntCounter::with_opts(Opts::new(
            "range_requests_total",
            "Byte-range responses (206)",
        ))
        .ok();
        if let Some(m) = &range_requests_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let precompressed_served_total = IntCounterVec::new(
            Opts::new(
                "precompressed_served_total",
                "Objects served from precompressed variants",
            ),
            &["encoding"],
        )
        .ok();
        if let Some(m) = &precompressed_served_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let quota_rejections_total = IntCounter::with_opts(Opts::new(
            "quota_rejections_total",
            "Requests rejected due to quotas/overload (429)",
        ))
        .ok();
        if let Some(m) = &quota_rejections_total {
            let _ = r.register(Box::new(m.clone()));
        }

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
    // If encoding fails, return 500 with a message.
    if enc.encode(&registry().gather(), &mut buf).is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "metrics encode error",
        )
            .into_response();
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    (headers, buf).into_response()
}

/// Middleware that records request count, latency, and a best-effort byte count.
pub async fn record_metrics(req: axum::http::Request<axum::body::Body>, next: Next) -> Response {
    let start = Instant::now();
    let resp = next.run(req).await;

    // Count by status code
    if let Some(m) = &metrics().requests_total {
        let code = resp.status().as_u16().to_string();
        m.with_label_values(&[&code]).inc();
    }

    // Latency
    if let Some(h) = &metrics().request_latency_seconds {
        h.observe(start.elapsed().as_secs_f64());
    }

    // Bytes (Content-Length only; if missing, skip)
    if let (Some(m), Some(len)) = (&metrics().bytes_out_total, content_length(&resp)) {
        m.inc_by(len);
    }

    // Specialized counters (best-effort)
    match resp.status().as_u16() {
        206 => {
            if let Some(m) = &metrics().range_requests_total {
                m.inc();
            }
        }
        304 => {
            if let Some(m) = &metrics().cache_hits_total {
                m.inc();
            }
        }
        429 => {
            if let Some(m) = &metrics().quota_rejections_total {
                m.inc();
            }
        }
        _ => {}
    }

    if let Some(enc) = content_encoding(&resp) {
        if let Some(m) = &metrics().precompressed_served_total {
            m.with_label_values(&[enc]).inc();
        }
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
#[allow(dead_code)]
pub fn bump_precompressed_served(encoding: &str) {
    let enc = match encoding {
        "br" => "br",
        "zstd" | "zst" => "zst",
        "gzip" => "gzip",
        _ => "identity",
    };
    if let Some(m) = &metrics().precompressed_served_total {
        m.with_label_values(&[enc]).inc();
    }
}

#[allow(dead_code)]
pub fn bump_cache_hit() {
    if let Some(m) = &metrics().cache_hits_total {
        m.inc();
    }
}

#[allow(dead_code)]
pub fn bump_quota_reject() {
    if let Some(m) = &metrics().quota_rejections_total {
        m.inc();
    }
}
