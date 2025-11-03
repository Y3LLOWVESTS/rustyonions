//! RO:WHAT  Readiness module: policy (truth), admin state, /readyz, and samplers.
//! RO:WHY   Keep lib.rs slim; keep readiness logic cohesive and testable.

pub mod policy;
pub mod sampler;
pub mod state;

use axum::{extract::State, response::IntoResponse};
use std::time::{Duration, Instant};

use crate::errors::http_map::GateError;
use crate::metrics::gates::{READY_STATE_CHANGES_TOTAL, READY_TRIPS_TOTAL};
use state::AdminState;

/// /readyz handler: consults local ReadyPolicy + sticky hold, else delegates to kernel.
pub async fn readyz(State(st): State<AdminState>) -> impl IntoResponse {
    if st.dev_ready {
        return (axum::http::StatusCode::OK, "ready (dev override)").into_response();
    }

    // Honor the hold window if previously tripped.
    let now = Instant::now();
    {
        // Take a copy of the Option<Instant>, since Instant is Copy.
        let mut guard = st.hold_until_lock();
        if let Some(until) = *guard {
            if now < until {
                return GateError::Degraded.into_response();
            } else {
                // Hold expired — clear and mark recovery.
                *guard = None;
                READY_STATE_CHANGES_TOTAL
                    .with_label_values(&["ready"])
                    .inc();
            }
        }
    }

    // Sample current policy state.
    let inflight = st.rp.inflight();
    let err_pct_like = st.rp.err_rate_pct();

    // Trip if either threshold is exceeded.
    let trip_inflight = inflight > st.max_inflight_threshold;
    let trip_err = err_pct_like >= st.error_rate_429_503_pct;

    if trip_inflight || trip_err {
        // Start/extend hold window.
        st.set_hold_until(now + Duration::from_secs(st.hold_for_secs.max(1)));

        // Mark trip metadata.
        let reason = if trip_inflight {
            "inflight"
        } else {
            "err_rate"
        };
        READY_TRIPS_TOTAL.with_label_values(&[reason]).inc();
        READY_STATE_CHANGES_TOTAL
            .with_label_values(&["degraded"])
            .inc();

        return GateError::Degraded.into_response();
    }

    // Otherwise delegate to kernel readiness handler (200 path).
    ron_kernel::metrics::readiness::readyz_handler(st.ready.clone()).await
}
