//! RO:WHAT  Global token-bucket admission guard that returns 429 when over limit.
//! RO:WHY   Prevents overload by bounding request rate upfront.
//! RO:INVARS Constant-time hot path; expose exhaust events via metrics.

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    Router,
};
use serde::Serialize;

#[derive(Clone)]
struct GlobalLimiter {
    inner: Arc<Mutex<Bucket>>,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last: Instant,
    rate_per_sec: f64,
    burst: f64,
}

impl GlobalLimiter {
    fn new(rate_per_sec: u32, burst: u32) -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(Mutex::new(Bucket {
                tokens: burst as f64,
                last: now,
                rate_per_sec: rate_per_sec as f64,
                burst: burst as f64,
            })),
        }
    }

    #[inline]
    fn allow(&self) -> bool {
        let mut b = self.inner.lock().expect("limiter poisoned");
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(b.last);
        b.last = now;

        let refill = elapsed.as_secs_f64() * b.rate_per_sec;
        b.tokens = (b.tokens + refill).min(b.burst);

        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    reason: &'a str,
    message: &'a str,
}

async fn quota_guard(
    State(state): State<GlobalLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if state.allow() {
        next.run(req).await
    } else {
        // Count quota exhausts for observability and readiness error-window calculations.
        // Metric defined in metrics module with label contract {scope = global|ip|token}.
        crate::metrics::ADMISSION_QUOTA_EXHAUSTED_TOTAL
            .with_label_values(&["global"])
            .inc();

        (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(ErrorBody {
                reason: "too_many_requests",
                message: "request rate exceeds configured limit",
            }),
        )
            .into_response()
    }
}

/// Attach the quota limiter layer to the Router.
/// Add `Sync` so Axum can apply the stateful layer.
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // TODO: drive from Config (admission.global_quota.{qps,burst}).
    let limiter = GlobalLimiter::new(500, 1000);
    router.layer(from_fn_with_state(limiter, quota_guard))
}
