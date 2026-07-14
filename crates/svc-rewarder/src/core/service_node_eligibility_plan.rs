//! RO:WHAT — Registry-descriptor-bound probation reward-cap enforcement.
//!
//! RO:WHY — BUILD_PLAN_Z Phase 18 permits probation participation only under
//! an economics-owned reward ceiling and denies unsafe lifecycle states.
//!
//! RO:INTERACTS — canonical `ron-proto` Service Node descriptors, the existing
//! deterministic Service Node reward plan, and validated ROC economics.
//!
//! RO:INVARIANTS — exact descriptor/candidate set; only probation or eligible
//! nodes enter planning; probation totals are capped per node; canonical order;
//! no wallet, ledger, receipt, balance, mint, burn, or finality authority.
//!
//! RO:CONFIG — the probation cap comes exclusively from validated
//! `configs/roc-economics*.toml` projection.
//!
//! RO:SECURITY — descriptor shape and state are validated locally, while a
//! trusted registry attestation remains required before payout handoff.
//!
//! RO:TEST — `internal_roc_beta_phase18_probation_reward_cap.rs`.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use ron_proto::{ServiceNodeEligibilityStateV1, ServiceNodeIdentityDescriptorV1};
use serde::{Deserialize, Serialize};

use super::service_node_plan::service_node_reward_plan_id;
use super::{
    compute_service_node_reward_plan, AmountMinor, ServiceNodeRewardPlan,
    ServiceNodeRewardPlanInput,
};
use crate::inputs::InternalRocRewardPlanningEconomics;
use crate::{Result, RewarderError};

/// Eligibility-aware Service Node reward-plan schema.
pub const SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_SCHEMA: &str =
    "ron.rewarder.service-node-eligibility-plan.v1";

/// Eligibility-aware Service Node reward-plan version.
pub const SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_VERSION: u16 = 1;

const ELIGIBILITY_REVIEW_DOMAIN: &[u8] = b"svc-rewarder|service-node-eligibility-review|v1";

const ELIGIBILITY_PLAN_DOMAIN: &[u8] = b"svc-rewarder|service-node-eligibility-plan|v1";

/// Reward plan bound to canonical lifecycle descriptors.
///
/// The nested plan contains the actual capped allocations. This wrapper binds
/// those allocations to the reviewed lifecycle descriptor set and records the
/// economics-owned probation ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeEligibilityRewardPlan {
    /// Stable wrapper schema.
    pub schema: String,

    /// Stable wrapper version.
    pub version: u16,

    /// Deterministic identity of this complete wrapper.
    pub eligibility_plan_id: String,

    /// Deterministic identity of the sorted descriptor review.
    pub eligibility_review_hash: String,

    /// Configured probation ceiling applied per Service Node.
    pub probation_reward_cap_minor_per_node_per_epoch: AmountMinor,

    /// Canonically sorted probation Service Node identities.
    pub probation_service_node_ids: Vec<String>,

    /// Existing identity-bound reward plan after lifecycle enforcement.
    pub plan: ServiceNodeRewardPlan,

    /// This wrapper remains planning-only.
    pub planning_only: bool,

    /// Registry-origin attestation remains required before payout handoff.
    pub registry_attestation_required: bool,

    /// No payout authority is granted here.
    pub payout_authority: bool,

    /// No payout was executed here.
    pub payout_executed: bool,

    /// No wallet state was mutated here.
    pub wallet_mutation: bool,

    /// No ledger state was mutated here.
    pub ledger_mutation: bool,

    /// No durable receipt was created here.
    pub receipt_created: bool,

    /// This artifact is not balance truth.
    pub balance_truth: bool,
}

impl ServiceNodeEligibilityRewardPlan {
    /// Validate identity, ordering, cap enforcement, and non-authority posture.
    ///
    /// # Errors
    ///
    /// Returns [`RewarderError`] for malformed hashes, unsorted probation
    /// identities, cap violations, a broken nested plan, or authority flags.
    pub fn validate(&self) -> Result<()> {
        if self.schema != SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_SCHEMA {
            return Err(RewarderError::BadRequest(
                "Service Node eligibility reward-plan schema mismatch".into(),
            ));
        }

        if self.version != SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_VERSION {
            return Err(RewarderError::BadRequest(
                "Service Node eligibility reward-plan version mismatch".into(),
            ));
        }

        validate_canonical_b3("eligibility_plan_id", &self.eligibility_plan_id)?;
        validate_canonical_b3("eligibility_review_hash", &self.eligibility_review_hash)?;

        if self.probation_reward_cap_minor_per_node_per_epoch.get() == 0 {
            return Err(RewarderError::BadRequest(
                "probation Service Node reward cap must be > 0".into(),
            ));
        }

        let mut previous_node: Option<&str> = None;

        for service_node_id in &self.probation_service_node_ids {
            validate_service_node_id(service_node_id)?;

            if previous_node.is_some_and(|previous| previous >= service_node_id.as_str()) {
                return Err(RewarderError::BadRequest(
                    "probation Service Node identities must be sorted and unique".into(),
                ));
            }

            previous_node = Some(service_node_id);
        }

        if !self.planning_only
            || !self.registry_attestation_required
            || self.payout_authority
            || self.payout_executed
            || self.wallet_mutation
            || self.ledger_mutation
            || self.receipt_created
            || self.balance_truth
        {
            return Err(RewarderError::BadRequest(
                "eligibility reward plan must remain planning-only and non-authoritative".into(),
            ));
        }

        self.plan.validate()?;

        let probation_nodes = self
            .probation_service_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

        let mut probation_totals = BTreeMap::<&str, AmountMinor>::new();

        for allocation in &self.plan.allocations {
            if !probation_nodes.contains(allocation.service_node_id.as_str()) {
                continue;
            }

            let total = probation_totals
                .entry(allocation.service_node_id.as_str())
                .or_default();

            *total = total.checked_add(allocation.amount_minor_units)?;

            if *total > self.probation_reward_cap_minor_per_node_per_epoch {
                return Err(RewarderError::Quarantined(format!(
                    "probation reward cap exceeded for {}",
                    allocation.service_node_id
                )));
            }
        }

        let expected_id = eligibility_reward_plan_id(self)?;

        if expected_id != self.eligibility_plan_id {
            return Err(RewarderError::Quarantined(
                "Service Node eligibility reward-plan identity mismatch".into(),
            ));
        }

        Ok(())
    }
}

/// Compute a lifecycle-bound Service Node reward plan.
///
/// Every distinct candidate Service Node must have exactly one canonical
/// descriptor. `Probation` nodes enter under the configured probation cap,
/// `Eligible` nodes use the normal economics caps, and all other lifecycle
/// states fail closed.
///
/// # Errors
///
/// Returns [`RewarderError`] for malformed or duplicate descriptors, descriptor
/// set mismatch, a lifecycle state that cannot earn, base-plan failure, checked
/// arithmetic failure, or wrapper invariant failure.
pub fn compute_service_node_reward_plan_with_eligibility(
    input: ServiceNodeRewardPlanInput,
    mut descriptors: Vec<ServiceNodeIdentityDescriptorV1>,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<ServiceNodeEligibilityRewardPlan> {
    economics.validate_binding()?;

    let candidate_node_ids = input
        .candidates
        .iter()
        .map(|candidate| candidate.service_node_id.clone())
        .collect::<BTreeSet<_>>();

    descriptors.sort_by(|left, right| left.service_node_id.cmp(&right.service_node_id));

    let mut descriptor_node_ids = BTreeSet::new();
    let mut probation_node_ids = BTreeSet::new();

    for descriptor in &descriptors {
        descriptor.validate().map_err(|error| {
            RewarderError::BadRequest(format!(
                "invalid Service Node eligibility descriptor: {error}"
            ))
        })?;

        if !descriptor_node_ids.insert(descriptor.service_node_id.clone()) {
            return Err(RewarderError::BadRequest(format!(
                "duplicate Service Node eligibility descriptor: {}",
                descriptor.service_node_id
            )));
        }

        if descriptor.state == ServiceNodeEligibilityStateV1::Probation {
            probation_node_ids.insert(descriptor.service_node_id.clone());
        } else if descriptor.state != ServiceNodeEligibilityStateV1::Eligible {
            return Err(RewarderError::BadRequest(format!(
                "Service Node {} in state {:?} cannot enter reward planning",
                descriptor.service_node_id, descriptor.state
            )));
        }
    }

    if candidate_node_ids != descriptor_node_ids {
        return Err(RewarderError::BadRequest(format!(
            "Service Node eligibility descriptor set mismatch: candidates={}, descriptors={}",
            candidate_node_ids.len(),
            descriptor_node_ids.len()
        )));
    }

    let eligibility_review_hash = eligibility_review_hash(&descriptors)?;

    let mut plan = compute_service_node_reward_plan(input, economics)?;

    apply_probation_caps(
        &mut plan,
        &probation_node_ids,
        economics.probation_reward_cap_minor_per_node_per_epoch,
    )?;

    let mut result = ServiceNodeEligibilityRewardPlan {
        schema: SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_SCHEMA.to_owned(),
        version: SERVICE_NODE_ELIGIBILITY_REWARD_PLAN_VERSION,
        eligibility_plan_id: String::new(),
        eligibility_review_hash,
        probation_reward_cap_minor_per_node_per_epoch: economics
            .probation_reward_cap_minor_per_node_per_epoch,
        probation_service_node_ids: probation_node_ids.into_iter().collect(),
        plan,
        planning_only: true,
        registry_attestation_required: true,
        payout_authority: false,
        payout_executed: false,
        wallet_mutation: false,
        ledger_mutation: false,
        receipt_created: false,
        balance_truth: false,
    };

    result.eligibility_plan_id = eligibility_reward_plan_id(&result)?;
    result.validate()?;

    Ok(result)
}

fn apply_probation_caps(
    plan: &mut ServiceNodeRewardPlan,
    probation_node_ids: &BTreeSet<String>,
    cap: AmountMinor,
) -> Result<()> {
    let mut node_used = BTreeMap::<String, AmountMinor>::new();
    let mut allocations = Vec::with_capacity(plan.allocations.len());
    let mut allocated_total = AmountMinor::ZERO;

    for mut allocation in std::mem::take(&mut plan.allocations) {
        if probation_node_ids.contains(&allocation.service_node_id) {
            let current = node_used
                .get(&allocation.service_node_id)
                .copied()
                .unwrap_or_default();

            let remaining = cap.checked_sub(current)?;
            allocation.amount_minor_units = allocation.amount_minor_units.min(remaining);

            if allocation.amount_minor_units.get() == 0 {
                continue;
            }

            let next = current.checked_add(allocation.amount_minor_units)?;

            node_used.insert(allocation.service_node_id.clone(), next);
        }

        allocated_total = allocated_total.checked_add(allocation.amount_minor_units)?;

        allocations.push(allocation);
    }

    plan.allocations = allocations;
    plan.totals.allocated_minor_units = allocated_total;
    plan.totals.residual_minor_units = plan.totals.pool_minor_units.checked_sub(allocated_total)?;
    plan.plan_id = service_node_reward_plan_id(plan)?;

    plan.validate()
}

fn eligibility_review_hash(descriptors: &[ServiceNodeIdentityDescriptorV1]) -> Result<String> {
    let encoded = serde_json::to_vec(descriptors).map_err(|error| {
        RewarderError::Internal(format!(
            "Service Node eligibility review encode failed: {error}"
        ))
    })?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(ELIGIBILITY_REVIEW_DOMAIN);
    hasher.update(&encoded);

    Ok(format!("b3:{}", hasher.finalize().to_hex()))
}

fn eligibility_reward_plan_id(plan: &ServiceNodeEligibilityRewardPlan) -> Result<String> {
    let mut identity = plan.clone();
    identity.eligibility_plan_id.clear();

    let encoded = serde_json::to_vec(&identity).map_err(|error| {
        RewarderError::Internal(format!(
            "Service Node eligibility reward-plan identity encode failed: {error}"
        ))
    })?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(ELIGIBILITY_PLAN_DOMAIN);
    hasher.update(&encoded);

    Ok(format!("b3:{}", hasher.finalize().to_hex()))
}

fn validate_service_node_id(value: &str) -> Result<()> {
    if !value.starts_with("service_node:")
        || value.len() <= "service_node:".len()
        || value.len() > 128
        || value.trim() != value
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.')
        })
    {
        return Err(RewarderError::BadRequest(
            "service_node_id must be canonical service_node:<id>".into(),
        ));
    }

    Ok(())
}

fn validate_canonical_b3(field: &str, value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(RewarderError::BadRequest(format!(
            "{field} must be b3:<64 lowercase hex chars>"
        )));
    };

    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(RewarderError::BadRequest(format!(
            "{field} must be b3:<64 lowercase hex chars>"
        )));
    }

    Ok(())
}
