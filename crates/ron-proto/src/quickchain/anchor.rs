//! RO:WHAT — Strict QuickChain Phase 5 Round 1 anchor-only dry-run DTOs.
//! RO:WHY — ECON/GOV: model compact external timestamp/commitment artifacts without external settlement or balance authority.
//! RO:INTERACTS — checkpoint headers, tree roots, future archive/DA evidence, ron-ledger dry-run export checks.
//! RO:INVARIANTS — DTO/validation only; dry-run only; no bridge; no balance mutation; wallet/ledger truth remains canonical.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — anchor commitments prove timestamp/commitment only and grant no spend, finality, bridge, or settlement authority.
//! RO:TEST — tests/quickchain_phase5_anchor_dry_run.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    validate_chain_id, validate_epoch_id, validate_ref, validate_schema, validate_version,
    QuickChainCanonicalEncodingV1, QuickChainResult, QuickChainValidationError,
};

/// Schema tag for a Phase 5 Round 1 anchor-only commitment.
pub const QUICKCHAIN_ANCHOR_COMMITMENT_SCHEMA: &str = "quickchain.anchor-commitment.v1";

/// Schema tag for a Phase 5 Round 1 anchor-only verification artifact.
pub const QUICKCHAIN_ANCHOR_VERIFICATION_SCHEMA: &str = "quickchain.anchor-verification.v1";

/// Medium for an anchor-only commitment.
///
/// These variants describe where a compact commitment reference may be recorded.
/// They do not authorize settlement, balance mutation, public bridge behavior, or
/// external-chain truth for internal ROC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainAnchorMediumV1 {
    /// Local dry-run artifact/file used during Phase 5 Round 1 tests.
    DryRunFile,
    /// Generic external timestamp reference, still commitment-only.
    ExternalTimestampReference,
    /// Generic external commitment reference, still commitment-only.
    ExternalCommitmentReference,
}

/// Semantics claimed by an anchor-only commitment.
///
/// Phase 5 Round 1 permits only timestamp/commitment semantics. Any future
/// stronger semantics require later governance and a separate phase gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainAnchorSemanticsV1 {
    /// The anchor proves only that a compact commitment existed at/near a reference time.
    TimestampCommitmentOnly,
}

/// Compact Phase 5 Round 1 anchor commitment DTO.
///
/// A valid commitment can be copied to a dry-run artifact or external timestamp
/// system, but it cannot mutate ROC balances, replace wallet/ledger truth,
/// unlock paid content, create bridge finality, or claim external settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainAnchorCommitmentV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id.
    pub epoch_id: String,
    /// Backend/reviewer-assigned dry-run anchor id.
    pub anchor_id: String,
    /// Dry-run provider/reference such as a file/artifact/timestamp system.
    pub anchor_provider_ref: String,
    /// Opaque external reference. This is not ROC truth.
    pub external_reference: String,
    /// Local dry-run artifact reference for reproducibility.
    pub dry_run_artifact_ref: String,
    /// Checkpoint height being committed.
    pub checkpoint_height: u64,
    /// Reviewed checkpoint/header commitment hash supplied by the caller.
    pub checkpoint_hash: ContentId,
    /// New state root from the checkpoint header.
    pub new_state_root: ContentId,
    /// Receipt root from the checkpoint header.
    pub receipt_root: ContentId,
    /// Accounting snapshot root from the checkpoint header.
    pub accounting_snapshot_root: ContentId,
    /// Reward manifest root from the checkpoint header.
    pub reward_manifest_root: ContentId,
    /// Data-availability root from the checkpoint header.
    pub data_availability_root: ContentId,
    /// Policy hash from the checkpoint header.
    pub policy_hash: ContentId,
    /// Validator-set hash from the checkpoint header.
    pub validator_set_hash: ContentId,
    /// Chain-params hash from the checkpoint header.
    pub chain_params_hash: ContentId,
    /// Canonical encoding used by the committed checkpoint material.
    pub canonical_encoding: QuickChainCanonicalEncodingV1,
    /// Anchor medium; commitment-only.
    pub anchor_medium: QuickChainAnchorMediumV1,
    /// Semantics; Phase 5 Round 1 must remain timestamp/commitment only.
    pub anchor_semantics: QuickChainAnchorSemanticsV1,
    /// Caller-supplied production timestamp; no wall-clock read happens here.
    pub produced_at_ms: u64,
    /// Must be true in Phase 5 Round 1.
    pub dry_run_only: bool,
    /// Must be true in Phase 5 Round 1.
    pub timestamp_commitment_only: bool,
    /// Must remain false: anchors cannot mutate ROC balances.
    pub balance_mutation_authorized: bool,
    /// Must remain false: anchors cannot replace wallet/ledger truth.
    pub wallet_ledger_truth_replaced: bool,
    /// Must remain false: anchors cannot authorize external settlement.
    pub external_settlement_authorized: bool,
    /// Must remain false: no bridge is authorized by this DTO.
    pub bridge_authorized: bool,
    /// Must remain false: external chains/artifacts are not internal ROC truth.
    pub external_chain_truth: bool,
    /// Must remain false: this DTO does not claim committee/finality status.
    pub finality_claimed: bool,
}

impl QuickChainAnchorCommitmentV1 {
    /// Validate anchor-only commitment shape and authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainAnchorCommitmentV1.schema",
            &self.schema,
            QUICKCHAIN_ANCHOR_COMMITMENT_SCHEMA,
        )?;
        validate_version("QuickChainAnchorCommitmentV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("anchor_id", &self.anchor_id)?;
        validate_ref("anchor_provider_ref", &self.anchor_provider_ref)?;
        validate_ref("external_reference", &self.external_reference)?;
        validate_ref("dry_run_artifact_ref", &self.dry_run_artifact_ref)?;

        if self.canonical_encoding != QuickChainCanonicalEncodingV1::JsonV1 {
            return Err(QuickChainValidationError::InvalidField {
                field: "canonical_encoding",
                reason: "Phase 5 anchor commitments currently allow only json-v1",
            });
        }

        if self.anchor_semantics != QuickChainAnchorSemanticsV1::TimestampCommitmentOnly {
            return Err(QuickChainValidationError::InvalidField {
                field: "anchor_semantics",
                reason: "Phase 5 Round 1 permits timestamp/commitment semantics only",
            });
        }

        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "anchor commitment timestamp must be caller supplied and non-zero",
            });
        }

        if !self.dry_run_only {
            return Err(QuickChainValidationError::InvalidField {
                field: "dry_run_only",
                reason: "Phase 5 Round 1 anchor commitments must remain dry-run only",
            });
        }

        if !self.timestamp_commitment_only {
            return Err(QuickChainValidationError::InvalidField {
                field: "timestamp_commitment_only",
                reason: "anchor commitment must prove timestamp/commitment only",
            });
        }

        if self.balance_mutation_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "balance_mutation_authorized",
                reason: "anchor commitment must not authorize balance mutation",
            });
        }

        if self.wallet_ledger_truth_replaced {
            return Err(QuickChainValidationError::InvalidField {
                field: "wallet_ledger_truth_replaced",
                reason: "anchor commitment must not replace wallet/ledger truth",
            });
        }

        if self.external_settlement_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_settlement_authorized",
                reason: "anchor commitment must not authorize external settlement",
            });
        }

        if self.bridge_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "bridge_authorized",
                reason: "anchor commitment must not authorize bridge behavior",
            });
        }

        if self.external_chain_truth {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_chain_truth",
                reason: "external anchor artifacts must not become internal ROC truth",
            });
        }

        if self.finality_claimed {
            return Err(QuickChainValidationError::InvalidField {
                field: "finality_claimed",
                reason: "anchor commitment must not claim finality",
            });
        }

        Ok(())
    }
}

/// Verification artifact for a Phase 5 Round 1 anchor-only commitment.
///
/// A valid verification says only that the commitment fields matched the reviewed
/// checkpoint/export inputs. It does not verify or claim an external chain state,
/// and it does not mutate ROC balances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainAnchorVerificationV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id.
    pub epoch_id: String,
    /// Anchor commitment id being checked.
    pub anchor_id: String,
    /// Checkpoint height that was checked.
    pub checkpoint_height: u64,
    /// Checkpoint commitment hash that was checked.
    pub checkpoint_hash: ContentId,
    /// External reference that was checked as an opaque commitment reference.
    pub observed_external_reference: String,
    /// Must be true: verification is commitment-only.
    pub verified_commitment_only: bool,
    /// Must remain false: verification found/caused no balance mutation.
    pub balance_mutation_detected: bool,
    /// Must remain false: verification did not replace wallet/ledger truth.
    pub wallet_ledger_truth_replaced: bool,
    /// Must remain false: verification did not detect/claim external settlement.
    pub external_settlement_detected: bool,
    /// Must remain false: verification did not detect/claim bridge behavior.
    pub bridge_detected: bool,
    /// Must remain false: verification did not claim finality.
    pub finality_detected: bool,
}

impl QuickChainAnchorVerificationV1 {
    /// Validate anchor-only verification shape and authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainAnchorVerificationV1.schema",
            &self.schema,
            QUICKCHAIN_ANCHOR_VERIFICATION_SCHEMA,
        )?;
        validate_version("QuickChainAnchorVerificationV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("anchor_id", &self.anchor_id)?;
        validate_ref(
            "observed_external_reference",
            &self.observed_external_reference,
        )?;

        if !self.verified_commitment_only {
            return Err(QuickChainValidationError::InvalidField {
                field: "verified_commitment_only",
                reason: "anchor verification must remain commitment-only",
            });
        }

        if self.balance_mutation_detected {
            return Err(QuickChainValidationError::InvalidField {
                field: "balance_mutation_detected",
                reason: "anchor verification must not detect or cause balance mutation",
            });
        }

        if self.wallet_ledger_truth_replaced {
            return Err(QuickChainValidationError::InvalidField {
                field: "wallet_ledger_truth_replaced",
                reason: "anchor verification must not replace wallet/ledger truth",
            });
        }

        if self.external_settlement_detected {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_settlement_detected",
                reason: "anchor verification must not claim external settlement",
            });
        }

        if self.bridge_detected {
            return Err(QuickChainValidationError::InvalidField {
                field: "bridge_detected",
                reason: "anchor verification must not claim bridge behavior",
            });
        }

        if self.finality_detected {
            return Err(QuickChainValidationError::InvalidField {
                field: "finality_detected",
                reason: "anchor verification must not claim finality",
            });
        }

        Ok(())
    }
}
