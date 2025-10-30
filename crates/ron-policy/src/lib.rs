//! RO:WHAT — ron-policy public API: load/validate bundles and evaluate decisions.
//!
//! RO:WHY  — Pillar 2 (Policy & Governance); Concerns: SEC/GOV. Deny-by-default guardrail.
//!
//! RO:INTERACTS — model, `parse::{json,toml,validate}`, `engine::{eval,index,obligations,metrics}`, `explain::trace`
//!
//! RO:INVARIANTS — DTOs are strict; no locks across `.await`; OAP caps: frame=1 MiB, chunk≈64 KiB (context only)
//!
//! RO:METRICS — `requests_total`, `rejected_total{reason}`, `eval_latency_seconds`
//!
//! RO:CONFIG — none (pure library); amnesia has no persistence here
//!
//! RO:SECURITY — capability enforcement happens in services; this crate only decides allow/deny
//!
//! RO:TEST — unit tests under `tests/*.rs`; bench: `benches/eval_throughput.rs`

#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod errors;
pub mod features;
pub mod model;

pub mod ctx;
pub mod engine;
pub mod explain;
pub mod parse;

pub use ctx::Context;
pub use engine::eval::{Decision, DecisionEffect, Evaluator};
pub use explain::trace::{DecisionTrace, TraceStep};
pub use model::{Action, Obligation, PolicyBundle, Rule, RuleCondition};

/// Convenience: load a bundle from JSON bytes.
///
/// # Errors
///
/// Returns `Error::Parse` on malformed JSON or `Error::Validation` if the
/// resulting `PolicyBundle` violates invariants.
pub fn load_json(bytes: &[u8]) -> Result<PolicyBundle, errors::Error> {
    let bundle = parse::json::from_slice(bytes)?;
    parse::validate::validate(&bundle)?;
    Ok(bundle)
}

/// Convenience: load a bundle from TOML bytes.
///
/// # Errors
///
/// Returns `Error::Parse` on malformed TOML or `Error::Validation` if the
/// resulting `PolicyBundle` violates invariants.
pub fn load_toml(bytes: &[u8]) -> Result<PolicyBundle, errors::Error> {
    let bundle = parse::toml::from_slice(bytes)?;
    parse::validate::validate(&bundle)?;
    Ok(bundle)
}
