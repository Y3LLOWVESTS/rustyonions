//! RO:WHAT — Reward policy DTO, resolver, and validation helpers for deterministic reward epochs.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Policy hash/id are part of the idempotent run key.
//! RO:INTERACTS — core::compute, http DTOs, manifest policy summary, future policy registry adapter.
//! RO:INVARIANTS — signed flag explicit; integer caps; no floating weights; canonical b3 policy hash.
//! RO:METRICS — stale/invalid policy counted by callers.
//! RO:CONFIG — default policy id from Config.rewarder.policy_id.
//! RO:SECURITY — policy hash is caller-verified in batch 2; signature verification seam remains explicit.
//! RO:TEST — tests/unit/accounting_policy.rs and idempotency/config tests.

use serde::{Deserialize, Serialize};

use crate::core::algebra::AmountMinor;
use crate::{Result, RewarderError};

/// Deterministic reward policy for one compute request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardPolicy {
    /// Logical policy id.
    pub id: String,
    /// Expected policy content hash.
    pub hash: String,
    /// Whether policy is signed/verified by the caller or future registry adapter.
    pub signed: bool,
    /// Max total payout for this epoch.
    pub max_payout_minor_units: AmountMinor,
    /// Minimum payout included in payout vector; dust goes to residual.
    pub min_payout_minor_units: AmountMinor,
    /// Basis-point multiplier applied to raw scores.
    pub weight_bps: u32,
    /// Rounding mode. Batch 2 supports `floor`.
    pub rounding: String,
}

impl RewardPolicy {
    /// Conservative default policy matching local dev docs.
    #[must_use]
    pub fn dev_default(policy_id: impl Into<String>, hash: impl Into<String>) -> Self {
        Self {
            id: policy_id.into(),
            hash: hash.into(),
            signed: true,
            max_payout_minor_units: AmountMinor(u128::MAX),
            min_payout_minor_units: AmountMinor(1),
            weight_bps: 10_000,
            rounding: "floor".into(),
        }
    }
}

/// Resolve policy from inline request material.
///
/// Batch 2 keeps the policy inline/dev-friendly, but all validation is centralized here so the
/// future registry-backed resolver can call the same checks.
pub fn resolve_reward_policy(
    inline: Option<RewardPolicy>,
    expected_id: &str,
    expected_hash: &str,
) -> Result<RewardPolicy> {
    let policy = inline.unwrap_or_else(|| {
        RewardPolicy::dev_default(expected_id.to_string(), expected_hash.to_string())
    });
    validate_reward_policy(&policy, expected_id, expected_hash)?;
    Ok(policy)
}

/// Validate a policy against the path/request expectation.
pub fn validate_reward_policy(
    policy: &RewardPolicy,
    expected_id: &str,
    expected_hash: &str,
) -> Result<()> {
    if expected_id.trim().is_empty() {
        return Err(RewarderError::BadRequest(
            "policy_id cannot be empty".into(),
        ));
    }
    if policy.id != expected_id {
        return Err(RewarderError::BadRequest(
            "policy object must match policy_id".into(),
        ));
    }
    if policy.hash != expected_hash {
        return Err(RewarderError::BadRequest(
            "policy object must match policy_hash".into(),
        ));
    }
    if !policy_hash_is_canonical(&policy.hash) {
        return Err(RewarderError::BadRequest(
            "policy_hash must be b3:<64 lowercase hex chars>".into(),
        ));
    }
    if policy.weight_bps == 0 {
        return Err(RewarderError::BadRequest(
            "policy weight_bps must be > 0".into(),
        ));
    }
    if policy.weight_bps > 100_000 {
        return Err(RewarderError::BadRequest(
            "policy weight_bps must be <= 100000".into(),
        ));
    }
    if policy.max_payout_minor_units < policy.min_payout_minor_units {
        return Err(RewarderError::BadRequest(
            "policy max_payout_minor_units must be >= min_payout_minor_units".into(),
        ));
    }
    if policy.rounding != "floor" {
        return Err(RewarderError::BadRequest(
            "batch 2 supports only rounding=floor".into(),
        ));
    }
    Ok(())
}

/// True when a policy hash is canonical `b3:<64 lowercase hex chars>`.
#[must_use]
pub fn policy_hash_is_canonical(hash: &str) -> bool {
    let Some(hex) = hash.strip_prefix("b3:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
