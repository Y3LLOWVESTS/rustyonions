//! RO:WHAT — ron-policy public API: load/validate bundles and evaluate decisions.
//!
//! RO:WHY  — Pillar 2 (Policy & Governance); Concerns: SEC/GOV. Deny-by-default guardrail.
//!
//! RO:INTERACTS — model, `parse::{json,toml,validate}`, `engine::{eval,index,obligations,metrics}`, `explain::trace`, economics
//!
//! RO:INVARIANTS — DTOs are strict; no locks across `.await`; OAP caps: frame=1 MiB, chunk≈64 KiB (context only)
//!
//! RO:METRICS — `requests_total`, `rejected_total{reason}`, `eval_latency_seconds`; economics metrics emitted by consumers
//!
//! RO:CONFIG — no service config; economics policy is parsed from caller-provided bytes
//!
//! RO:SECURITY — capability enforcement happens in services; this crate only decides allow/deny and validates economics policy
//!
//! RO:TEST — unit tests under `tests/*.rs`; bench: `benches/eval_throughput.rs`

#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod errors;
pub mod features;
pub mod model;
pub mod moderation;
pub mod persistence;
pub mod service_node_eligibility;
pub mod service_node_enforcement;
pub mod signed_moderation;

pub mod ctx;
pub mod economics;
pub mod engine;
pub mod explain;
pub mod parse;

pub use ctx::Context;
pub use economics::{
    canonical_internal_roc_economics_bytes, internal_roc_economics_config_hash,
    load_economics_toml, load_internal_roc_economics_toml_for_profile,
    normalized_internal_roc_economics_config, validate_economics_policy, ActionEconomics,
    EconomicsLimits, EconomicsPolicy, InternalRocEconomicsProfile, PayoutSplit, PricingKind,
    RoundingMode,
};
pub use engine::eval::{Decision, DecisionEffect, Evaluator};
pub use explain::trace::{DecisionTrace, TraceStep};
pub use model::{Action, Obligation, PolicyBundle, Rule, RuleCondition};
pub use moderation::{
    B3Id, B3IdError, CompositionError as ModerationPolicyCompositionError,
    Decision as ModerationDecision, Effect as ModerationEffect, Policy as ModerationPolicy,
    ReasonCode as ModerationReasonCode,
};
pub use persistence::{
    Decision as PersistenceDecision, Effect as PersistenceEffect, Intent as PersistenceIntent,
    Policy as PersistencePolicy, ReasonCode as PersistenceReasonCode,
    ReviewLevel as PersistenceReviewLevel,
};
pub use service_node_eligibility::{
    ServiceNodeEligibilityDecisionV1, ServiceNodeEligibilityObservationV1,
    ServiceNodeEligibilityPolicyError, ServiceNodeEligibilityPolicyV1,
    ServiceNodeEligibilityReasonCodeV1, ServiceNodeHistoryThresholdsV1,
    ServiceNodeRewardReviewPostureV1, SERVICE_NODE_ELIGIBILITY_POLICY_VERSION,
};
pub use service_node_enforcement::{
    ServiceNodeViolationDecisionReasonV1, ServiceNodeViolationDecisionV1,
    ServiceNodeViolationPolicyError, ServiceNodeViolationPolicyV1,
    SERVICE_NODE_VIOLATION_POLICY_VERSION,
};
pub use signed_moderation::{
    encode_ed25519_signature, verify_signed_moderation_policy, SignedModerationPolicyError,
    SignedModerationPolicyV1, TrustedModerationSigner, VerifiedModerationPolicy,
    SIGNED_MODERATION_POLICY_DOMAIN, SIGNED_MODERATION_POLICY_VERSION,
};

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
