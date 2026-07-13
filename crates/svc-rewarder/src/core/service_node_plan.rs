//! RO:WHAT — Deterministic service-node reward planning bound to accounting identity.
//!
//! RO:WHY — Phase 14 reward plans must preserve `service_node_id`, derive
//! category from trusted evidence class, and apply economics caps without
//! accepting an arbitrary payout recipient.
//!
//! RO:INTERACTS — validated Internal ROC economics, accounting snapshot
//! identity, later registry recipient resolution, and Phase 14 exit tests.
//!
//! RO:INVARIANTS — integer-only checked math; canonical ordering; category,
//! account, content, and event caps; no wallet or ledger mutation.
//!
//! RO:SECURITY — inputs carry no payout recipient or client-selected amount.
//!
//! RO:TEST — internal_roc_beta_phase14d_service_node_reward_plan.rs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::core::{checked_mul_div_floor, AmountMinor};
use crate::inputs::InternalRocRewardPlanningEconomics;
use crate::{Result, RewarderError};

/// Canonical service-node reward-plan schema.
pub const SERVICE_NODE_REWARD_PLAN_SCHEMA: &str = "ron.rewarder.service-node-plan.v1";

/// Canonical service-node reward-plan version.
pub const SERVICE_NODE_REWARD_PLAN_VERSION: u16 = 1;

const MAX_SERVICE_NODE_REWARD_CANDIDATES: usize = 4_096;

/// Proof-eligible Service Node evidence classes accepted by reward
/// planning.
///
/// Policy refusal and moderation evidence are deliberately absent:
/// those classes remain review/challenge material rather than direct
/// reward candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceNodeRewardEvidenceClass {
    /// Verified content delivery evidence.
    Delivery,
    /// Verified availability evidence.
    Availability,
    /// Verified bounded range-request evidence.
    RangeRequest,
    /// Verified repair evidence.
    Repair,
    /// Verified hot-cache service evidence.
    HotCache,
}

impl ServiceNodeRewardEvidenceClass {
    /// Return the economics-owned category for proof-eligible Service
    /// Node work.
    #[must_use]
    pub const fn reward_category(self) -> &'static str {
        "node_delivery"
    }
}

/// One accounting-classified Service Node candidate.
///
/// The DTO intentionally has no payout recipient or requested reward
/// amount. Recipient resolution remains a later trusted registry step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardCandidate {
    /// Canonical Service Node identity retained from evidence.
    pub service_node_id: String,
    /// Proof-eligible evidence class.
    pub evidence_class: ServiceNodeRewardEvidenceClass,
    /// Canonical content identity retained from evidence.
    pub content_id: String,
    /// Number of accepted evidence events represented by this row.
    pub evidence_count: u64,
    /// Deterministic eligible-work score supplied by accounting.
    pub eligible_score: u128,
    /// Evidence signature/proof verification posture.
    pub evidence_verified: bool,
    /// Accounting acceptance posture.
    pub accounting_accepted: bool,
    /// Declarative ron-policy gate posture.
    pub policy_gate_passed: bool,
    /// Whether this row required challenge acceptance.
    pub challenge_required: bool,
    /// Whether a required challenge was accepted.
    pub challenge_accepted: bool,
}

impl ServiceNodeRewardCandidate {
    fn validate(&self, economics: &InternalRocRewardPlanningEconomics) -> Result<()> {
        validate_service_node_id(&self.service_node_id)?;
        validate_canonical_b3("content_id", &self.content_id)?;

        economics.category_cap(self.evidence_class.reward_category())?;

        if self.evidence_count == 0 {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate evidence_count must be > 0".into(),
            ));
        }

        if self.eligible_score == 0 {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate eligible_score must be > 0".into(),
            ));
        }

        if !self.evidence_verified {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate requires verified evidence".into(),
            ));
        }

        if !self.accounting_accepted {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate requires accounting acceptance".into(),
            ));
        }

        if !self.policy_gate_passed {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate requires ron-policy gate".into(),
            ));
        }

        if self.challenge_required && !self.challenge_accepted {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate requires accepted challenge".into(),
            ));
        }

        if !self.challenge_required && self.challenge_accepted {
            return Err(RewarderError::BadRequest(
                "service-node reward candidate cannot claim challenge acceptance when no challenge was required"
                    .into(),
            ));
        }

        Ok(())
    }
}

/// Input for one deterministic Service Node reward plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardPlanInput {
    /// Reward epoch identity.
    pub epoch_id: String,
    /// Canonical accounting snapshot artifact CID.
    pub accounting_snapshot_cid: String,
    /// Canonical economics identity selected for this plan.
    pub economics_config_hash: String,
    /// Canonical policy identity or approval reference.
    pub policy_hash: String,
    /// Pool made available by the accounting/policy boundary.
    pub pool_minor_units: AmountMinor,
    /// Accounting-classified Service Node candidates.
    pub candidates: Vec<ServiceNodeRewardCandidate>,
}

/// One deterministic, non-authoritative Service Node allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardAllocation {
    /// Service Node subject retained for later registry resolution.
    pub service_node_id: String,
    /// Evidence class used for this allocation.
    pub evidence_class: ServiceNodeRewardEvidenceClass,
    /// Economics-owned reward category.
    pub reward_category: String,
    /// Content identity used for per-content caps.
    pub content_id: String,
    /// Accepted events represented by the allocation.
    pub evidence_count: u64,
    /// Deterministic eligible-work score.
    pub eligible_score: u128,
    /// Planned ROC minor units.
    pub amount_minor_units: AmountMinor,
}

/// Deterministic plan totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardPlanTotals {
    /// Effective epoch pool.
    pub pool_minor_units: AmountMinor,
    /// Sum of planned allocations.
    pub allocated_minor_units: AmountMinor,
    /// Unallocated pool remainder.
    pub residual_minor_units: AmountMinor,
}

/// Identity-bound Service Node reward plan.
///
/// This is planning material only. It is not a wallet request, ledger
/// receipt, balance, or payout-execution record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceNodeRewardPlan {
    /// Stable schema.
    pub schema: String,
    /// Stable schema version.
    pub version: u16,
    /// Deterministic BLAKE3 identity of this plan.
    pub plan_id: String,
    /// Reward epoch identity.
    pub epoch_id: String,
    /// Accounting snapshot artifact CID.
    pub accounting_snapshot_cid: String,
    /// Economics config identity.
    pub economics_config_hash: String,
    /// Policy identity.
    pub policy_hash: String,
    /// Plan totals.
    pub totals: ServiceNodeRewardPlanTotals,
    /// Canonically ordered allocations.
    pub allocations: Vec<ServiceNodeRewardAllocation>,
    /// This artifact is planning-only.
    pub planning_only: bool,
    /// A trusted registry must later resolve node identity.
    pub registry_resolution_required: bool,
    /// Rewarder never grants payout authority.
    pub payout_authority: bool,
    /// Rewarder did not execute a payout.
    pub payout_executed: bool,
    /// Rewarder did not mutate a wallet.
    pub wallet_mutation: bool,
    /// Rewarder did not mutate the ledger.
    pub ledger_mutation: bool,
    /// Rewarder did not create a receipt.
    pub receipt_created: bool,
    /// Rewarder output is not balance truth.
    pub balance_truth: bool,
}

impl ServiceNodeRewardPlan {
    /// Validate plan conservation, canonical ordering, identity, and
    /// non-authority posture.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError` when the plan has malformed identity,
    /// broken conservation, noncanonical ordering, or authority flags.
    pub fn validate(&self) -> Result<()> {
        if self.schema != SERVICE_NODE_REWARD_PLAN_SCHEMA {
            return Err(RewarderError::BadRequest(
                "service-node reward plan schema mismatch".into(),
            ));
        }

        if self.version != SERVICE_NODE_REWARD_PLAN_VERSION {
            return Err(RewarderError::BadRequest(
                "service-node reward plan version mismatch".into(),
            ));
        }

        validate_epoch_id(&self.epoch_id)?;
        validate_canonical_b3("accounting_snapshot_cid", &self.accounting_snapshot_cid)?;
        validate_canonical_b3("economics_config_hash", &self.economics_config_hash)?;
        validate_canonical_b3("policy_hash", &self.policy_hash)?;
        validate_canonical_b3("plan_id", &self.plan_id)?;

        if !self.planning_only
            || !self.registry_resolution_required
            || self.payout_authority
            || self.payout_executed
            || self.wallet_mutation
            || self.ledger_mutation
            || self.receipt_created
            || self.balance_truth
        {
            return Err(RewarderError::BadRequest(
                "service-node reward plan must remain planning-only and non-authoritative".into(),
            ));
        }

        let mut previous_key: Option<(&str, ServiceNodeRewardEvidenceClass, &str)> = None;
        let mut allocated = AmountMinor::ZERO;

        for allocation in &self.allocations {
            validate_service_node_id(&allocation.service_node_id)?;
            validate_canonical_b3("allocation.content_id", &allocation.content_id)?;

            if allocation.reward_category != allocation.evidence_class.reward_category() {
                return Err(RewarderError::BadRequest(
                    "service-node allocation category does not match evidence class".into(),
                ));
            }

            if allocation.evidence_count == 0
                || allocation.eligible_score == 0
                || allocation.amount_minor_units.get() == 0
            {
                return Err(RewarderError::BadRequest(
                    "service-node allocation counters and amount must be > 0".into(),
                ));
            }

            let key = (
                allocation.service_node_id.as_str(),
                allocation.evidence_class,
                allocation.content_id.as_str(),
            );

            if previous_key.is_some_and(|previous| previous >= key) {
                return Err(RewarderError::BadRequest(
                    "service-node allocations must be sorted and unique".into(),
                ));
            }

            allocated = allocated.checked_add(allocation.amount_minor_units)?;
            previous_key = Some(key);
        }

        if allocated != self.totals.allocated_minor_units {
            return Err(RewarderError::Quarantined(
                "service-node allocation sum does not match plan total".into(),
            ));
        }

        let residual = self
            .totals
            .pool_minor_units
            .checked_sub(self.totals.allocated_minor_units)?;

        if residual != self.totals.residual_minor_units {
            return Err(RewarderError::Quarantined(
                "service-node reward plan residual is inconsistent".into(),
            ));
        }

        let expected_id = service_node_reward_plan_id(self)?;
        if expected_id != self.plan_id {
            return Err(RewarderError::Quarantined(
                "service-node reward plan identity mismatch".into(),
            ));
        }

        Ok(())
    }
}

/// Compute an identity-bound, category/account/content/event-capped
/// Service Node reward plan.
///
/// Category is derived from the accepted evidence-class enum. The
/// caller cannot supply a payout recipient, rate, amount, or category.
///
/// # Errors
///
/// Returns `RewarderError` for malformed identities, duplicate rows,
/// failed evidence/policy/challenge posture, event-cap violations,
/// arithmetic overflow, or broken output invariants.
pub fn compute_service_node_reward_plan(
    mut input: ServiceNodeRewardPlanInput,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<ServiceNodeRewardPlan> {
    economics.validate_binding()?;

    validate_epoch_id(&input.epoch_id)?;
    validate_canonical_b3("accounting_snapshot_cid", &input.accounting_snapshot_cid)?;
    validate_canonical_b3("economics_config_hash", &input.economics_config_hash)?;
    validate_canonical_b3("policy_hash", &input.policy_hash)?;

    if input.economics_config_hash != economics.economics_config_hash {
        return Err(RewarderError::BadRequest(
            "service-node plan economics_config_hash mismatch".into(),
        ));
    }

    if input.pool_minor_units.get() == 0 {
        return Err(RewarderError::BadRequest(
            "service-node reward plan pool must be > 0".into(),
        ));
    }

    if input.candidates.is_empty() {
        return Err(RewarderError::BadRequest(
            "service-node reward plan requires candidates".into(),
        ));
    }

    if input.candidates.len() > MAX_SERVICE_NODE_REWARD_CANDIDATES {
        return Err(RewarderError::BadRequest(format!(
            "service-node reward plan exceeds candidate limit: {}",
            MAX_SERVICE_NODE_REWARD_CANDIDATES
        )));
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

    let mut seen = BTreeSet::<(String, ServiceNodeRewardEvidenceClass, String)>::new();
    let mut node_event_counts = BTreeMap::<String, u64>::new();
    let mut category_scores = BTreeMap::<String, u128>::new();

    for candidate in &input.candidates {
        candidate.validate(economics)?;

        let key = (
            candidate.service_node_id.clone(),
            candidate.evidence_class,
            candidate.content_id.clone(),
        );

        if !seen.insert(key) {
            return Err(RewarderError::BadRequest(
                "duplicate service-node reward candidate".into(),
            ));
        }

        let node_events = node_event_counts
            .entry(candidate.service_node_id.clone())
            .or_insert(0);

        *node_events = node_events
            .checked_add(candidate.evidence_count)
            .ok_or_else(|| {
                RewarderError::Quarantined("service-node evidence count overflow".into())
            })?;

        if *node_events > economics.max_events_per_account_per_epoch {
            return Err(RewarderError::BadRequest(format!(
                "service node {} exceeds economics max events per account per epoch: {}",
                candidate.service_node_id, economics.max_events_per_account_per_epoch
            )));
        }

        let category = candidate.evidence_class.reward_category().to_owned();

        let category_score = category_scores.entry(category).or_insert(0);

        *category_score = category_score
            .checked_add(candidate.eligible_score)
            .ok_or_else(|| {
                RewarderError::Quarantined("service-node category score overflow".into())
            })?;
    }

    let pool = input.pool_minor_units.min(economics.epoch_pool_cap_minor);

    let mut category_pools = BTreeMap::<String, AmountMinor>::new();
    for category in category_scores.keys() {
        category_pools.insert(
            category.clone(),
            economics.effective_category_pool_cap(category, pool)?,
        );
    }

    let mut node_used = BTreeMap::<String, AmountMinor>::new();
    let mut content_used = BTreeMap::<String, AmountMinor>::new();
    let mut allocations = Vec::new();
    let mut allocated_total = AmountMinor::ZERO;

    for candidate in input.candidates {
        let category = candidate.evidence_class.reward_category();

        let category_pool = category_pools
            .get(category)
            .copied()
            .ok_or_else(|| RewarderError::Internal("missing validated category pool".into()))?;

        let category_score = category_scores
            .get(category)
            .copied()
            .ok_or_else(|| RewarderError::Internal("missing validated category score".into()))?;

        let proportional = checked_mul_div_floor(
            category_pool.get(),
            candidate.eligible_score,
            category_score,
        )?;

        let node_current = node_used
            .get(&candidate.service_node_id)
            .copied()
            .unwrap_or_default();

        let content_current = content_used
            .get(&candidate.content_id)
            .copied()
            .unwrap_or_default();

        let node_remaining = economics
            .max_reward_minor_per_account_per_epoch
            .checked_sub(node_current)?;

        let content_remaining = economics
            .max_reward_minor_per_content_per_epoch
            .checked_sub(content_current)?;

        let amount = AmountMinor(proportional)
            .min(node_remaining)
            .min(content_remaining);

        if amount.get() == 0 {
            continue;
        }

        let next_node = node_current.checked_add(amount)?;
        let next_content = content_current.checked_add(amount)?;
        allocated_total = allocated_total.checked_add(amount)?;

        node_used.insert(candidate.service_node_id.clone(), next_node);
        content_used.insert(candidate.content_id.clone(), next_content);

        allocations.push(ServiceNodeRewardAllocation {
            service_node_id: candidate.service_node_id,
            evidence_class: candidate.evidence_class,
            reward_category: category.to_owned(),
            content_id: candidate.content_id,
            evidence_count: candidate.evidence_count,
            eligible_score: candidate.eligible_score,
            amount_minor_units: amount,
        });
    }

    allocations.sort_by(|left, right| {
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

    let residual = pool.checked_sub(allocated_total)?;

    let mut plan = ServiceNodeRewardPlan {
        schema: SERVICE_NODE_REWARD_PLAN_SCHEMA.to_owned(),
        version: SERVICE_NODE_REWARD_PLAN_VERSION,
        plan_id: String::new(),
        epoch_id: input.epoch_id,
        accounting_snapshot_cid: input.accounting_snapshot_cid,
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: input.policy_hash,
        totals: ServiceNodeRewardPlanTotals {
            pool_minor_units: pool,
            allocated_minor_units: allocated_total,
            residual_minor_units: residual,
        },
        allocations,
        planning_only: true,
        registry_resolution_required: true,
        payout_authority: false,
        payout_executed: false,
        wallet_mutation: false,
        ledger_mutation: false,
        receipt_created: false,
        balance_truth: false,
    };

    plan.plan_id = service_node_reward_plan_id(&plan)?;
    plan.validate()?;

    Ok(plan)
}

fn service_node_reward_plan_id(plan: &ServiceNodeRewardPlan) -> Result<String> {
    let mut identity = plan.clone();
    identity.plan_id.clear();

    let bytes = serde_json::to_vec(&identity).map_err(|error| {
        RewarderError::Internal(format!(
            "service-node reward plan identity encode failed: {error}"
        ))
    })?;

    Ok(format!("b3:{}", blake3::hash(&bytes).to_hex()))
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

fn validate_epoch_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || value.trim() != value
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
    {
        return Err(RewarderError::BadRequest(
            "service-node reward epoch_id is invalid".into(),
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
