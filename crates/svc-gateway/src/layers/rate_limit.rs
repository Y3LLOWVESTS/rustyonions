//! Token-bucket rate limit (route-scoped) returning 429 when empty.
//! RO:CONF  `SVC_GATEWAY_RL_RPS` (default 5), `SVC_GATEWAY_RL_BURST` (default 10).
//! RO:OBS   Increments `gateway_rejections_total{reason="rate_limit"}`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::OnceCell;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

struct Bucket {
    last: Instant,
    tokens: f64,
    rps: f64,
    burst: f64,
}

impl Bucket {
    fn new(rps: f64, burst: f64) -> Self {
        Self { last: Instant::now(), tokens: burst, rps, burst }
    }
    fn take(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;

        // Refill
        self.tokens += self.rps * dur_secs(elapsed);
        if self.tokens > self.burst {
            self.tokens = self.burst;
        }

        // Consume
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

fn dur_secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

fn bucket() -> MutexGuard<'static, Bucket> {
    static BKT: OnceCell<Mutex<Bucket>> = OnceCell::new();
    let rps = std::env::var("SVC_GATEWAY_RL_RPS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(5.0);
    let burst = std::env::var("SVC_GATEWAY_RL_BURST")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(10.0);
    BKT.get_or_init(|| Mutex::new(Bucket::new(rps, burst)))
        .lock()
        .expect("bucket lock")
}

pub async fn rate_limit_mw(req: Request<Body>, next: Next) -> Response {
    // Fast decision under a small lock; no async work in the critical section.
    if !bucket().take() {
        crate::observability::rejects::counter()
            .with_label_values(&["rate_limit"])
            .inc();
        return (StatusCode::TOO_MANY_REQUESTS, "rate limited").into_response();
    }
    next.run(req).await
}
