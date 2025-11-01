//! RO:WHAT   Compute /readyz based on inflight, 429/503 error rate, queue saturation.
//! RO:WHY    Truthful readiness prevents cascading failure during overload.
//! RO:INVARS Hold degraded state for cfg.hold_for; do not flap rapidly.
//!
//! Notes:
//! - Emits Prometheus metrics (gauges + counters) under `crate::metrics::gates::*`.
//! - We only count a transition when the state actually changes
//!   (i.e., we don't increment on every `ready()` call while already degraded).

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---- Readiness configuration ------------------------------------------------

#[derive(Clone)]
pub struct ReadyCfg {
    /// Max allowed in-flight requests before we degrade readiness.
    pub max_inflight_threshold: u64,
    /// Rolling 429/503 error rate (percentage, e.g., 2.0 means 2%) to trip readiness.
    pub error_rate_pct: f64,
    /// Window over which error rate is computed.
    pub window: Duration,
    /// How long to hold degraded state before returning to ready (anti-flap).
    pub hold_for: Duration,
}

// ---- Internal state ---------------------------------------------------------

#[derive(Default)]
struct State {
    last_trip: Option<Instant>,
    inflight: u64,
    rolling_err_rate: f64,
    queue_saturated: bool,
}

// ---- Policy -----------------------------------------------------------------

#[derive(Clone, Default)]
pub struct ReadyPolicy {
    cfg: ReadyCfg,
    state: Arc<Mutex<State>>,
}

impl ReadyPolicy {
    pub fn new(cfg: ReadyCfg) -> Self {
        Self {
            cfg,
            state: Default::default(),
        }
    }

    /// Update current in-flight request count (called by admission/queues).
    pub fn update_inflight(&self, v: u64) {
        {
            self.state.lock().inflight = v;
        }
        // Gauge: ready_inflight_current
        crate::metrics::gates::READY_INFLIGHT_CURRENT.set(v as i64);
    }

    /// Update rolling error rate percentage (0.0–100.0) for 429/503s.
    pub fn update_err_rate(&self, pct: f64) {
        {
            self.state.lock().rolling_err_rate = pct;
        }
        // Gauge: ready_error_rate_pct
        crate::metrics::gates::READY_ERROR_RATE_PCT.set(pct);
    }

    /// Mark whether the queue/dispatcher is saturated (backpressure).
    pub fn set_queue_saturated(&self, yes: bool) {
        {
            self.state.lock().queue_saturated = yes;
        }
        // Gauge: ready_queue_saturated (0/1)
        crate::metrics::gates::READY_QUEUE_SATURATED.set(if yes { 1 } else { 0 });
    }

    /// Compute whether the service is currently ready.
    ///
    /// Returns `true` when ready; `false` when degraded.
    /// Holds degraded state for `cfg.hold_for` to avoid flapping.
    pub fn ready(&self) -> bool {
        let mut s = self.state.lock();
        let now = Instant::now();

        // Trip conditions (in priority order to label the reason deterministically).
        let inflight_trip = s.inflight > self.cfg.max_inflight_threshold;
        let err_rate_trip = s.rolling_err_rate >= self.cfg.error_rate_pct;
        let queue_trip = s.queue_saturated;
        let tripped = inflight_trip || err_rate_trip || queue_trip;

        if tripped {
            // Only count a transition when we *enter* degraded (avoid double-counting).
            let entering_degraded = s.last_trip.is_none();
            if entering_degraded {
                // Counter: ready_trips_total{reason=...}
                if inflight_trip {
                    crate::metrics::gates::READY_TRIPS_TOTAL
                        .with_label_values(&["inflight"])
                        .inc();
                } else if err_rate_trip {
                    crate::metrics::gates::READY_TRIPS_TOTAL
                        .with_label_values(&["err_rate"])
                        .inc();
                } else if queue_trip {
                    crate::metrics::gates::READY_TRIPS_TOTAL
                        .with_label_values(&["queue"])
                        .inc();
                }
                // Counter: ready_state_changes_total{to="degraded"}
                crate::metrics::gates::READY_STATE_CHANGES_TOTAL
                    .with_label_values(&["degraded"])
                    .inc();
            }
            s.last_trip = Some(now);
            return false;
        }

        // If previously degraded, enforce hold period before returning to ready.
        if let Some(t) = s.last_trip {
            if now.duration_since(t) < self.cfg.hold_for {
                return false;
            }
            // Transition: degraded -> ready
            s.last_trip = None;
            crate::metrics::gates::READY_STATE_CHANGES_TOTAL
                .with_label_values(&["ready"])
                .inc();
        }

        true
    }
}
