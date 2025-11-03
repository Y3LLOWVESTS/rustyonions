//! RO:WHAT  Local readiness policy bridge with atomics (truth source for /readyz).
//! RO:WHY   /readyz must reflect *actual* concurrency/error pressure.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

pub struct ReadyPolicy {
    inflight: AtomicI64,
    err_rate_bits: AtomicU64, // f64 stored as bits for atomicity
    queue_saturated: AtomicBool,
}

impl ReadyPolicy {
    #[inline]
    pub fn new() -> Self {
        Self {
            inflight: AtomicI64::new(0),
            err_rate_bits: AtomicU64::new(0f64.to_bits()),
            queue_saturated: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn update_inflight(&self, v: i64) {
        let val = v.max(0);
        self.inflight.store(val, Ordering::Release);
        // mirror to gauge for observability
        crate::metrics::gates::READY_INFLIGHT_CURRENT.set(val);
    }

    #[inline]
    pub fn inc(&self) {
        let v = self.inflight.fetch_add(1, Ordering::AcqRel) + 1;
        crate::metrics::gates::READY_INFLIGHT_CURRENT.set(v.max(0));
    }

    #[inline]
    pub fn dec(&self) {
        let v = self.inflight.fetch_sub(1, Ordering::AcqRel) - 1;
        crate::metrics::gates::READY_INFLIGHT_CURRENT.set(v.max(0));
    }

    #[inline]
    pub fn update_err_rate(&self, pct: f64) {
        let c = pct.clamp(0.0, 100.0);
        self.err_rate_bits.store(c.to_bits(), Ordering::Release);
        crate::metrics::gates::READY_ERROR_RATE_PCT.set(c);
    }

    #[inline]
    pub fn set_queue_saturated(&self, on: bool) {
        self.queue_saturated.store(on, Ordering::Release);
        crate::metrics::gates::READY_QUEUE_SATURATED.set(if on { 1 } else { 0 });
    }

    #[inline]
    pub fn inflight(&self) -> i64 {
        self.inflight.load(Ordering::Acquire)
    }

    #[inline]
    pub fn err_rate_pct(&self) -> f64 {
        f64::from_bits(self.err_rate_bits.load(Ordering::Acquire))
    }

    #[allow(dead_code)]
    #[inline]
    pub fn queue_saturated(&self) -> bool {
        self.queue_saturated.load(Ordering::Acquire)
    }
}

// Silence clippy::new_without_default by providing Default in terms of new()
impl Default for ReadyPolicy {
    fn default() -> Self {
        Self::new()
    }
}
