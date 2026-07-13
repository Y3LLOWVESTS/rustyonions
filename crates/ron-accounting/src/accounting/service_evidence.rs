//! RO:WHAT — Deterministic Phase 14 classification of service evidence.
//! RO:WHY — Verified evidence must be classified before snapshots or reward plans.
//! RO:INTERACTS — strict accounting-ingress DTO and later reward projection.
//! RO:INVARIANTS — deterministic ordering; policy evidence is not reward material.
//! RO:SECURITY — no reward amount, payout, receipt, balance, wallet, or ledger authority.
//! RO:TEST — tests/service_evidence_classification.rs.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use super::node_evidence_wire::{
    EvidenceContentId, ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingKindV1,
};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const SERVICE_EVIDENCE_CLASSIFICATION_BATCH_SCHEMA: &str =
    "ron.accounting.service-evidence-classification-batch.v1";

pub const SERVICE_EVIDENCE_CLASSIFICATION_BATCH_VERSION: u16 = 1;

pub const MAX_SERVICE_EVIDENCE_CLASSIFICATION_ITEMS: usize = 4_096;

/// Accounting classification assigned before reward planning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceEvidenceAccountingClassV1 {
    /// Verified service contribution that may enter later capped planning.
    ProofEligibleService,

    /// Refusal/moderation evidence retained for policy and audit review.
    PolicyEvidenceOnly,
}

/// Deterministic accounting decision for one reviewed evidence item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceEvidenceAccountingDecisionV1 {
    pub sequence: u64,
    pub kind: ServiceEvidenceAccountingKindV1,

    pub proof_id: String,
    pub service_node_id: String,
    pub content_id: EvidenceContentId,
    pub observed_at_ms: u64,

    pub class: ServiceEvidenceAccountingClassV1,

    pub verified_evidence: bool,
    pub requires_policy_review: bool,
    pub requires_economics_config: bool,
    pub requires_cap: bool,

    /// Candidate status only. No amount or payout is created here.
    pub reward_planning_candidate: bool,

    pub accounting_snapshot_member: bool,
    pub reward_plan_created: bool,
    pub direct_protocol_roc_allocation: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Deterministically ordered batch of classification decisions.
///
/// This is not an accounting snapshot, reward plan, payout, or receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceEvidenceClassificationBatchV1 {
    pub schema: String,
    pub version: u16,

    pub decisions: Vec<ServiceEvidenceAccountingDecisionV1>,

    pub input_count: usize,
    pub proof_eligible_count: usize,
    pub policy_evidence_count: usize,

    pub deterministic_order: bool,

    pub accounting_snapshot_created: bool,
    pub reward_plan_created: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Classify one strict service-evidence accounting input.
///
/// # Errors
///
/// Rejects malformed, unsigned, self-traffic, or authority-bearing input.
pub fn classify_service_evidence(
    input: &ServiceEvidenceAccountingInputV1,
) -> Result<ServiceEvidenceAccountingDecisionV1> {
    input.validate().map_err(|error| {
        Error::schema(format!(
            "invalid service evidence accounting input: \
                 {error}"
        ))
    })?;

    let (class, reward_planning_candidate, requires_economics_config, requires_cap) =
        match input.kind {
            ServiceEvidenceAccountingKindV1::Delivery
            | ServiceEvidenceAccountingKindV1::Availability
            | ServiceEvidenceAccountingKindV1::RangeRequest
            | ServiceEvidenceAccountingKindV1::Repair
            | ServiceEvidenceAccountingKindV1::HotCache => (
                ServiceEvidenceAccountingClassV1::ProofEligibleService,
                true,
                true,
                true,
            ),

            ServiceEvidenceAccountingKindV1::PolicyRefusal
            | ServiceEvidenceAccountingKindV1::ModerationAction => (
                ServiceEvidenceAccountingClassV1::PolicyEvidenceOnly,
                false,
                false,
                false,
            ),
        };

    Ok(ServiceEvidenceAccountingDecisionV1 {
        sequence: input.sequence,
        kind: input.kind,
        proof_id: input.proof_id.clone(),
        service_node_id: input.service_node_id.clone(),
        content_id: input.content_id.clone(),
        observed_at_ms: input.observed_at_ms,
        class,
        verified_evidence: true,
        requires_policy_review: true,
        requires_economics_config,
        requires_cap,
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

/// Classify a bounded batch in deterministic order.
///
/// Input order does not affect output order. Duplicate outbox sequences or
/// duplicate `(kind, proof_id)` identities reject the whole batch.
///
/// # Errors
///
/// Returns a schema violation for empty/oversized batches, duplicates, or
/// invalid evidence input.
pub fn classify_service_evidence_batch(
    inputs: &[ServiceEvidenceAccountingInputV1],
) -> Result<ServiceEvidenceClassificationBatchV1> {
    if inputs.is_empty() {
        return Err(Error::schema(
            "service evidence classification batch must not be empty",
        ));
    }

    if inputs.len() > MAX_SERVICE_EVIDENCE_CLASSIFICATION_ITEMS {
        return Err(Error::schema(format!(
            "service evidence classification batch exceeds \
             maximum: max={}, actual={}",
            MAX_SERVICE_EVIDENCE_CLASSIFICATION_ITEMS,
            inputs.len()
        )));
    }

    let mut sequences = BTreeSet::new();
    let mut proof_keys = BTreeSet::new();

    for input in inputs {
        input.validate().map_err(|error| {
            Error::schema(format!(
                "invalid service evidence accounting input: \
                     {error}"
            ))
        })?;

        if !sequences.insert(input.sequence) {
            return Err(Error::schema(format!(
                "duplicate service evidence sequence: {}",
                input.sequence
            )));
        }

        if !proof_keys.insert((input.kind, input.proof_id.clone())) {
            return Err(Error::schema(format!(
                "duplicate service evidence proof identity: \
                 kind={:?}, proof_id={}",
                input.kind, input.proof_id
            )));
        }
    }

    let mut ordered = inputs.to_vec();

    ordered.sort_by(|left, right| {
        (left.sequence, left.kind, left.proof_id.as_str()).cmp(&(
            right.sequence,
            right.kind,
            right.proof_id.as_str(),
        ))
    });

    let decisions = ordered
        .iter()
        .map(classify_service_evidence)
        .collect::<Result<Vec<_>>>()?;

    let proof_eligible_count = decisions
        .iter()
        .filter(|decision| decision.class == ServiceEvidenceAccountingClassV1::ProofEligibleService)
        .count();

    let policy_evidence_count = decisions.len() - proof_eligible_count;

    Ok(ServiceEvidenceClassificationBatchV1 {
        schema: SERVICE_EVIDENCE_CLASSIFICATION_BATCH_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_CLASSIFICATION_BATCH_VERSION,
        input_count: decisions.len(),
        proof_eligible_count,
        policy_evidence_count,
        decisions,
        deterministic_order: true,
        accounting_snapshot_created: false,
        reward_plan_created: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    })
}
