// crates/omnigate/src/admission/quotas.rs
//! RO:WHAT  Global + per-IP token-bucket admission guards that return 429 when over limit.
//! RO:WHY   Prevent abuse/overload by bounding request rate upfront, before heavy work.
//! RO:INVARS Constant-time hot path; no label explosion; pure edge guards; no drift in deps.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    Router,
};

use crate::errors::GateError;

// -----------------------------
// Bucket + limiter primitives
// -----------------------------

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last: Instant,
    rate_per_sec: f64,
    burst: f64,
}

impl Bucket {
    fn new(qps: u64, burst: u64) -> Self {
        let now = Instant::now();
        Self {
            tokens: burst as f64,
            last: now,
            rate_per_sec: qps as f64,
            burst: burst as f64,
        }
    }

    #[inline]
    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();
        self.last = now;

        // refill
        self.tokens = (self.tokens + dt * self.rate_per_sec).min(self.burst);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// -----------------------------
// Global limiter
// -----------------------------

#[derive(Clone)]
struct GlobalLimiter {
    inner: Arc<Mutex<Bucket>>,
}

impl GlobalLimiter {
    fn new(qps: u64, burst: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Bucket::new(qps, burst))),
        }
    }

    #[inline]
    fn allow(&self) -> bool {
        let mut b = self.inner.lock().expect("global limiter poisoned");
        b.allow()
    }
}

async fn global_quota_guard(
    State(glob): State<GlobalLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if glob.allow() {
        next.run(req).await
    } else {
        // Heuristic small backoff; refine later if needed.
        let retry_ms = 50u64;

        // Metrics bump (scope=global, reason=qps).
        crate::metrics::gates::QUOTA_REJECT_TOTAL
            .with_label_values(&["global", "qps"])
            .inc();

        GateError::RateLimitedGlobal {
            retry_after_ms: retry_ms,
        }
        .into_response()
    }
}

// -----------------------------
// Per-IP limiter (optional)
// -----------------------------

#[derive(Clone)]
struct IpLimiter {
    rate: u64,
    burst: u64,
    enabled: bool,
    by_ip: Arc<Mutex<HashMap<String, Bucket>>>,
}

impl IpLimiter {
    fn new(enabled: bool, qps: u64, burst: u64) -> Self {
        Self {
            rate: qps,
            burst,
            enabled,
            by_ip: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[inline]
    fn allow(&self, ip: &str) -> bool {
        if !self.enabled {
            return true;
        }
        let mut map = self.by_ip.lock().expect("ip limiter poisoned");
        let b = map
            .entry(ip.to_string())
            .or_insert_with(|| Bucket::new(self.rate, self.burst));
        b.allow()
    }
}

fn ip_from_headers(headers: &HeaderMap) -> String {
    if let Some(v) = headers.get("x-forwarded-for") {
        if let Ok(s) = v.to_str() {
            if let Some(first) = s.split(',').next() {
                return first.trim().to_string();
            }
        }
    }
    if let Some(v) = headers.get("x-real-ip") {
        if let Ok(s) = v.to_str() {
            return s.trim().to_string();
        }
    }
    "local".to_string()
}

async fn ip_quota_guard(State(ipq): State<IpLimiter>, req: Request<Body>, next: Next) -> Response {
    let ip = ip_from_headers(req.headers());
    if ipq.allow(&ip) {
        next.run(req).await
    } else {
        let retry_ms = 50u64;

        // Metrics bump (scope=ip, reason=qps).
        crate::metrics::gates::QUOTA_REJECT_TOTAL
            .with_label_values(&["ip", "qps"])
            .inc();

        GateError::RateLimitedIp {
            retry_after_ms: retry_ms,
        }
        .into_response()
    }
}

// -----------------------------
// Attach points
// -----------------------------

/// Attach the quota limiter layer with **default constants** (test/back-compat).
/// Prefer `attach_with_cfg` in production paths.
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .layer(from_fn_with_state(
            GlobalLimiter::new(500, 1000),
            global_quota_guard,
        ))
        .layer(from_fn_with_state(
            IpLimiter::new(false, 200, 400),
            ip_quota_guard,
        ))
}

/// Attach quota limiters using values from **Admission** config.
/// Order: global first (cheap, broad), then per-IP (optional).
pub fn attach_with_cfg<S>(router: Router<S>, adm: &crate::config::Admission) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let ip = &adm.ip_quota;

    router
        .layer(from_fn_with_state(
            GlobalLimiter::new(adm.global_quota.qps, adm.global_quota.burst),
            global_quota_guard,
        ))
        .layer(from_fn_with_state(
            IpLimiter::new(ip.enabled, ip.qps, ip.burst),
            ip_quota_guard,
        ))
}
