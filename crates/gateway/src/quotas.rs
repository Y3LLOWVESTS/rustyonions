// crates/gateway/src/quotas.rs
#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    env,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Global quotas singleton (lazy).
static QUOTAS: OnceLock<Quotas> = OnceLock::new();

/// Token-bucket quotas (per tenant).
pub struct Quotas {
    inner: Mutex<HashMap<String, Bucket>>,
    rate_per_sec: f64,
    burst: f64,
}

#[derive(Clone)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

impl Quotas {
    fn new(rate_per_sec: f64, burst: f64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            rate_per_sec,
            burst,
        }
    }

    fn enabled(&self) -> bool {
        self.rate_per_sec > 0.0 && self.burst > 0.0
    }

    /// Returns None if allowed; Some(retry_after_secs) if throttled.
    async fn check_and_consume(&self, tenant: &str, cost: f64) -> Option<u64> {
        if !self.enabled() {
            return None;
        }

        let mut map = self.inner.lock().await;
        let now = Instant::now();
        let b = map.entry(tenant.to_string()).or_insert_with(|| Bucket {
            tokens: self.burst,
            last: now,
        });

        // Refill
        let dt = now.duration_since(b.last);
        let refill = self.rate_per_sec * secs(dt);
        b.tokens = (b.tokens + refill).min(self.burst);
        b.last = now;

        // Consume or compute wait
        if b.tokens >= cost {
            b.tokens -= cost;
            None
        } else {
            let needed = cost - b.tokens;
            // How many whole seconds to the next available token(s)?
            let secs = (needed / self.rate_per_sec).ceil().max(0.0) as u64;
            Some(secs)
        }
    }
}

#[inline]
fn secs(d: Duration) -> f64 {
    d.as_secs() as f64 + d.subsec_nanos() as f64 / 1_000_000_000.0
}

/// Initialize from env on first use; subsequent calls return the same instance.
/// RON_QUOTA_RPS=rate (float), RON_QUOTA_BURST=burst (float).
fn quotas() -> &'static Quotas {
    QUOTAS.get_or_init(|| {
        let rate = env::var("RON_QUOTA_RPS").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let burst = env::var("RON_QUOTA_BURST").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        Quotas::new(rate, burst)
    })
}

/// Public check: return None if allowed, Some(retry_after_secs) if throttled.
pub async fn check(tenant: &str) -> Option<u64> {
    quotas().check_and_consume(tenant, 1.0).await
}
