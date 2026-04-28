//! RO:WHAT — Tracing initialization for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: PERF/RES/GOV. Operators need structured logs around economic runs.
//! RO:INTERACTS — main bootstrap and config::LogConfig.
//! RO:INVARIANTS — never log Authorization headers or raw snapshots.
//! RO:METRICS — tracing is separate from Prometheus metrics.
//! RO:CONFIG — log.level and log.format.
//! RO:SECURITY — secret redaction is enforced by caller discipline and no header logging.
//! RO:TEST — compile coverage; manual run smoke.

use tracing_subscriber::EnvFilter;

use crate::config::LogConfig;

/// Initialize tracing once. Repeated calls are ignored by tracing subscriber.
pub fn init_tracing(cfg: &LogConfig) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cfg.level));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if cfg.format == "json" {
        let _ = builder.json().try_init();
    } else {
        let _ = builder.try_init();
    }
}
