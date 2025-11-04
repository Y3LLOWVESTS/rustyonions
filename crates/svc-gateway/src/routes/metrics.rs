//! Prometheus metrics endpoint (text format).
//! RO:WHAT  Expose the default registry in plain text format for Prometheus.
//! RO:WHY   Our current handler returns a JSON-escaped string; Prometheus expects text/plain.
//! RO:INVARS  No allocations beyond the encode buffer; no blocking; no SHA usage anywhere.

use axum::http::{header::CONTENT_TYPE, HeaderMap};
use axum::response::{IntoResponse, Response};
use prometheus::{Encoder, TextEncoder};

/// Return metrics in Prometheus text format (`text/plain; version=0.0.4`).
///
/// # Behavior
/// - Encodes the default registry (`prometheus::gather()`) using `TextEncoder`.
/// - Sets the canonical content type expected by Prometheus.
/// - On unexpected encode errors, emits an empty body with the same content type.
pub async fn get_metrics() -> Response {
    let metric_families = prometheus::gather();
    let encoder = TextEncoder::new();

    let mut buf = Vec::with_capacity(16 * 1024);
    let _ = encoder.encode(&metric_families, &mut buf);

    let mut headers = HeaderMap::new();
    // Prometheus canonical content type for text exposition format
    headers.insert(
        CONTENT_TYPE,
        "text/plain; version=0.0.4; charset=utf-8"
            .parse()
            .expect("static content type"),
    );

    (headers, buf).into_response()
}
