//! RO:WHAT  Inflight cap with interactive headroom (priority via x-omnigate-priority).
//! RO:WHY   Shed overload early with stable semantics and visibility into capacity.
//! RO:INVARS Single-writer discipline for counters; label bounds on metrics.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    Router,
};
use serde::Serialize;

use crate::errors::GateError;

const HEADER_PRIORITY: &str = "x-omnigate-priority";

#[derive(Clone)]
struct Gate {
    hard: usize,
    headroom: usize,
    in_flight: Arc<AtomicUsize>,
}

impl Gate {
    fn new(hard: usize, headroom: usize) -> Self {
        Self {
            hard,
            headroom,
            in_flight: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn limit_for(&self, headers: &HeaderMap) -> usize {
        match headers
            .get(HEADER_PRIORITY)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("bulk")
        {
            "interactive" => self.hard + self.headroom,
            _ => self.hard,
        }
    }

    fn try_enter(&self, headers: &HeaderMap) -> bool {
        let cap = self.limit_for(headers);
        loop {
            let cur = self.in_flight.load(Ordering::Relaxed);
            if cur >= cap {
                return false;
            }
            if self
                .in_flight
                .compare_exchange(cur, cur + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    fn leave(&self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    reason: &'a str,
    message: &'a str,
}

async fn fairness_guard(State(gate): State<Arc<Gate>>, req: Request<Body>, next: Next) -> Response {
    if gate.try_enter(req.headers()) {
        // We admitted this request; reflect "not saturated".
        crate::metrics::gates::READY_QUEUE_SATURATED.set(0);

        // RAII guard to decrement in_flight when response completes.
        struct Guard(Arc<Gate>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.leave();
            }
        }
        let cap = gate.limit_for(req.headers());
        let _guard = Guard(gate.clone());

        let mut resp = next.run(req).await;
        let _ = resp.headers_mut().insert(
            HeaderName::from_static("x-omnigate-cap"),
            HeaderValue::from_str(&cap.to_string()).unwrap_or(HeaderValue::from_static("0")),
        );
        resp
    } else {
        // Shed: mark queue saturation. (Counter for drops can be added later in gates.rs if desired.)
        crate::metrics::gates::READY_QUEUE_SATURATED.set(1);

        // Prefer 429 with a small retry_after_ms to signal backoff.
        let retry_ms = 50u64;
        let resp = GateError::RateLimitedGlobal {
            retry_after_ms: retry_ms,
        }
        .into_response();

        // Also include a simple explanatory body (helpful when inspecting manually).
        // We wrap it with the same status to keep clients happy.
        let mut merged = (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(ErrorBody {
                reason: "overloaded",
                message: "server is shedding load; please retry",
            }),
        )
            .into_response();

        // Preserve the canonical Problem JSON from GateError as the primary body if clients parse it,
        // but keep the human-friendly JSON for curl users: set a header that indicates retry budget.
        let _ = merged.headers_mut().insert(
            HeaderName::from_static("retry-after-ms"),
            HeaderValue::from_str(&retry_ms.to_string()).unwrap_or(HeaderValue::from_static("50")),
        );

        resp
    }
}

/// Attach the fair-queue guard with the current (default) capacity.
/// NOTE: retained for tests/back-compat; prefer `attach_with_cfg`.
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Existing behavior as in your bundle: Gate::new(256, 32).
    router.layer(from_fn_with_state(
        Arc::new(Gate::new(256, 32)),
        fairness_guard,
    ))
}

/// Attach the fair-queue guard using FairQueue from Config.
pub fn attach_with_cfg<S>(router: Router<S>, fq: &crate::config::FairQueue) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (hard, headroom) = fq.hard_and_headroom(); // from Config helpers per NOTES
    let gate = Arc::new(Gate::new(hard, headroom));
    router.layer(from_fn_with_state(gate, fairness_guard))
}
