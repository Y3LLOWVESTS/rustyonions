//! Dev-only routes (opt-in via env).
//! RO:CONF  `SVC_GATEWAY_DEV_ROUTES=1` to enable.
//! RO:NOTE  Guarded by body caps / rate limit in router assembly.

use axum::{
    body::{Body, Bytes},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Toggle for mounting dev routes.
#[must_use]
pub fn enabled() -> bool {
    std::env::var("SVC_GATEWAY_DEV_ROUTES")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

#[derive(Serialize)]
struct EchoResp<'a> {
    echo: &'a str,
    len: usize,
}

/// POST /dev/echo — echoes body length; body caps enforced by middleware.
pub async fn echo_post(body: Bytes) -> impl IntoResponse {
    let resp = EchoResp {
        echo: "hi",
        len: body.len(),
    };
    (StatusCode::OK, Json(resp))
}

/// GET /dev/rl — trivial “OK” for rate-limit tests (429 comes from middleware).
pub async fn burst_ok() -> Response {
    (StatusCode::OK, Body::from("ok")).into_response()
}
