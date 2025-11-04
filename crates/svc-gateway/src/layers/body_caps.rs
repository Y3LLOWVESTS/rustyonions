//! Body-size cap (route-scoped).
//! RO:WHAT  Reject requests with `Content-Length` exceeding a configured cap.
//! RO:WHY   Cheap protection against oversized uploads; observable via metrics.
//! RO:CONF  `SVC_GATEWAY_MAX_BODY_BYTES` (default 1 MiB). Header-only (no streaming).
//! RO:OBS   Increments `gateway_rejections_total{reason="body_cap"}` on reject.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware: if `Content-Length` is present and exceeds the cap, reject with 413.
///
/// This version is header-only. If `Content-Length` is absent or invalid, we let the request
/// pass through; a streaming cap (for unknown/incorrect lengths) can be added later.
pub async fn body_cap_mw(req: Request<Body>, next: Next) -> Response {
    let cap = std::env::var("SVC_GATEWAY_MAX_BODY_BYTES")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_048_576); // 1 MiB default

    if let Some(len) = req.headers().get(axum::http::header::CONTENT_LENGTH) {
        if let Ok(len_str) = len.to_str() {
            if let Ok(n) = len_str.parse::<u64>() {
                if n > cap {
                    // Reuse the shared rejects counter to avoid AlreadyReg panics.
                    crate::observability::rejects::counter()
                        .with_label_values(&["body_cap"])
                        .inc();
                    return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response();
                }
            }
        }
    }

    next.run(req).await
}
