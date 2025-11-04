//! RO:WHAT  /healthz — liveness probe.
//! RO:WHY   Simple process up check. Always 200 "ok".

use axum::response::{IntoResponse, Response};

pub async fn handler() -> Response {
    "ok".into_response()
}
