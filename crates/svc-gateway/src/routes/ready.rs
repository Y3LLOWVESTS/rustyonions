//! Readiness endpoint.
//! RO:WHAT   Truthful readiness gate (env override remains for dev).
//! RO:WHY    Operators need a real signal; keep override for quick local bring-up.
//! RO:TEST   Set `SVC_GATEWAY_READY_SLEEP_MS` to simulate slow work and exercise
//!           the concurrency cap + timeout layers.

use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::time::Duration;
use tokio::time::sleep;

/// `/readyz` handler consulting sampler thresholds, with optional sleep to
/// simulate slow checks and exercise guards.
///
/// # Errors
/// This function does not fail; it always returns a `Response`.
pub async fn handler(State(_state): State<AppState>) -> Response {
    // Optional: simulate slow work to demonstrate concurrency/timeout guards.
    if let Ok(ms_str) = std::env::var("SVC_GATEWAY_READY_SLEEP_MS") {
        if let Ok(ms) = ms_str.parse::<u64>() {
            sleep(Duration::from_millis(ms)).await;
        }
    }

    // Dev override wins if explicitly set.
    if matches!(std::env::var("SVC_GATEWAY_DEV_READY").as_deref(), Ok("1")) {
        return (StatusCode::OK, "ready").into_response();
    }

    // Truth table based on the sampler snapshot.
    let snap = crate::observability::readiness::snapshot();
    let thr = crate::observability::readiness::Thresholds::default();

    let inflight_ok = snap.inflight_current <= thr.max_inflight;
    let error_ok = snap.error_rate_pct <= thr.max_error_pct;
    let queue_ok = thr.allow_queue_saturation || !snap.queue_saturated;

    if inflight_ok && error_ok && queue_ok {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}
