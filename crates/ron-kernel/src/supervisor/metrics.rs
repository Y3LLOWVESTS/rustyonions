// crates/ron-kernel/src/supervisor/metrics.rs
#![forbid(unsafe_code)]
// Allow expect() only during startup-time metric construction (never in hot paths).
#![allow(clippy::expect_used)]

use std::sync::OnceLock;

use prometheus::{register, GaugeVec, IntCounterVec, Opts};

/// Supervisor metrics bundle kept for ergonomics in call sites.
/// Currently not constructed by callers; kept for future ergonomic use.
#[allow(dead_code)]
#[derive(Clone)]
pub struct SupervisorMetrics {
    /// Total number of restarts performed by the supervisor (label: service)
    pub restarts_total: IntCounterVec,
    /// Current backoff delay before restarting a service (seconds; f64) (label: service)
    pub backoff_seconds: GaugeVec,
}

impl SupervisorMetrics {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            restarts_total: restarts_metric().clone(),
            backoff_seconds: backoff_metric().clone(),
        }
    }
}

/// Global counter for restarts (labels: service).
/// Usage: `restarts_metric().with_label_values(&[service]).inc();`
pub fn restarts_metric() -> &'static IntCounterVec {
    static RESTARTS: OnceLock<IntCounterVec> = OnceLock::new();

    RESTARTS.get_or_init(|| {
        let v = IntCounterVec::new(
            Opts::new(
                "supervisor_restarts_total",
                "Total number of restarts performed by the supervisor",
            ),
            &["service"],
        )
        .expect("IntCounterVec::new(supervisor_restarts_total)");
        let _ = register(Box::new(v.clone())); // ignore AlreadyRegistered
        v
    })
}

/// Global gauge for the **current** backoff delay (seconds; f64) (labels: service).
/// Usage: `backoff_metric().with_label_values(&[service]).set(delay_secs_f64);`
pub fn backoff_metric() -> &'static GaugeVec {
    static BACKOFF_GAUGE: OnceLock<GaugeVec> = OnceLock::new();

    BACKOFF_GAUGE.get_or_init(|| {
        let g = GaugeVec::new(
            Opts::new(
                "supervisor_backoff_seconds",
                "Current backoff delay before restarting a service",
            ),
            &["service"],
        )
        .expect("GaugeVec::new(supervisor_backoff_seconds)");
        let _ = register(Box::new(g.clone())); // ignore AlreadyRegistered
        g
    })
}
