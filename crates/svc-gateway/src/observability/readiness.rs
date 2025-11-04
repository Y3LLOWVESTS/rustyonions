//! Readiness sampler (lightweight, no `AppState` changes).
//! RO:WHAT   Maintain a global readiness snapshot updated on a short interval.
//! RO:WHY    Let `/readyz` consult real gates instead of a hard-coded toggle.
//! RO:SHAPE  Global `OnceLock` + Atomics so we don't touch `AppState` (tiny blast radius).
//! RO:FUTURE Hook real signals (inflight, error rate, queue depth) as they land.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    OnceLock,
};
use std::time::Duration;
use tokio::task::JoinHandle;

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub inflight_current: u64,
    pub error_rate_pct: u64,
    pub queue_saturated: bool,
}

struct ReadyState {
    inflight_current: AtomicU64,
    error_rate_pct: AtomicU64,
    queue_saturated: AtomicBool,
}

impl ReadyState {
    const fn new() -> Self {
        Self {
            inflight_current: AtomicU64::new(0),
            error_rate_pct: AtomicU64::new(0),
            queue_saturated: AtomicBool::new(false),
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            inflight_current: self.inflight_current.load(Ordering::Relaxed),
            error_rate_pct: self.error_rate_pct.load(Ordering::Relaxed),
            queue_saturated: self.queue_saturated.load(Ordering::Relaxed),
        }
    }
}

static READY: OnceLock<ReadyState> = OnceLock::new();
static TASK: OnceLock<JoinHandle<()>> = OnceLock::new();

fn ready() -> &'static ReadyState {
    READY.get_or_init(ReadyState::new)
}

pub fn ensure_started() {
    if TASK.get().is_some() {
        return;
    }
    let handle = tokio::spawn(async move {
        let tick = Duration::from_millis(500);
        loop {
            // Take a snapshot and publish as gauges (gateway-prefixed to avoid collisions).
            let snap = ready().snapshot();
            crate::observability::ready_metrics::set_inflight(snap.inflight_current);
            crate::observability::ready_metrics::set_error_pct(snap.error_rate_pct);
            crate::observability::ready_metrics::set_queue_saturated(snap.queue_saturated);
            tokio::time::sleep(tick).await;
        }
    });
    let _ = TASK.set(handle);
}

#[must_use]
pub fn snapshot() -> Snapshot {
    ready().snapshot()
}

#[derive(Clone, Copy)]
pub struct Thresholds {
    pub max_error_pct: u64,
    pub max_inflight: u64,
    pub allow_queue_saturation: bool,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_error_pct: 5,
            max_inflight: 10_000,
            allow_queue_saturation: false,
        }
    }
}
