//! RO:WHAT — Deterministic accounting classification for User Node verification.
//! RO:WHY — Phase 14 requires verified User Node work before capped planning.
//! RO:INTERACTS — strict verification accounting input and later snapshots.
//! RO:INVARIANTS — deterministic order; challenge work remains separately gated.
//! RO:SECURITY — no amount, payout, receipt, balance, wallet, or ledger authority.
//! RO:TEST — tests/user_verification_classification.rs.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use super::node_evidence_wire::{
    EvidenceContentId, UserVerificationAccountingInputV1, UserVerificationEvidenceKindV1,
    UserVerificationResultV1,
};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const USER_VERIFICATION_CLASSIFICATION_BATCH_SCHEMA: &str =
    "ron.accounting.user-verification-classification-batch.v1";

pub const USER_VERIFICATION_CLASSIFICATION_BATCH_VERSION: u16 = 1;

pub const MAX_USER_VERIFICATION_CLASSIFICATION_ITEMS: usize = 4_096;

/// Accounting class assigned to valid User Node verification evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserVerificationAccountingClassV1 {
    /// Verified-valid material may enter later capped reward planning.
    ProofEligibleVerification,

    /// Invalid/challenge findings require later challenge acceptance.
    ChallengeEvidenceOnly,
}

/// Deterministic accounting decision for one User Node evidence item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationAccountingDecisionV1 {
    pub sequence: u64,
    pub evidence_id: String,
    pub user_node_id: String,
    pub subject_ref: String,

    pub verification_kind: UserVerificationEvidenceKindV1,

    pub result: UserVerificationResultV1,
    pub input_digest: EvidenceContentId,
    pub observed_at_ms: u64,

    pub class: UserVerificationAccountingClassV1,

    pub verified_evidence: bool,
    pub requires_policy_review: bool,
    pub requires_economics_config: bool,
    pub requires_cap: bool,
    pub requires_challenge_acceptance: bool,

    /// Candidate status only; no points or amount are assigned here.
    pub reward_planning_candidate: bool,

    pub accounting_snapshot_member: bool,
    pub reward_plan_created: bool,
    pub direct_protocol_roc_allocation: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Deterministically ordered batch of User Node decisions.
///
/// This is classification output only, not a snapshot root, reward plan,
/// payout, balance, or receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserVerificationClassificationBatchV1 {
    pub schema: String,
    pub version: u16,

    pub decisions: Vec<UserVerificationAccountingDecisionV1>,

    pub input_count: usize,
    pub proof_eligible_count: usize,
    pub challenge_evidence_count: usize,

    pub deterministic_order: bool,

    pub accounting_snapshot_created: bool,
    pub reward_plan_created: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Classify one strict User Node verification input.
///
/// # Errors
///
/// Rejects malformed, unattested, or authority-bearing evidence.
pub fn classify_user_verification(
    input: &UserVerificationAccountingInputV1,
) -> Result<UserVerificationAccountingDecisionV1> {
    input.validate().map_err(|error| {
        Error::schema(format!(
            "invalid user verification accounting input: \
             {error}"
        ))
    })?;

    let (class, requires_challenge_acceptance, reward_planning_candidate) = match input.result {
        UserVerificationResultV1::VerifiedValid => (
            UserVerificationAccountingClassV1::ProofEligibleVerification,
            false,
            true,
        ),

        UserVerificationResultV1::VerifiedInvalid | UserVerificationResultV1::ChallengeRaised => (
            UserVerificationAccountingClassV1::ChallengeEvidenceOnly,
            true,
            false,
        ),
    };

    Ok(UserVerificationAccountingDecisionV1 {
        sequence: input.sequence,
        evidence_id: input.evidence_id.clone(),
        user_node_id: input.user_node_id.clone(),
        subject_ref: input.subject_ref.clone(),
        verification_kind: input.verification_kind,
        result: input.result,
        input_digest: input.input_digest.clone(),
        observed_at_ms: input.observed_at_ms,
        class,
        verified_evidence: true,
        requires_policy_review: true,
        requires_economics_config: reward_planning_candidate,
        requires_cap: reward_planning_candidate,
        requires_challenge_acceptance,
        reward_planning_candidate,
        accounting_snapshot_member: false,
        reward_plan_created: false,
        direct_protocol_roc_allocation: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    })
}

/// Classify a bounded batch in deterministic sequence order.
///
/// Input ordering does not affect output ordering. Duplicate sequences or
/// duplicate `(user_node_id, evidence_id)` identities reject the batch.
///
/// # Errors
///
/// Returns a schema violation for invalid, empty, oversized, or duplicate
/// input.
pub fn classify_user_verification_batch(
    inputs: &[UserVerificationAccountingInputV1],
) -> Result<UserVerificationClassificationBatchV1> {
    if inputs.is_empty() {
        return Err(Error::schema(
            "user verification classification batch must not be empty",
        ));
    }

    if inputs.len() > MAX_USER_VERIFICATION_CLASSIFICATION_ITEMS {
        return Err(Error::schema(format!(
            "user verification classification batch exceeds \
             maximum: max={}, actual={}",
            MAX_USER_VERIFICATION_CLASSIFICATION_ITEMS,
            inputs.len()
        )));
    }

    let mut sequences = BTreeSet::new();
    let mut evidence_keys = BTreeSet::new();

    for input in inputs {
        input.validate().map_err(|error| {
            Error::schema(format!(
                "invalid user verification accounting input: \
                 {error}"
            ))
        })?;

        if !sequences.insert(input.sequence) {
            return Err(Error::schema(format!(
                "duplicate user verification sequence: {}",
                input.sequence
            )));
        }

        if !evidence_keys.insert((input.user_node_id.clone(), input.evidence_id.clone())) {
            return Err(Error::schema(format!(
                "duplicate user verification evidence identity: \
                 user_node_id={}, evidence_id={}",
                input.user_node_id, input.evidence_id
            )));
        }
    }

    let mut ordered = inputs.to_vec();

    ordered.sort_by(|left, right| {
        (
            left.sequence,
            left.verification_kind,
            left.evidence_id.as_str(),
        )
            .cmp(&(
                right.sequence,
                right.verification_kind,
                right.evidence_id.as_str(),
            ))
    });

    let decisions = ordered
        .iter()
        .map(classify_user_verification)
        .collect::<Result<Vec<_>>>()?;

    let proof_eligible_count = decisions
        .iter()
        .filter(|decision| {
            decision.class == UserVerificationAccountingClassV1::ProofEligibleVerification
        })
        .count();

    let challenge_evidence_count = decisions.len() - proof_eligible_count;

    Ok(UserVerificationClassificationBatchV1 {
        schema: USER_VERIFICATION_CLASSIFICATION_BATCH_SCHEMA.to_owned(),
        version: USER_VERIFICATION_CLASSIFICATION_BATCH_VERSION,
        input_count: decisions.len(),
        proof_eligible_count,
        challenge_evidence_count,
        decisions,
        deterministic_order: true,
        accounting_snapshot_created: false,
        reward_plan_created: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    })
}
