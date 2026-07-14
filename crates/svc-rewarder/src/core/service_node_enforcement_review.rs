//! RO:WHAT — Enforcement-aware Service Node reward review.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 19 requires contained nodes to earn no
//! rewards without allowing one bad candidate to suppress honest-node planning.
//!
//! RO:INTERACTS — canonical eligibility descriptors and enforcement statuses
//! from `ron-proto`, Phase 18 eligibility planning, and validated ROC economics.
//!
//! RO:INVARIANTS — candidate/descriptors match exactly; contained descriptors
//! require exact registry status; contained candidates receive no allocation;
//! pending appeals do not restore rewards; canonical ordering and hashes.
//!
//! RO:SECURITY — planning/review only; no registry, wallet, ledger, receipt,
//! payout, mint, burn, settlement, balance, or finality authority.
//!
//! RO:TEST —
//! `tests/internal_roc_beta_phase19_enforcement_reward_denial.rs`.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use ron_proto::{
    ContentId, ServiceNodeAppealStatusV1, ServiceNodeEligibilityStateV1,
    ServiceNodeEnforcementStatusV1, ServiceNodeIdentityDescriptorV1, ServiceNodeViolationKindV1,
};
use serde::{Deserialize, Serialize};

use super::service_node_plan::{
    validate_canonical_b3, validate_epoch_id, validate_service_node_id,
    MAX_SERVICE_NODE_REWARD_CANDIDATES,
};
use super::{
    compute_service_node_reward_plan_with_eligibility, ServiceNodeEligibilityRewardPlan,
    ServiceNodeRewardPlanInput,
};
use crate::inputs::InternalRocRewardPlanningEconomics;
use crate::{Result, RewarderError};

/// Canonical Phase 19 enforcement reward-review schema.
pub const SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_SCHEMA: &str =
    "ron.rewarder.service-node-enforcement-review.v1";

/// Canonical Phase 19 enforcement reward-review version.
pub const SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_VERSION: u16 = 1;

const INPUT_HASH_DOMAIN: &[u8] = b"svc-rewarder|service-node-enforcement-input|v1";

const ENFORCEMENT_HASH_DOMAIN: &[u8] = b"svc-rewarder|service-node-enforcement-status|v1";

const REVIEW_ID_DOMAIN: &[u8] = b"svc-rewarder|service-node-enforcement-review|v1";

/// One Service Node denied from reward planning by registry containment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardDenialV1 {
    /// Canonical Service Node identity.
    pub service_node_id: String,

    /// Registry lifecycle state that denied rewards.
    pub state: ServiceNodeEligibilityStateV1,

    /// Registry enforcement-status identity.
    pub status_id: String,

    /// Objective violation responsible for containment.
    pub violation: ServiceNodeViolationKindV1,

    /// Evidence root supporting containment.
    pub evidence_root: ContentId,

    /// Epoch in which containment became effective.
    pub effective_epoch: u64,

    /// Visible appeal posture; pending/accepted does not restore rewards.
    pub appeal: ServiceNodeAppealStatusV1,

    /// Number of accounting candidate rows denied for this node.
    pub candidate_rows: u64,
}

impl ServiceNodeRewardDenialV1 {
    /// Validate denial shape and containment posture.
    ///
    /// # Errors
    ///
    /// Returns [`RewarderError`] for malformed identity, non-containment state,
    /// empty status identity, invalid epoch/count, or invalid appeal posture.
    pub fn validate(&self) -> Result<()> {
        validate_service_node_id(&self.service_node_id)?;

        if !matches!(
            self.state,
            ServiceNodeEligibilityStateV1::Degraded
                | ServiceNodeEligibilityStateV1::Quarantined
                | ServiceNodeEligibilityStateV1::Blocked
        ) {
            return Err(RewarderError::BadRequest(
                "reward denial requires degraded, quarantined, or blocked state".into(),
            ));
        }

        if self.status_id.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "reward denial status_id must not be empty".into(),
            ));
        }

        if self.effective_epoch == 0 {
            return Err(RewarderError::BadRequest(
                "reward denial effective_epoch must be > 0".into(),
            ));
        }

        if self.candidate_rows == 0 {
            return Err(RewarderError::BadRequest(
                "reward denial candidate_rows must be > 0".into(),
            ));
        }

        self.appeal.validate().map_err(|error| {
            RewarderError::BadRequest(format!(
                "invalid Service Node reward-denial appeal: {error}"
            ))
        })
    }

    /// Denied candidates never authorize reward allocation.
    #[must_use]
    pub const fn permits_reward_planning(&self) -> bool {
        false
    }

    /// Denial records grant no economic mutation authority.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

/// Deterministic enforcement-aware reward review.
///
/// `plan` is `None` when every submitted candidate is contained. This is a
/// successful denial review, not an empty or fabricated reward plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeEnforcementRewardReview {
    /// Stable wrapper schema.
    pub schema: String,

    /// Stable wrapper version.
    pub version: u16,

    /// Deterministic identity of this complete review.
    pub review_id: String,

    /// Hash of the canonically ordered original candidate input.
    pub reward_input_hash: String,

    /// Hash of reviewed descriptors and enforcement statuses.
    pub enforcement_review_hash: String,

    /// Canonically sorted denied Service Nodes.
    pub denied_service_nodes: Vec<ServiceNodeRewardDenialV1>,

    /// Reward plan for remaining probation/eligible nodes.
    pub plan: Option<ServiceNodeEligibilityRewardPlan>,

    /// This artifact is review/planning material only.
    pub planning_only: bool,

    /// Registry-origin descriptors/statuses remain required.
    pub registry_attestation_required: bool,

    /// No payout authority exists here.
    pub payout_authority: bool,

    /// No payout execution occurred.
    pub payout_executed: bool,

    /// No wallet mutation occurred.
    pub wallet_mutation: bool,

    /// No ledger mutation occurred.
    pub ledger_mutation: bool,

    /// No receipt was created.
    pub receipt_created: bool,

    /// This artifact is not balance truth.
    pub balance_truth: bool,
}

impl ServiceNodeEnforcementRewardReview {
    /// Validate hashes, denial ordering, nested plan, and non-authority posture.
    ///
    /// # Errors
    ///
    /// Returns [`RewarderError`] when shape, ordering, nested allocation
    /// exclusion, identity, or authority invariants fail.
    pub fn validate(&self) -> Result<()> {
        validate_review_header(self)?;
        validate_denial_order(&self.denied_service_nodes)?;

        let denied_ids = self
            .denied_service_nodes
            .iter()
            .map(|denial| denial.service_node_id.as_str())
            .collect::<BTreeSet<_>>();

        if let Some(plan) = &self.plan {
            plan.validate()?;

            if plan
                .plan
                .allocations
                .iter()
                .any(|allocation| denied_ids.contains(allocation.service_node_id.as_str()))
            {
                return Err(RewarderError::Quarantined(
                    "contained Service Node escaped reward denial".into(),
                ));
            }
        } else if self.denied_service_nodes.is_empty() {
            return Err(RewarderError::BadRequest(
                "enforcement reward review requires a plan or denials".into(),
            ));
        }

        validate_non_authority(self)?;

        let expected = enforcement_reward_review_id(self)?;

        if expected != self.review_id {
            return Err(RewarderError::Quarantined(
                "Service Node enforcement reward-review identity mismatch".into(),
            ));
        }

        Ok(())
    }

    /// Whether all submitted candidate nodes were denied.
    #[must_use]
    pub const fn all_candidates_denied(&self) -> bool {
        self.plan.is_none()
    }

    /// Review output grants no economic mutation authority.
    #[must_use]
    pub const fn authorizes_economic_mutation(&self) -> bool {
        false
    }
}

/// Compute an enforcement-aware Service Node reward review.
///
/// Contained candidates are removed before Phase 18 reward planning and appear
/// in `denied_service_nodes`. Candidate lifecycle state still fails closed.
/// Pending or accepted appeals remain visible but do not restore rewards.
///
/// # Errors
///
/// Returns [`RewarderError`] for malformed candidate input, descriptor mismatch,
/// candidate state, missing/extra/mismatched enforcement status, hash failure,
/// nested reward-plan failure, or wrapper invariant failure.
pub fn compute_service_node_reward_review_with_enforcement(
    mut input: ServiceNodeRewardPlanInput,
    descriptors: Vec<ServiceNodeIdentityDescriptorV1>,
    statuses: Vec<ServiceNodeEnforcementStatusV1>,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<ServiceNodeEnforcementRewardReview> {
    validate_input(&mut input, economics)?;

    let reward_input_hash = hash_json(INPUT_HASH_DOMAIN, &input, "reward input")?;

    let descriptor_index = index_descriptors(descriptors, &input)?;

    let contained_ids = contained_service_node_ids(&descriptor_index);

    let status_index = index_statuses(statuses, &descriptor_index, &contained_ids)?;

    let enforcement_review_hash = hash_json(
        ENFORCEMENT_HASH_DOMAIN,
        &(
            descriptor_index.values().collect::<Vec<_>>(),
            status_index.values().collect::<Vec<_>>(),
        ),
        "enforcement review",
    )?;

    let denied_service_nodes = build_denials(&input, &contained_ids, &status_index)?;

    let allowed_ids = descriptor_index
        .iter()
        .filter_map(|(service_node_id, descriptor)| {
            matches!(
                descriptor.state,
                ServiceNodeEligibilityStateV1::Probation | ServiceNodeEligibilityStateV1::Eligible
            )
            .then_some(service_node_id.clone())
        })
        .collect::<BTreeSet<_>>();

    input
        .candidates
        .retain(|candidate| allowed_ids.contains(&candidate.service_node_id));

    let allowed_descriptors = descriptor_index
        .into_values()
        .filter(|descriptor| allowed_ids.contains(&descriptor.service_node_id))
        .collect::<Vec<_>>();

    let plan = if input.candidates.is_empty() {
        None
    } else {
        Some(compute_service_node_reward_plan_with_eligibility(
            input,
            allowed_descriptors,
            economics,
        )?)
    };

    let mut review = ServiceNodeEnforcementRewardReview {
        schema: SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_SCHEMA.to_string(),
        version: SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_VERSION,
        review_id: String::new(),
        reward_input_hash,
        enforcement_review_hash,
        denied_service_nodes,
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

    review.review_id = enforcement_reward_review_id(&review)?;

    review.validate()?;

    Ok(review)
}

fn validate_input(
    input: &mut ServiceNodeRewardPlanInput,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<()> {
    economics.validate_binding()?;

    validate_epoch_id(&input.epoch_id)?;
    validate_canonical_b3("accounting_snapshot_cid", &input.accounting_snapshot_cid)?;
    validate_canonical_b3("economics_config_hash", &input.economics_config_hash)?;
    validate_canonical_b3("policy_hash", &input.policy_hash)?;

    if input.economics_config_hash != economics.economics_config_hash {
        return Err(RewarderError::BadRequest(
            "service-node enforcement review economics hash mismatch".into(),
        ));
    }

    if input.pool_minor_units.get() == 0 {
        return Err(RewarderError::BadRequest(
            "service-node enforcement review pool must be > 0".into(),
        ));
    }

    if input.candidates.is_empty() {
        return Err(RewarderError::BadRequest(
            "service-node enforcement review requires candidates".into(),
        ));
    }

    if input.candidates.len() > MAX_SERVICE_NODE_REWARD_CANDIDATES {
        return Err(RewarderError::BadRequest(
            "service-node enforcement review exceeds candidate limit".into(),
        ));
    }

    input.candidates.sort_by(|left, right| {
        (
            left.service_node_id.as_str(),
            left.evidence_class,
            left.content_id.as_str(),
        )
            .cmp(&(
                right.service_node_id.as_str(),
                right.evidence_class,
                right.content_id.as_str(),
            ))
    });

    for candidate in &input.candidates {
        candidate.validate(economics)?;
    }

    Ok(())
}

fn index_descriptors(
    mut descriptors: Vec<ServiceNodeIdentityDescriptorV1>,
    input: &ServiceNodeRewardPlanInput,
) -> Result<BTreeMap<String, ServiceNodeIdentityDescriptorV1>> {
    descriptors.sort_by(|left, right| left.service_node_id.cmp(&right.service_node_id));

    let mut index = BTreeMap::new();

    for descriptor in descriptors {
        descriptor.validate().map_err(|error| {
            RewarderError::BadRequest(format!(
                "invalid Service Node enforcement descriptor: {error}"
            ))
        })?;

        if matches!(descriptor.state, ServiceNodeEligibilityStateV1::Candidate) {
            return Err(RewarderError::BadRequest(format!(
                "Service Node {} in Candidate state cannot enter enforcement reward review",
                descriptor.service_node_id
            )));
        }

        let service_node_id = descriptor.service_node_id.clone();

        if index.insert(service_node_id.clone(), descriptor).is_some() {
            return Err(RewarderError::BadRequest(format!(
                "duplicate Service Node enforcement descriptor: {service_node_id}"
            )));
        }
    }

    let candidate_ids = input
        .candidates
        .iter()
        .map(|candidate| candidate.service_node_id.clone())
        .collect::<BTreeSet<_>>();

    let descriptor_ids = index.keys().cloned().collect::<BTreeSet<_>>();

    if candidate_ids != descriptor_ids {
        return Err(RewarderError::BadRequest(
            "Service Node enforcement descriptor set mismatch".into(),
        ));
    }

    Ok(index)
}

fn contained_service_node_ids(
    descriptors: &BTreeMap<String, ServiceNodeIdentityDescriptorV1>,
) -> BTreeSet<String> {
    descriptors
        .iter()
        .filter_map(|(service_node_id, descriptor)| {
            matches!(
                descriptor.state,
                ServiceNodeEligibilityStateV1::Degraded
                    | ServiceNodeEligibilityStateV1::Quarantined
                    | ServiceNodeEligibilityStateV1::Blocked
            )
            .then_some(service_node_id.clone())
        })
        .collect()
}

fn index_statuses(
    mut statuses: Vec<ServiceNodeEnforcementStatusV1>,
    descriptors: &BTreeMap<String, ServiceNodeIdentityDescriptorV1>,
    contained_ids: &BTreeSet<String>,
) -> Result<BTreeMap<String, ServiceNodeEnforcementStatusV1>> {
    statuses.sort_by(|left, right| left.service_node_id.cmp(&right.service_node_id));

    let mut index = BTreeMap::new();

    for status in statuses {
        status.validate().map_err(|error| {
            RewarderError::BadRequest(format!("invalid Service Node enforcement status: {error}"))
        })?;

        let descriptor = descriptors.get(&status.service_node_id).ok_or_else(|| {
            RewarderError::BadRequest(format!(
                "enforcement status has no candidate descriptor: {}",
                status.service_node_id
            ))
        })?;

        if status.state != descriptor.state
            || status.effective_epoch != descriptor.state_effective_epoch
        {
            return Err(RewarderError::BadRequest(format!(
                "Service Node enforcement status does not match descriptor: {}",
                status.service_node_id
            )));
        }

        let service_node_id = status.service_node_id.clone();

        if index.insert(service_node_id.clone(), status).is_some() {
            return Err(RewarderError::BadRequest(format!(
                "duplicate Service Node enforcement status: {service_node_id}"
            )));
        }
    }

    let status_ids = index.keys().cloned().collect::<BTreeSet<_>>();

    if status_ids != *contained_ids {
        return Err(RewarderError::BadRequest(
            "Service Node enforcement status set mismatch".into(),
        ));
    }

    Ok(index)
}

fn build_denials(
    input: &ServiceNodeRewardPlanInput,
    contained_ids: &BTreeSet<String>,
    statuses: &BTreeMap<String, ServiceNodeEnforcementStatusV1>,
) -> Result<Vec<ServiceNodeRewardDenialV1>> {
    let mut denials = Vec::with_capacity(contained_ids.len());

    for service_node_id in contained_ids {
        let status = statuses.get(service_node_id).ok_or_else(|| {
            RewarderError::Internal("validated enforcement status disappeared".into())
        })?;

        let row_count = input
            .candidates
            .iter()
            .filter(|candidate| candidate.service_node_id == *service_node_id)
            .count();

        let candidate_rows = u64::try_from(row_count)
            .map_err(|_| RewarderError::Internal("denied candidate row count overflow".into()))?;

        let denial = ServiceNodeRewardDenialV1 {
            service_node_id: service_node_id.clone(),
            state: status.state,
            status_id: status.status_id.clone(),
            violation: status.reason,
            evidence_root: status.evidence_root.clone(),
            effective_epoch: status.effective_epoch,
            appeal: status.appeal.clone(),
            candidate_rows,
        };

        denial.validate()?;
        denials.push(denial);
    }

    Ok(denials)
}

fn validate_review_header(review: &ServiceNodeEnforcementRewardReview) -> Result<()> {
    if review.schema != SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_SCHEMA {
        return Err(RewarderError::BadRequest(
            "Service Node enforcement reward-review schema mismatch".into(),
        ));
    }

    if review.version != SERVICE_NODE_ENFORCEMENT_REWARD_REVIEW_VERSION {
        return Err(RewarderError::BadRequest(
            "Service Node enforcement reward-review version mismatch".into(),
        ));
    }

    validate_canonical_b3("review_id", &review.review_id)?;
    validate_canonical_b3("reward_input_hash", &review.reward_input_hash)?;
    validate_canonical_b3("enforcement_review_hash", &review.enforcement_review_hash)
}

fn validate_denial_order(denials: &[ServiceNodeRewardDenialV1]) -> Result<()> {
    let mut previous: Option<&str> = None;

    for denial in denials {
        denial.validate()?;

        if previous.is_some_and(|value| value >= denial.service_node_id.as_str()) {
            return Err(RewarderError::BadRequest(
                "Service Node reward denials must be sorted and unique".into(),
            ));
        }

        previous = Some(denial.service_node_id.as_str());
    }

    Ok(())
}

fn validate_non_authority(review: &ServiceNodeEnforcementRewardReview) -> Result<()> {
    if !review.planning_only
        || !review.registry_attestation_required
        || review.payout_authority
        || review.payout_executed
        || review.wallet_mutation
        || review.ledger_mutation
        || review.receipt_created
        || review.balance_truth
    {
        return Err(RewarderError::Quarantined(
            "Service Node enforcement reward review crossed authority boundary".into(),
        ));
    }

    Ok(())
}

fn enforcement_reward_review_id(review: &ServiceNodeEnforcementRewardReview) -> Result<String> {
    let nested_plan_id = review
        .plan
        .as_ref()
        .map(|plan| plan.eligibility_plan_id.as_str());

    hash_json(
        REVIEW_ID_DOMAIN,
        &(
            review.reward_input_hash.as_str(),
            review.enforcement_review_hash.as_str(),
            &review.denied_service_nodes,
            nested_plan_id,
        ),
        "enforcement reward-review identity",
    )
}

fn hash_json<T>(domain: &[u8], value: &T, label: &str) -> Result<String>
where
    T: Serialize,
{
    let encoded = serde_json::to_vec(value).map_err(|error| {
        RewarderError::Internal(format!("Service Node {label} encode failed: {error}"))
    })?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);

    Ok(format!("b3:{}", hasher.finalize().to_hex()))
}
