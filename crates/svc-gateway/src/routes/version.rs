//! Build/version endpoint (no SHA; respects no-SHA policy).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub async fn handler() -> Response {
    let version = env!("CARGO_PKG_VERSION");
    let built_at = option_env!("SVC_GATEWAY_BUILD_TS").unwrap_or("0");
    let obj = serde_json::json!({
        "name": "svc-gateway",
        "version": version,
        "built_at_unix": built_at,
    });
    (StatusCode::OK, Json(obj)).into_response()
}
