//! RO:WHAT — Readiness module facade for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF. Exposes health gates used by HTTP and supervisor code.
//! RO:INTERACTS — http handlers, metrics.
//! RO:INVARIANTS — readiness is explicit and observable.
//! RO:METRICS — degraded causes are mirrored by handlers.
//! RO:CONFIG — config_loaded gate reflects validated config.
//! RO:SECURITY — no sensitive dependency detail in public body.
//! RO:TEST — integration/readiness.rs.

pub mod health;

pub use health::{HealthSnapshot, HealthState};
