//! RO:WHAT — Prometheus metrics registry for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: PERF/RES/GOV. Reward economics must be observable under normal and degraded paths.
//! RO:INTERACTS — http handlers, readiness, main bootstrap.
//! RO:INVARIANTS — metric names are stable; labels are low cardinality; registry is per-state to avoid test collisions.
//! RO:METRICS — reward_runs_total, reward_compute_latency_seconds, ledger_intents_total, settlement_intents_planned_total.
//! RO:CONFIG — metrics_addr is reserved for split serving later.
//! RO:SECURITY — no secrets or account IDs in metric labels.
//! RO:TEST — readiness/http integration tests scrape metrics text.

use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGaugeVec, Opts, Registry,
    TextEncoder,
};

use crate::{Result, RewarderError};

/// Service metrics handle.
#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    reward_runs_total: IntCounterVec,
    compute_latency_seconds: Histogram,
    ledger_intents_total: IntCounterVec,
    settlement_intents_planned_total: IntCounter,
    rejected_total: IntCounterVec,
    readyz_degraded: IntGaugeVec,
}

impl Metrics {
    /// Create a fresh per-service registry.
    pub fn new() -> Result<Self> {
        let registry = Registry::new_custom(Some("svc_rewarder".into()), None)
            .map_err(|e| RewarderError::Internal(e.to_string()))?;

        let reward_runs_total = IntCounterVec::new(
            Opts::new("reward_runs_total", "Reward runs by status."),
            &["status"],
        )
        .map_err(|e| RewarderError::Internal(e.to_string()))?;

        let compute_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "reward_compute_latency_seconds",
            "Reward compute latency in seconds.",
        ))
        .map_err(|e| RewarderError::Internal(e.to_string()))?;

        let ledger_intents_total = IntCounterVec::new(
            Opts::new("ledger_intents_total", "Ledger/wallet intent results."),
            &["result"],
        )
        .map_err(|e| RewarderError::Internal(e.to_string()))?;

        let settlement_intents_planned_total = IntCounter::new(
            "settlement_intents_planned_total",
            "Number of wallet settlement intents planned from reward manifests.",
        )
        .map_err(|e| RewarderError::Internal(e.to_string()))?;

        let rejected_total = IntCounterVec::new(
            Opts::new("rejected_total", "Rejected rewarder requests by reason."),
            &["reason"],
        )
        .map_err(|e| RewarderError::Internal(e.to_string()))?;

        let readyz_degraded = IntGaugeVec::new(
            Opts::new("readyz_degraded", "Readiness degraded causes."),
            &["cause"],
        )
        .map_err(|e| RewarderError::Internal(e.to_string()))?;

        registry
            .register(Box::new(reward_runs_total.clone()))
            .map_err(|e| RewarderError::Internal(e.to_string()))?;
        registry
            .register(Box::new(compute_latency_seconds.clone()))
            .map_err(|e| RewarderError::Internal(e.to_string()))?;
        registry
            .register(Box::new(ledger_intents_total.clone()))
            .map_err(|e| RewarderError::Internal(e.to_string()))?;
        registry
            .register(Box::new(settlement_intents_planned_total.clone()))
            .map_err(|e| RewarderError::Internal(e.to_string()))?;
        registry
            .register(Box::new(rejected_total.clone()))
            .map_err(|e| RewarderError::Internal(e.to_string()))?;
        registry
            .register(Box::new(readyz_degraded.clone()))
            .map_err(|e| RewarderError::Internal(e.to_string()))?;

        Ok(Self {
            registry,
            reward_runs_total,
            compute_latency_seconds,
            ledger_intents_total,
            settlement_intents_planned_total,
            rejected_total,
            readyz_degraded,
        })
    }

    /// Increment run counter.
    pub fn inc_run(&self, status: &str) {
        self.reward_runs_total.with_label_values(&[status]).inc();
    }

    /// Observe compute latency.
    pub fn observe_compute_seconds(&self, seconds: f64) {
        self.compute_latency_seconds.observe(seconds);
    }

    /// Increment intent result counter.
    pub fn inc_intent(&self, result: &str) {
        self.ledger_intents_total.with_label_values(&[result]).inc();
    }

    /// Increment planned settlement intent counter.
    pub fn inc_planned_intents(&self, count: usize) {
        self.settlement_intents_planned_total
            .inc_by(u64::try_from(count).unwrap_or(u64::MAX));
    }

    /// Increment reject counter.
    pub fn inc_reject(&self, reason: &str) {
        self.rejected_total.with_label_values(&[reason]).inc();
    }

    /// Set readiness degradation gauge for a cause.
    pub fn set_degraded(&self, cause: &str, degraded: bool) {
        self.readyz_degraded
            .with_label_values(&[cause])
            .set(if degraded { 1 } else { 0 });
    }

    /// Render Prometheus text format.
    pub fn render(&self) -> Result<String> {
        let families = self.registry.gather();
        let mut buf = Vec::new();
        TextEncoder::new()
            .encode(&families, &mut buf)
            .map_err(|e| RewarderError::Internal(e.to_string()))?;
        String::from_utf8(buf).map_err(|e| RewarderError::Internal(e.to_string()))
    }
}
