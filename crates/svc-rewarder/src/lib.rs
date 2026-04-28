//! RO:WHAT — Library surface for the svc-rewarder deterministic ROC reward service.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/GOV. Keeps reward math pure while exposing a small service API.
//! RO:INTERACTS — config, http, core, inputs, outputs, metrics, readiness, bus, security.
//! RO:INVARIANTS — forbid unsafe; deterministic manifests; integer-only money; no external-chain logic.
//! RO:METRICS — exposes Metrics for reward runs, rejects, latency, and ledger-intent outcomes.
//! RO:CONFIG — Config loaded by config::load with env/file overlays; amnesia honored.
//! RO:SECURITY — capability checks live in security::caps; DTOs use deny_unknown_fields.
//! RO:TEST — unit: core/config; integration: http_compute/readiness/egress_dedupe.

#![forbid(unsafe_code)]
#![deny(clippy::await_holding_lock)]

pub mod bus;
pub mod concurrency;
pub mod config;
pub mod core;
pub mod errors;
pub mod http;
pub mod inputs;
pub mod metrics;
pub mod outputs;
pub mod prelude;
pub mod readiness;
pub mod security;
pub mod telemetry;
pub mod util;

pub use crate::config::Config;
pub use crate::errors::{Result, RewarderError};
pub use crate::http::RewarderState;
pub use crate::metrics::Metrics;
pub use crate::readiness::HealthState;
