//! RO:WHAT  AdminState: thresholds, kernel handles, sticky hold, and ReadyPolicy handle.

use super::policy::ReadyPolicy;
use ron_kernel::metrics::{health::HealthState, readiness::Readiness as KernelReadiness};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
pub struct AdminState {
    pub health: HealthState,
    pub ready: KernelReadiness,
    pub dev_ready: bool,
    pub max_inflight_threshold: i64,
    pub error_rate_429_503_pct: f64,
    pub hold_for_secs: u64,
    pub rp: Arc<ReadyPolicy>,
    hold_until: Arc<Mutex<Option<Instant>>>,
}

impl AdminState {
    pub fn new(
        health: HealthState,
        ready: KernelReadiness,
        dev_ready: bool,
        cfg: &crate::config::Readiness,
        rp: Arc<ReadyPolicy>,
    ) -> Self {
        Self {
            health,
            ready,
            dev_ready,
            max_inflight_threshold: cfg.max_inflight_threshold as i64,
            error_rate_429_503_pct: cfg.error_rate_429_503_pct,
            hold_for_secs: cfg.hold_for_secs,
            rp,
            hold_until: Arc::new(Mutex::new(None)),
        }
    }

    #[inline]
    pub fn hold_until_lock(&self) -> std::sync::MutexGuard<'_, Option<Instant>> {
        self.hold_until.lock().expect("hold_until mutex poisoned")
    }

    #[inline]
    pub fn set_hold_until(&self, when: Instant) {
        *self.hold_until.lock().unwrap() = Some(when);
    }

    #[inline]
    pub fn clear_hold_until(&self) {
        *self.hold_until.lock().unwrap() = None;
    }
}
