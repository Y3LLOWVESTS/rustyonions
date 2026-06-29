//! RO:WHAT — Strict QuickChain Phase 5 Round 3 chosen external posture DTOs.
//! RO:WHY — ECON/GOV: choose one external posture while keeping external evidence non-authoritative for internal ROC.
//! RO:INTERACTS — Phase 5 anchor commitments, DA fallback evidence, ron-ledger read-only posture checks.
//! RO:INVARIANTS — anchor-only selected now; evidence only; no balance, receipt, paid-unlock, bridge, or public runtime authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — external posture evidence cannot replace wallet/ledger truth, create receipts, unlock paid content, or settle ROC.
//! RO:TEST — tests/quickchain_phase5_external_posture.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    validate_chain_id, validate_epoch_id, validate_ref, validate_schema, validate_version,
    QuickChainCanonicalEncodingV1, QuickChainResult, QuickChainValidationError,
};

/// Schema tag for the Phase 5 Round 3 chosen external posture decision.
pub const QUICKCHAIN_EXTERNAL_POSTURE_DECISION_SCHEMA: &str =
    "quickchain.external-posture-decision.v1";

/// Schema tag for the Phase 5 Round 3 chosen external posture verification.
pub const QUICKCHAIN_EXTERNAL_POSTURE_VERIFICATION_SCHEMA: &str =
    "quickchain.external-posture-verification.v1";

/// Round 3 external integration posture vocabulary.
///
/// The current build chooses `AnchorOnly`. The other variants are explicit
/// deferred labels so a future governance decision cannot be smuggled in as a
/// missing/unknown field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainExternalIntegrationPostureV1 {
    /// Chosen now: external evidence may show commitment/timestamp visibility only.
    AnchorOnly,
    /// Deferred: external data availability provider posture.
    ExternalDataAvailabilityDeferred,
    /// Deferred: external layer-two or rollup posture.
    ExternalLayerTwoDeferred,
    /// Deferred: hybrid posture combining more than one external path.
    HybridDeferred,
}

/// Round 3 semantics for the chosen posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainExternalPostureSemanticsV1 {
    /// Evidence and anchoring only; internal wallet/ledger truth remains canonical.
    EvidenceAndAnchoringOnly,
}

/// Strict Phase 5 Round 3 chosen external posture decision.
///
/// This DTO deliberately chooses exactly one current posture: anchor-only
/// evidence/commitment hardening. It is not bridge configuration, not an external
/// settlement instruction, not paid-content authorization, and not receipt truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainExternalPostureDecisionV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id or planning epoch id.
    pub epoch_id: String,
    /// Durable posture decision id for audit/replay references.
    pub posture_id: String,
    /// Governance/reference label for the chosen posture.
    pub decision_ref: String,
    /// Anchor commitment or posture evidence hash being referenced.
    pub anchor_commitment_hash: ContentId,
    /// Human/audit-facing evidence reference; not authority.
    pub evidence_ref: String,
    /// Canonical encoding used by referenced internal evidence.
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    /// The single selected external posture.
    pub chosen_posture: QuickChainExternalIntegrationPostureV1,
    /// The semantics granted to this posture.
    pub posture_semantics: QuickChainExternalPostureSemanticsV1,
    /// Caller-supplied production timestamp; no wall-clock read happens here.
    pub produced_at_ms: u64,
    /// Must be true for this Round 3 sweep.
    pub anchor_only_selected: bool,
    /// Must be true: evidence/status/anchoring only.
    pub evidence_and_anchoring_only: bool,
    /// Must be true: wallet/ledger truth remains canonical for ROC.
    pub wallet_ledger_truth_canonical: bool,
    /// Must remain false unless a later governance phase explicitly chooses external DA.
    pub external_da_selected: bool,
    /// Must remain false unless a later governance phase explicitly chooses external L2/rollup.
    pub external_l2_selected: bool,
    /// Must remain false unless a later governance phase explicitly chooses hybrid.
    pub hybrid_selected: bool,
    /// Must remain false: external posture cannot mutate ROC balances.
    pub balance_mutation_authorized: bool,
    /// Must remain false: external posture cannot create wallet/ledger receipts.
    pub receipt_authority_authorized: bool,
    /// Must remain false: external posture cannot unlock paid content.
    pub paid_unlock_authorized: bool,
    /// Must remain false: external posture cannot settle internal ROC.
    pub external_settlement_authorized: bool,
    /// Must remain false: no bridge is authorized by this posture.
    pub bridge_authorized: bool,
    /// Must remain false: no ROX/Solana runtime is authorized here.
    pub rox_solana_runtime_authorized: bool,
    /// Must remain false: no exchange-facing logic is authorized here.
    pub exchange_facing_authorized: bool,
    /// Must remain false: no liquidity behavior is authorized here.
    pub liquidity_authorized: bool,
    /// Must remain false: no public staking behavior is authorized here.
    pub staking_authorized: bool,
}

impl QuickChainExternalPostureDecisionV1 {
    /// Validate chosen-posture shape and Round 3 authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainExternalPostureDecisionV1.schema",
            &self.schema,
            QUICKCHAIN_EXTERNAL_POSTURE_DECISION_SCHEMA,
        )?;
        validate_version("QuickChainExternalPostureDecisionV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("posture_id", &self.posture_id)?;
        validate_ref("decision_ref", &self.decision_ref)?;
        validate_ref("evidence_ref", &self.evidence_ref)?;

        if self.canonical_encoding != QuickChainCanonicalEncodingV1::JsonV1 {
            return Err(QuickChainValidationError::InvalidField {
                field: "canonical_encoding",
                reason: "Phase 5 Round 3 posture evidence currently allows only json-v1",
            });
        }

        if self.chosen_posture != QuickChainExternalIntegrationPostureV1::AnchorOnly {
            return Err(QuickChainValidationError::InvalidField {
                field: "chosen_posture",
                reason:
                    "Phase 5 Round 3 currently chooses anchor-only as the single external posture",
            });
        }

        if self.posture_semantics != QuickChainExternalPostureSemanticsV1::EvidenceAndAnchoringOnly
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "posture_semantics",
                reason: "external posture must remain evidence-and-anchoring-only",
            });
        }

        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "external posture timestamp must be caller supplied and non-zero",
            });
        }

        require_true(
            "anchor_only_selected",
            self.anchor_only_selected,
            "anchor-only must be the selected Round 3 posture",
        )?;
        require_true(
            "evidence_and_anchoring_only",
            self.evidence_and_anchoring_only,
            "external posture must be evidence and anchoring only",
        )?;
        require_true(
            "wallet_ledger_truth_canonical",
            self.wallet_ledger_truth_canonical,
            "internal wallet/ledger truth must remain canonical",
        )?;

        reject_true(
            "external_da_selected",
            self.external_da_selected,
            "external DA is not the selected Round 3 posture",
        )?;
        reject_true(
            "external_l2_selected",
            self.external_l2_selected,
            "external L2/rollup is not the selected Round 3 posture",
        )?;
        reject_true(
            "hybrid_selected",
            self.hybrid_selected,
            "hybrid is not the selected Round 3 posture",
        )?;
        reject_true(
            "balance_mutation_authorized",
            self.balance_mutation_authorized,
            "external posture must not authorize balance mutation",
        )?;
        reject_true(
            "receipt_authority_authorized",
            self.receipt_authority_authorized,
            "external posture must not authorize wallet/ledger receipt creation",
        )?;
        reject_true(
            "paid_unlock_authorized",
            self.paid_unlock_authorized,
            "external posture must not authorize paid unlock",
        )?;
        reject_true(
            "external_settlement_authorized",
            self.external_settlement_authorized,
            "external posture must not authorize external settlement",
        )?;
        reject_true(
            "bridge_authorized",
            self.bridge_authorized,
            "external posture must not authorize bridge behavior",
        )?;
        reject_true(
            "rox_solana_runtime_authorized",
            self.rox_solana_runtime_authorized,
            "external posture must not authorize ROX/Solana runtime",
        )?;
        reject_true(
            "exchange_facing_authorized",
            self.exchange_facing_authorized,
            "external posture must not authorize exchange-facing logic",
        )?;
        reject_true(
            "liquidity_authorized",
            self.liquidity_authorized,
            "external posture must not authorize liquidity behavior",
        )?;
        reject_true(
            "staking_authorized",
            self.staking_authorized,
            "external posture must not authorize public staking behavior",
        )
    }
}

/// Verification artifact for a chosen external posture decision.
///
/// A valid verification says only that a strict posture decision was read and
/// accepted as evidence/status metadata. It does not verify an external chain,
/// does not settle ROC, does not create receipts, and does not unlock content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainExternalPostureVerificationV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id or planning epoch id.
    pub epoch_id: String,
    /// Verified posture decision id.
    pub posture_id: String,
    /// The chosen posture observed during verification.
    pub chosen_posture: QuickChainExternalIntegrationPostureV1,
    /// Anchor commitment or posture evidence hash that was observed.
    pub anchor_commitment_hash: ContentId,
    /// Evidence reference observed during verification.
    pub observed_evidence_ref: String,
    /// Must be true: exactly one path was selected.
    pub single_path_selected: bool,
    /// Must be true: verification is evidence-only.
    pub verified_evidence_only: bool,
    /// Must be true: internal wallet/ledger truth remains canonical.
    pub wallet_ledger_truth_canonical: bool,
    /// Must remain false: verification found/caused no balance mutation.
    pub balance_mutation_detected: bool,
    /// Must remain false: verification found/caused no receipt authority.
    pub receipt_authority_detected: bool,
    /// Must remain false: verification found/caused no paid unlock authority.
    pub paid_unlock_detected: bool,
    /// Must remain false: verification did not detect/claim external settlement.
    pub external_settlement_detected: bool,
    /// Must remain false: verification did not detect/claim bridge behavior.
    pub bridge_detected: bool,
    /// Must remain false: verification did not detect/claim ROX/Solana runtime.
    pub rox_solana_runtime_detected: bool,
    /// Must remain false: verification did not detect/claim exchange-facing logic.
    pub exchange_facing_detected: bool,
    /// Must remain false: verification did not detect/claim liquidity behavior.
    pub liquidity_detected: bool,
    /// Must remain false: verification did not detect/claim public staking behavior.
    pub staking_detected: bool,
}

impl QuickChainExternalPostureVerificationV1 {
    /// Validate chosen-posture verification shape and authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainExternalPostureVerificationV1.schema",
            &self.schema,
            QUICKCHAIN_EXTERNAL_POSTURE_VERIFICATION_SCHEMA,
        )?;
        validate_version(
            "QuickChainExternalPostureVerificationV1.version",
            self.version,
        )?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("posture_id", &self.posture_id)?;
        validate_ref("observed_evidence_ref", &self.observed_evidence_ref)?;

        if self.chosen_posture != QuickChainExternalIntegrationPostureV1::AnchorOnly {
            return Err(QuickChainValidationError::InvalidField {
                field: "chosen_posture",
                reason:
                    "external posture verification must observe anchor-only in this Round 3 sweep",
            });
        }

        require_true(
            "single_path_selected",
            self.single_path_selected,
            "external posture verification must observe exactly one selected path",
        )?;
        require_true(
            "verified_evidence_only",
            self.verified_evidence_only,
            "external posture verification must remain evidence-only",
        )?;
        require_true(
            "wallet_ledger_truth_canonical",
            self.wallet_ledger_truth_canonical,
            "internal wallet/ledger truth must remain canonical",
        )?;

        reject_true(
            "balance_mutation_detected",
            self.balance_mutation_detected,
            "external posture verification must not detect or cause balance mutation",
        )?;
        reject_true(
            "receipt_authority_detected",
            self.receipt_authority_detected,
            "external posture verification must not detect or cause receipt authority",
        )?;
        reject_true(
            "paid_unlock_detected",
            self.paid_unlock_detected,
            "external posture verification must not detect or cause paid unlock authority",
        )?;
        reject_true(
            "external_settlement_detected",
            self.external_settlement_detected,
            "external posture verification must not claim external settlement",
        )?;
        reject_true(
            "bridge_detected",
            self.bridge_detected,
            "external posture verification must not claim bridge behavior",
        )?;
        reject_true(
            "rox_solana_runtime_detected",
            self.rox_solana_runtime_detected,
            "external posture verification must not claim ROX/Solana runtime",
        )?;
        reject_true(
            "exchange_facing_detected",
            self.exchange_facing_detected,
            "external posture verification must not claim exchange-facing logic",
        )?;
        reject_true(
            "liquidity_detected",
            self.liquidity_detected,
            "external posture verification must not claim liquidity behavior",
        )?;
        reject_true(
            "staking_detected",
            self.staking_detected,
            "external posture verification must not claim public staking behavior",
        )
    }
}

fn require_true(field: &'static str, actual: bool, reason: &'static str) -> QuickChainResult<()> {
    if actual {
        Ok(())
    } else {
        Err(QuickChainValidationError::InvalidField { field, reason })
    }
}

fn reject_true(field: &'static str, actual: bool, reason: &'static str) -> QuickChainResult<()> {
    if actual {
        Err(QuickChainValidationError::InvalidField { field, reason })
    } else {
        Ok(())
    }
}
