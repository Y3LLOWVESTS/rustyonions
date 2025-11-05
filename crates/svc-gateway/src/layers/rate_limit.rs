//! Global token-bucket rate limit (lock-free fast path).
//! RO:WHAT   Enforce RPS with fixed-capacity bucket; emit 429 + Retry-After.
//! RO:WHY    Cheap back-pressure/abuse damping without mutex contention.
//! RO:METRICS increments `gateway_rejections_total{reason="rate_limit"}` on reject.
//! RO:CONFIG `SVC_GATEWAY_RL_RPS` (u64), `SVC_GATEWAY_RL_BURST` (u64), `SVC_GATEWAY_RL_TARPIT_MS` (u64).

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

use crate::observability::rejects::counter as rejects_counter;

#[derive(Debug)]
struct TokenBucket {
    /// steady refill rate (tokens/sec)
    rps: u64,
    /// hard cap on tokens (burst)
    capacity: u64,
    /// current tokens (0..=capacity)
    tokens: AtomicU64,
    /// last whole second we refilled (unix seconds)
    last_sec: AtomicU64,
}

impl TokenBucket {
    fn new(rps: u64, burst: u64) -> Self {
        let now = now_secs();
        // capacity must be at least 1 to avoid degenerate 0-cap bucket
        let cap = burst.max(1);
        Self {
            rps: rps.max(1),
            capacity: cap,
            tokens: AtomicU64::new(cap), // start full
            last_sec: AtomicU64::new(now),
        }
    }

    #[inline]
    fn refill_if_needed(&self) {
        let cur = now_secs();
        let last = self.last_sec.load(Ordering::Relaxed);

        if cur > last
            && self
                .last_sec
                .compare_exchange(last, cur, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            // elapsed whole seconds since the last successful refill tick
            let elapsed = cur.saturating_sub(last);
            if elapsed > 0 {
                let add = elapsed.saturating_mul(self.rps);
                // cap at capacity; safe because both are u64
                let before = self.tokens.load(Ordering::Relaxed);
                let after = before.saturating_add(add).min(self.capacity);
                // store (not critical if we race; any winner that writes <= capacity is fine)
                self.tokens.store(after, Ordering::Relaxed);
            }
        }
    }

    /// Try to consume one token.
    /// Never panics; never underflows; wait-free fast path on success.
    #[must_use]
    #[inline]
    fn try_take(&self) -> bool {
        self.refill_if_needed();

        // lock-free decrement with CAS so we never subtract below zero
        let mut cur = self.tokens.load(Ordering::Relaxed);
        loop {
            if cur == 0 {
                return false;
            }
            // we know cur > 0, so cur - 1 is safe; CAS guards against races
            match self.tokens.compare_exchange_weak(
                cur,
                cur - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(observed) => {
                    cur = observed;
                    // retry until we either see 0 or win the CAS
                }
            }
        }
    }
}

#[inline]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

/// Route-scoped middleware: enforce global RPS (lock-free).
///
/// # Behavior
/// - Honors env:
///   - `SVC_GATEWAY_RL_RPS` (default 5000)
///   - `SVC_GATEWAY_RL_BURST` (default = RPS)
///   - `SVC_GATEWAY_RL_TARPIT_MS` (default 0; add small sleep on reject)
/// - On reject: returns 429 and `Retry-After: 1` header, increments `gateway_rejections_total{reason="rate_limit"}`
///
/// # Errors
/// Never returns an error directly; upstream handler may.
pub async fn rate_limit_mw(req: Request<Body>, next: Next) -> Response {
    static BUCKET: OnceCell<TokenBucket> = OnceCell::new();
    static TARPIT_MS: OnceCell<u64> = OnceCell::new();

    let rps = std::env::var("SVC_GATEWAY_RL_RPS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5_000);
    let burst = std::env::var("SVC_GATEWAY_RL_BURST")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(rps);
    let tarpit_ms = *TARPIT_MS.get_or_init(|| {
        std::env::var("SVC_GATEWAY_RL_TARPIT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    });

    let bucket = BUCKET.get_or_init(|| TokenBucket::new(rps, burst));

    if bucket.try_take() {
        next.run(req).await
    } else {
        rejects_counter().with_label_values(&["rate_limit"]).inc();
        if tarpit_ms > 0 {
            sleep(Duration::from_millis(tarpit_ms)).await;
        }
        (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", "1")],
            "rate limited",
        )
            .into_response()
    }
}
