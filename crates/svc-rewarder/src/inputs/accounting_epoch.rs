//! RO:WHAT — Canonical ron-accounting epoch-snapshot handoff into Phase 14 reward planning.
//!
//! RO:WHY — Reward planning must consume the deterministic accounting artifact
//! instead of accepting free-form node identities, scores, content IDs, economics
//! hashes, or payout recipients.
//!
//! RO:INTERACTS — ron-accounting epoch snapshots, validated ron-policy economics,
//! signed reward policy, Service Node reward plans, and User Node planned points.
//!
//! RO:INVARIANTS — snapshot CID and economics identity match; one accepted
//! accounting row contributes one neutral planning point; event counts are capped;
//! no recipient, amount, wallet, ledger, receipt, balance, or finality authority.
//!
//! RO:SECURITY — policy/moderation-only Service Node evidence cannot enter reward
//! planning; User Node points assign no ROC amount or recipient.
//!
//! RO:TEST — internal_roc_beta_phase14d_accounting_epoch_handoff.rs.

use std::collections::BTreeMap;

use ron_accounting::accounting::epoch_snapshot::{
    canonical_accounting_epoch_snapshot_artifact_cid, AccountingEconomicsProfileV1,
    AccountingEpochSnapshotV1,
};
use ron_accounting::accounting::node_evidence_wire::{
    ServiceEvidenceAccountingKindV1, UserVerificationEvidenceKindV1,
};
use serde::{Deserialize, Serialize};

use crate::core::{
    AmountMinor, ServiceNodeRewardCandidate, ServiceNodeRewardEvidenceClass,
    ServiceNodeRewardPlanInput,
};
use crate::inputs::{
    validate_reward_policy, InternalRocRewardPlanningEconomics, RewardFundingSource, RewardPolicy,
};
use crate::{Result, RewarderError};

/// Canonical accounting-to-rewarder handoff schema.
pub const ACCOUNTING_EPOCH_REWARD_HANDOFF_SCHEMA: &str = "ron.rewarder.accounting-epoch-handoff.v1";

/// Canonical accounting-to-rewarder handoff version.
pub const ACCOUNTING_EPOCH_REWARD_HANDOFF_VERSION: u16 = 1;

/// Canonical User Node verification point-plan schema.
pub const USER_VERIFICATION_POINT_PLAN_SCHEMA: &str =
    "ron.rewarder.user-verification-point-plan.v1";

/// Canonical User Node verification point-plan version.
pub const USER_VERIFICATION_POINT_PLAN_VERSION: u16 = 1;

/// One proof-eligible User Node verification planning point.
///
/// A point records one accepted accounting row. It does not assign a
/// ROC amount or select a payout recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationPlannedPointV1 {
    /// Accounting sequence retained for audit ordering.
    pub sequence: u64,
    /// User Node identity retained from accounting.
    pub user_node_id: String,
    /// Verified work class retained from accounting.
    pub verification_kind: UserVerificationEvidenceKindV1,
    /// Evidence identity retained from accounting.
    pub evidence_id: String,
    /// Subject reviewed by this verification.
    pub subject_ref: String,
    /// Canonical digest of the verified material.
    pub input_digest: String,
    /// Neutral accepted-row count. Always one for a snapshot row.
    pub planned_points: u64,
}

/// Deterministic User Node verification point plan.
///
/// Monetary rates for these points remain future economics
/// configuration. This artifact does not assign ROC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationPointPlanV1 {
    /// Stable schema.
    pub schema: String,
    /// Stable schema version.
    pub version: u16,
    /// Deterministic BLAKE3 identity of this point plan.
    pub plan_id: String,
    /// Accounting snapshot artifact CID.
    pub accounting_snapshot_cid: String,
    /// Economics configuration identity.
    pub economics_config_hash: String,
    /// Canonically ordered proof-eligible points.
    pub points: Vec<UserVerificationPlannedPointV1>,
    /// Sum of neutral planned points.
    pub total_planned_points: u64,
    /// Canonical ordering posture.
    pub deterministic_order: bool,
    /// Planning-only posture.
    pub planning_only: bool,
    /// No ROC amount was assigned.
    pub reward_amount_assigned: bool,
    /// No payout authority exists.
    pub payout_authority: bool,
    /// No wallet mutation occurred.
    pub wallet_mutation: bool,
    /// No ledger mutation occurred.
    pub ledger_mutation: bool,
}

impl UserVerificationPointPlanV1 {
    /// Validate point totals, canonical ordering, identity, and
    /// non-authority posture.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError` when any point, total, plan identity,
    /// ordering rule, or non-authority flag is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.schema != USER_VERIFICATION_POINT_PLAN_SCHEMA {
            return Err(RewarderError::BadRequest(
                "user verification point-plan schema mismatch".into(),
            ));
        }

        if self.version != USER_VERIFICATION_POINT_PLAN_VERSION {
            return Err(RewarderError::BadRequest(
                "user verification point-plan version mismatch".into(),
            ));
        }

        validate_canonical_b3("user point-plan plan_id", &self.plan_id)?;
        validate_canonical_b3(
            "user point-plan accounting_snapshot_cid",
            &self.accounting_snapshot_cid,
        )?;
        validate_canonical_b3(
            "user point-plan economics_config_hash",
            &self.economics_config_hash,
        )?;

        if !self.deterministic_order
            || !self.planning_only
            || self.reward_amount_assigned
            || self.payout_authority
            || self.wallet_mutation
            || self.ledger_mutation
        {
            return Err(RewarderError::BadRequest(
                "user verification point plan must remain deterministic, planning-only, and non-authoritative"
                    .into(),
            ));
        }

        let mut previous_key: Option<(&str, UserVerificationEvidenceKindV1, &str, u64)> = None;

        let mut total = 0_u64;

        for point in &self.points {
            validate_user_node_id(&point.user_node_id)?;
            validate_token("user verification evidence_id", &point.evidence_id)?;
            validate_token("user verification subject_ref", &point.subject_ref)?;
            validate_canonical_b3("user verification input_digest", &point.input_digest)?;

            if point.sequence == 0 {
                return Err(RewarderError::BadRequest(
                    "user verification sequence must be > 0".into(),
                ));
            }

            if point.planned_points != 1 {
                return Err(RewarderError::BadRequest(
                    "each accepted user verification snapshot row must contribute exactly one neutral planning point"
                        .into(),
                ));
            }

            let key = (
                point.user_node_id.as_str(),
                point.verification_kind,
                point.evidence_id.as_str(),
                point.sequence,
            );

            if previous_key.is_some_and(|previous| previous >= key) {
                return Err(RewarderError::BadRequest(
                    "user verification points must be strictly canonical and unique".into(),
                ));
            }

            total = total.checked_add(point.planned_points).ok_or_else(|| {
                RewarderError::Quarantined("user verification point total overflow".into())
            })?;

            previous_key = Some(key);
        }

        if total != self.total_planned_points {
            return Err(RewarderError::Quarantined(
                "user verification planned-point total mismatch".into(),
            ));
        }

        let expected_id = user_verification_point_plan_id(self)?;

        if expected_id != self.plan_id {
            return Err(RewarderError::Quarantined(
                "user verification point-plan identity mismatch".into(),
            ));
        }

        Ok(())
    }
}

/// Canonical output of the accounting-to-rewarder handoff.
///
/// Service Node rows become recipient-free capped-plan inputs. User
/// Node verification rows become neutral planned points awaiting an
/// economics-configured monetary rate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingEpochRewardPlanningHandoffV1 {
    /// Stable schema.
    pub schema: String,
    /// Stable schema version.
    pub version: u16,
    /// Accounting snapshot artifact CID.
    pub accounting_snapshot_cid: String,
    /// Economics configuration identity.
    pub economics_config_hash: String,
    /// Validated policy identity.
    pub policy_hash: String,
    /// Service Node planning input, when eligible service rows exist.
    pub service_node_plan_input: Option<ServiceNodeRewardPlanInput>,
    /// User Node verification neutral-point plan.
    pub user_verification_point_plan: UserVerificationPointPlanV1,
    /// Accounting artifact was validated and canonically hashed.
    pub accounting_snapshot_verified: bool,
    /// Signed policy validation passed.
    pub policy_gate_passed: bool,
    /// Handoff remains planning-only.
    pub planning_only: bool,
    /// No payout authority exists.
    pub payout_authority: bool,
    /// No wallet mutation occurred.
    pub wallet_mutation: bool,
    /// No ledger mutation occurred.
    pub ledger_mutation: bool,
}

impl AccountingEpochRewardPlanningHandoffV1 {
    /// Validate cross-artifact identity and non-authority posture.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError` when accounting, economics, policy,
    /// service-plan, User Node point-plan, or authority posture is
    /// inconsistent.
    pub fn validate(&self) -> Result<()> {
        if self.schema != ACCOUNTING_EPOCH_REWARD_HANDOFF_SCHEMA {
            return Err(RewarderError::BadRequest(
                "accounting reward handoff schema mismatch".into(),
            ));
        }

        if self.version != ACCOUNTING_EPOCH_REWARD_HANDOFF_VERSION {
            return Err(RewarderError::BadRequest(
                "accounting reward handoff version mismatch".into(),
            ));
        }

        validate_canonical_b3("accounting_snapshot_cid", &self.accounting_snapshot_cid)?;
        validate_canonical_b3("economics_config_hash", &self.economics_config_hash)?;
        validate_canonical_b3("policy_hash", &self.policy_hash)?;

        if !self.accounting_snapshot_verified
            || !self.policy_gate_passed
            || !self.planning_only
            || self.payout_authority
            || self.wallet_mutation
            || self.ledger_mutation
        {
            return Err(RewarderError::BadRequest(
                "accounting reward handoff must remain verified, policy-gated, planning-only, and non-authoritative"
                    .into(),
            ));
        }

        self.user_verification_point_plan.validate()?;

        if self.user_verification_point_plan.accounting_snapshot_cid != self.accounting_snapshot_cid
            || self.user_verification_point_plan.economics_config_hash != self.economics_config_hash
        {
            return Err(RewarderError::BadRequest(
                "user verification point plan is not bound to the handoff identities".into(),
            ));
        }

        if let Some(service_input) = &self.service_node_plan_input {
            if service_input.candidates.is_empty() {
                return Err(RewarderError::BadRequest(
                    "service-node plan input must not be empty".into(),
                ));
            }

            if service_input.accounting_snapshot_cid != self.accounting_snapshot_cid
                || service_input.economics_config_hash != self.economics_config_hash
                || service_input.policy_hash != self.policy_hash
            {
                return Err(RewarderError::BadRequest(
                    "service-node plan input is not bound to the handoff identities".into(),
                ));
            }
        }

        if self.service_node_plan_input.is_none()
            && self.user_verification_point_plan.points.is_empty()
        {
            return Err(RewarderError::BadRequest(
                "accounting reward handoff requires eligible planning material".into(),
            ));
        }

        Ok(())
    }
}

/// Convert a canonical accounting epoch snapshot into reward-planning
/// inputs.
///
/// Service evidence is aggregated by node, evidence class, and
/// content. Each accepted accounting row contributes one neutral
/// eligible-work point and one event. User verification rows become
/// point-only planning material and receive no ROC amount.
///
/// # Errors
///
/// Returns `RewarderError` for malformed accounting artifacts,
/// economics/policy mismatches, policy-only evidence leakage,
/// event-cap violations, arithmetic overflow, or authority-bearing
/// material.
pub fn accounting_epoch_reward_planning_handoff(
    snapshot: &AccountingEpochSnapshotV1,
    policy: &RewardPolicy,
    available_pool: AmountMinor,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<AccountingEpochRewardPlanningHandoffV1> {
    economics.validate_binding()?;

    validate_reward_policy(policy, &policy.id, &policy.hash)?;

    if policy.funding_source != RewardFundingSource::ProtocolPool {
        return Err(RewarderError::BadRequest(
            "Phase 14 node reward planning requires protocol_pool funding".into(),
        ));
    }

    if available_pool.get() == 0 {
        return Err(RewarderError::BadRequest(
            "accounting reward handoff pool must be > 0".into(),
        ));
    }

    let snapshot = snapshot.canonicalized().map_err(|error| {
        RewarderError::BadRequest(format!("invalid accounting epoch snapshot: {error}"))
    })?;

    validate_accounting_economics_binding(&snapshot, economics)?;

    let accounting_snapshot_cid = canonical_accounting_epoch_snapshot_artifact_cid(&snapshot)
        .map_err(|error| {
            RewarderError::BadRequest(format!("accounting snapshot CID failed: {error}"))
        })?;

    let authorized_pool = available_pool
        .min(policy.max_payout_minor_units)
        .min(economics.epoch_pool_cap_minor);

    if authorized_pool.get() == 0 {
        return Err(RewarderError::BadRequest(
            "authorized accounting reward pool is zero".into(),
        ));
    }

    let service_candidates = service_candidates_from_snapshot(&snapshot, economics)?;

    let service_node_plan_input = if service_candidates.is_empty() {
        None
    } else {
        Some(ServiceNodeRewardPlanInput {
            epoch_id: snapshot.epoch_id.clone(),
            accounting_snapshot_cid: accounting_snapshot_cid.clone(),
            economics_config_hash: economics.economics_config_hash.clone(),
            policy_hash: policy.hash.clone(),
            pool_minor_units: authorized_pool,
            candidates: service_candidates,
        })
    };

    let user_verification_point_plan =
        user_verification_points_from_snapshot(&snapshot, &accounting_snapshot_cid, economics)?;

    let handoff = AccountingEpochRewardPlanningHandoffV1 {
        schema: ACCOUNTING_EPOCH_REWARD_HANDOFF_SCHEMA.to_owned(),
        version: ACCOUNTING_EPOCH_REWARD_HANDOFF_VERSION,
        accounting_snapshot_cid,
        economics_config_hash: economics.economics_config_hash.clone(),
        policy_hash: policy.hash.clone(),
        service_node_plan_input,
        user_verification_point_plan,
        accounting_snapshot_verified: true,
        policy_gate_passed: true,
        planning_only: true,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    };

    handoff.validate()?;
    Ok(handoff)
}

fn validate_accounting_economics_binding(
    snapshot: &AccountingEpochSnapshotV1,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<()> {
    let expected_profile = match snapshot.economics_config.profile {
        AccountingEconomicsProfileV1::Canonical => "canonical",
        AccountingEconomicsProfileV1::Development => "development",
    };

    if snapshot.economics_config.config_schema != economics.schema
        || snapshot.economics_config.config_version != economics.version
        || snapshot.economics_config.economics_config_hash.as_str()
            != economics.economics_config_hash
        || expected_profile != economics.profile
    {
        return Err(RewarderError::BadRequest(
            "accounting snapshot economics binding does not match selected rewarder economics"
                .into(),
        ));
    }

    Ok(())
}

fn service_candidates_from_snapshot(
    snapshot: &AccountingEpochSnapshotV1,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<Vec<ServiceNodeRewardCandidate>> {
    let mut node_event_counts = BTreeMap::<String, u64>::new();

    let mut aggregated =
        BTreeMap::<(String, ServiceNodeRewardEvidenceClass, String), (u64, u128)>::new();

    for row in &snapshot.service_rows {
        let evidence_class = service_reward_class(row.kind)?;

        let node_events = node_event_counts
            .entry(row.service_node_id.clone())
            .or_insert(0);

        *node_events = node_events.checked_add(1).ok_or_else(|| {
            RewarderError::Quarantined("service-node snapshot event-count overflow".into())
        })?;

        if *node_events > economics.max_events_per_account_per_epoch {
            return Err(RewarderError::BadRequest(format!(
                "service node {} exceeds economics max events per account per epoch: {}",
                row.service_node_id, economics.max_events_per_account_per_epoch
            )));
        }

        let entry = aggregated
            .entry((
                row.service_node_id.clone(),
                evidence_class,
                row.content_id.as_str().to_owned(),
            ))
            .or_insert((0, 0));

        entry.0 = entry.0.checked_add(1).ok_or_else(|| {
            RewarderError::Quarantined("service-node evidence-count aggregation overflow".into())
        })?;

        entry.1 = entry.1.checked_add(1).ok_or_else(|| {
            RewarderError::Quarantined("service-node neutral-score aggregation overflow".into())
        })?;
    }

    Ok(aggregated
        .into_iter()
        .map(
            |((service_node_id, evidence_class, content_id), (evidence_count, eligible_score))| {
                ServiceNodeRewardCandidate {
                    service_node_id,
                    evidence_class,
                    content_id,
                    evidence_count,
                    eligible_score,
                    evidence_verified: true,
                    accounting_accepted: true,
                    policy_gate_passed: true,
                    challenge_required: false,
                    challenge_accepted: false,
                }
            },
        )
        .collect())
}

fn user_verification_points_from_snapshot(
    snapshot: &AccountingEpochSnapshotV1,
    accounting_snapshot_cid: &str,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<UserVerificationPointPlanV1> {
    let mut user_event_counts = BTreeMap::<String, u64>::new();
    let mut points = Vec::with_capacity(snapshot.user_verification_rows.len());

    for row in &snapshot.user_verification_rows {
        let event_count = user_event_counts
            .entry(row.user_node_id.clone())
            .or_insert(0);

        *event_count = event_count.checked_add(1).ok_or_else(|| {
            RewarderError::Quarantined("user verification event-count overflow".into())
        })?;

        if *event_count > economics.max_events_per_account_per_epoch {
            return Err(RewarderError::BadRequest(format!(
                "user node {} exceeds economics max events per account per epoch: {}",
                row.user_node_id, economics.max_events_per_account_per_epoch
            )));
        }

        points.push(UserVerificationPlannedPointV1 {
            sequence: row.sequence,
            user_node_id: row.user_node_id.clone(),
            verification_kind: row.verification_kind,
            evidence_id: row.evidence_id.clone(),
            subject_ref: row.subject_ref.clone(),
            input_digest: row.input_digest.as_str().to_owned(),
            planned_points: 1,
        });
    }

    points.sort_by(|left, right| {
        (
            left.user_node_id.as_str(),
            left.verification_kind,
            left.evidence_id.as_str(),
            left.sequence,
        )
            .cmp(&(
                right.user_node_id.as_str(),
                right.verification_kind,
                right.evidence_id.as_str(),
                right.sequence,
            ))
    });

    let total_planned_points = points.iter().try_fold(0_u64, |total, point| {
        total.checked_add(point.planned_points).ok_or_else(|| {
            RewarderError::Quarantined("user verification point total overflow".into())
        })
    })?;

    let mut plan = UserVerificationPointPlanV1 {
        schema: USER_VERIFICATION_POINT_PLAN_SCHEMA.to_owned(),
        version: USER_VERIFICATION_POINT_PLAN_VERSION,
        plan_id: String::new(),
        accounting_snapshot_cid: accounting_snapshot_cid.to_owned(),
        economics_config_hash: economics.economics_config_hash.clone(),
        points,
        total_planned_points,
        deterministic_order: true,
        planning_only: true,
        reward_amount_assigned: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    };

    plan.plan_id = user_verification_point_plan_id(&plan)?;
    plan.validate()?;

    Ok(plan)
}

fn service_reward_class(
    kind: ServiceEvidenceAccountingKindV1,
) -> Result<ServiceNodeRewardEvidenceClass> {
    match kind {
        ServiceEvidenceAccountingKindV1::Delivery => Ok(ServiceNodeRewardEvidenceClass::Delivery),
        ServiceEvidenceAccountingKindV1::Availability => {
            Ok(ServiceNodeRewardEvidenceClass::Availability)
        }
        ServiceEvidenceAccountingKindV1::RangeRequest => {
            Ok(ServiceNodeRewardEvidenceClass::RangeRequest)
        }
        ServiceEvidenceAccountingKindV1::Repair => Ok(ServiceNodeRewardEvidenceClass::Repair),
        ServiceEvidenceAccountingKindV1::HotCache => Ok(ServiceNodeRewardEvidenceClass::HotCache),
        ServiceEvidenceAccountingKindV1::PolicyRefusal
        | ServiceEvidenceAccountingKindV1::ModerationAction => Err(RewarderError::BadRequest(
            "policy-refusal or moderation-only service evidence must not enter reward planning"
                .into(),
        )),
    }
}

fn user_verification_point_plan_id(plan: &UserVerificationPointPlanV1) -> Result<String> {
    let mut identity = plan.clone();
    identity.plan_id.clear();

    let bytes = serde_json::to_vec(&identity).map_err(|error| {
        RewarderError::Internal(format!(
            "user verification point-plan identity encode failed: {error}"
        ))
    })?;

    Ok(format!("b3:{}", blake3::hash(&bytes).to_hex()))
}

fn validate_user_node_id(value: &str) -> Result<()> {
    if !value.starts_with("user_node:") || value.len() <= "user_node:".len() {
        return Err(RewarderError::BadRequest(
            "user_node_id must be canonical user_node:<id>".into(),
        ));
    }

    validate_token("user_node_id", value)
}

fn validate_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 512
        || value.trim() != value
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        })
    {
        return Err(RewarderError::BadRequest(format!(
            "{field} is not a canonical bounded token"
        )));
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
