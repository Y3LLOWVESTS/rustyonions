//! RO:WHAT — Inert QuickChain Phase 5 Round 1 anchor-only dry-run export and verification helpers.
//! RO:WHY — ECON/GOV: ledger can package reviewed checkpoint commitments for timestamp-only dry-runs without external settlement.
//! RO:INTERACTS — ron-proto anchor/checkpoint DTOs and prior deterministic root/checkpoint material.
//! RO:INVARIANTS — no IO, clocks, services, balance mutation, wallet mutation, bridge behavior, or external-chain truth.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through quickchain-preflight.
//! RO:SECURITY — exports/verifications are evidence-only and grant no spend, unlock, finality, validator, or settlement authority.
//! RO:TEST — tests/quickchain_phase5_anchor_dry_run.rs.

use ron_proto::{
    quickchain::{
        QuickChainAnchorCommitmentV1, QuickChainAnchorMediumV1, QuickChainAnchorSemanticsV1,
        QuickChainAnchorVerificationV1, QuickChainCheckpointHeaderV1,
        QUICKCHAIN_ANCHOR_COMMITMENT_SCHEMA, QUICKCHAIN_ANCHOR_VERIFICATION_SCHEMA,
        QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};
use thiserror::Error;

/// Explicit caller-supplied context for one anchor-only dry-run export.
///
/// This context contains no wall-clock reads, network calls, file writes, wallet
/// authority, or external-chain authority. It only carries already-reviewed
/// labels and an explicit timestamp supplied by the caller/test harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainAnchorDryRunExportContext {
    anchor_id: String,
    anchor_provider_ref: String,
    external_reference: String,
    dry_run_artifact_ref: String,
    produced_at_ms: u64,
}

impl QuickChainAnchorDryRunExportContext {
    /// Create caller-supplied dry-run export context.
    #[must_use]
    pub fn new(
        anchor_id: impl Into<String>,
        anchor_provider_ref: impl Into<String>,
        external_reference: impl Into<String>,
        dry_run_artifact_ref: impl Into<String>,
        produced_at_ms: u64,
    ) -> Self {
        Self {
            anchor_id: anchor_id.into(),
            anchor_provider_ref: anchor_provider_ref.into(),
            external_reference: external_reference.into(),
            dry_run_artifact_ref: dry_run_artifact_ref.into(),
            produced_at_ms,
        }
    }

    /// Dry-run anchor id.
    #[must_use]
    pub fn anchor_id(&self) -> &str {
        &self.anchor_id
    }

    /// Opaque provider/reference id.
    #[must_use]
    pub fn anchor_provider_ref(&self) -> &str {
        &self.anchor_provider_ref
    }

    /// Opaque external reference.
    #[must_use]
    pub fn external_reference(&self) -> &str {
        &self.external_reference
    }

    /// Local dry-run artifact reference.
    #[must_use]
    pub fn dry_run_artifact_ref(&self) -> &str {
        &self.dry_run_artifact_ref
    }

    /// Caller-supplied production timestamp.
    #[must_use]
    pub const fn produced_at_ms(&self) -> u64 {
        self.produced_at_ms
    }
}

/// Failure while exporting or verifying anchor-only dry-run evidence.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainAnchorDryRunError {
    /// Checkpoint DTO failed strict validation before export/verification.
    #[error("invalid QuickChain checkpoint header: {reason}")]
    InvalidCheckpoint {
        /// Bounded validation reason.
        reason: String,
    },

    /// Anchor DTO failed strict validation after assembly.
    #[error("invalid QuickChain anchor commitment: {reason}")]
    InvalidCommitment {
        /// Bounded validation reason.
        reason: String,
    },

    /// Anchor verification DTO failed strict validation after assembly.
    #[error("invalid QuickChain anchor verification: {reason}")]
    InvalidVerification {
        /// Bounded validation reason.
        reason: String,
    },

    /// Commitment did not match reviewed checkpoint input.
    #[error("QuickChain anchor commitment mismatch: {field}")]
    CommitmentMismatch {
        /// Field that failed deterministic comparison.
        field: &'static str,
    },
}

/// Export one strict anchor-only dry-run commitment from a validated checkpoint header.
///
/// The supplied `checkpoint_hash` is treated as reviewed input. This helper does
/// not compute a checkpoint hash, write an artifact, contact a network, mutate a
/// wallet, mutate ledger state, or assign settlement/finality meaning.
pub fn export_anchor_dry_run_commitment(
    checkpoint: &QuickChainCheckpointHeaderV1,
    checkpoint_hash: ContentId,
    context: &QuickChainAnchorDryRunExportContext,
) -> Result<QuickChainAnchorCommitmentV1, QuickChainAnchorDryRunError> {
    checkpoint
        .validate()
        .map_err(|error| QuickChainAnchorDryRunError::InvalidCheckpoint {
            reason: error.to_string(),
        })?;

    let commitment = QuickChainAnchorCommitmentV1 {
        schema: QUICKCHAIN_ANCHOR_COMMITMENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: checkpoint.chain_id.clone(),
        epoch_id: checkpoint.epoch_id.clone(),
        anchor_id: context.anchor_id().to_string(),
        anchor_provider_ref: context.anchor_provider_ref().to_string(),
        external_reference: context.external_reference().to_string(),
        dry_run_artifact_ref: context.dry_run_artifact_ref().to_string(),
        checkpoint_height: checkpoint.height,
        checkpoint_hash,
        new_state_root: checkpoint.new_state_root.clone(),
        receipt_root: checkpoint.receipt_root.clone(),
        accounting_snapshot_root: checkpoint.accounting_snapshot_root.clone(),
        reward_manifest_root: checkpoint.reward_manifest_root.clone(),
        data_availability_root: checkpoint.data_availability_root.clone(),
        policy_hash: checkpoint.policy_hash.clone(),
        validator_set_hash: checkpoint.validator_set_hash.clone(),
        chain_params_hash: checkpoint.chain_params_hash.clone(),
        canonical_encoding: checkpoint.canonical_encoding,
        anchor_medium: QuickChainAnchorMediumV1::DryRunFile,
        anchor_semantics: QuickChainAnchorSemanticsV1::TimestampCommitmentOnly,
        produced_at_ms: context.produced_at_ms(),
        dry_run_only: true,
        timestamp_commitment_only: true,
        balance_mutation_authorized: false,
        wallet_ledger_truth_replaced: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        external_chain_truth: false,
        finality_claimed: false,
    };

    commitment
        .validate()
        .map_err(|error| QuickChainAnchorDryRunError::InvalidCommitment {
            reason: error.to_string(),
        })?;

    Ok(commitment)
}

/// Verify that an anchor-only dry-run commitment matches reviewed checkpoint input.
///
/// Verification is field equality over inert DTO material only. It does not read
/// external systems, claim external-chain truth, mutate balances, unlock paid
/// content, or produce finality.
pub fn verify_anchor_dry_run_commitment(
    commitment: &QuickChainAnchorCommitmentV1,
    checkpoint: &QuickChainCheckpointHeaderV1,
    checkpoint_hash: &ContentId,
) -> Result<QuickChainAnchorVerificationV1, QuickChainAnchorDryRunError> {
    checkpoint
        .validate()
        .map_err(|error| QuickChainAnchorDryRunError::InvalidCheckpoint {
            reason: error.to_string(),
        })?;
    commitment
        .validate()
        .map_err(|error| QuickChainAnchorDryRunError::InvalidCommitment {
            reason: error.to_string(),
        })?;

    ensure_equal_str("chain_id", &commitment.chain_id, &checkpoint.chain_id)?;
    ensure_equal_str("epoch_id", &commitment.epoch_id, &checkpoint.epoch_id)?;
    ensure_equal_u64(
        "checkpoint_height",
        commitment.checkpoint_height,
        checkpoint.height,
    )?;
    ensure_equal_content_id(
        "checkpoint_hash",
        &commitment.checkpoint_hash,
        checkpoint_hash,
    )?;
    ensure_equal_content_id(
        "new_state_root",
        &commitment.new_state_root,
        &checkpoint.new_state_root,
    )?;
    ensure_equal_content_id(
        "receipt_root",
        &commitment.receipt_root,
        &checkpoint.receipt_root,
    )?;
    ensure_equal_content_id(
        "accounting_snapshot_root",
        &commitment.accounting_snapshot_root,
        &checkpoint.accounting_snapshot_root,
    )?;
    ensure_equal_content_id(
        "reward_manifest_root",
        &commitment.reward_manifest_root,
        &checkpoint.reward_manifest_root,
    )?;
    ensure_equal_content_id(
        "data_availability_root",
        &commitment.data_availability_root,
        &checkpoint.data_availability_root,
    )?;
    ensure_equal_content_id(
        "policy_hash",
        &commitment.policy_hash,
        &checkpoint.policy_hash,
    )?;
    ensure_equal_content_id(
        "validator_set_hash",
        &commitment.validator_set_hash,
        &checkpoint.validator_set_hash,
    )?;
    ensure_equal_content_id(
        "chain_params_hash",
        &commitment.chain_params_hash,
        &checkpoint.chain_params_hash,
    )?;

    if commitment.canonical_encoding != checkpoint.canonical_encoding {
        return Err(QuickChainAnchorDryRunError::CommitmentMismatch {
            field: "canonical_encoding",
        });
    }

    let verification = QuickChainAnchorVerificationV1 {
        schema: QUICKCHAIN_ANCHOR_VERIFICATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: commitment.chain_id.clone(),
        epoch_id: commitment.epoch_id.clone(),
        anchor_id: commitment.anchor_id.clone(),
        checkpoint_height: commitment.checkpoint_height,
        checkpoint_hash: commitment.checkpoint_hash.clone(),
        observed_external_reference: commitment.external_reference.clone(),
        verified_commitment_only: true,
        balance_mutation_detected: false,
        wallet_ledger_truth_replaced: false,
        external_settlement_detected: false,
        bridge_detected: false,
        finality_detected: false,
    };

    verification
        .validate()
        .map_err(|error| QuickChainAnchorDryRunError::InvalidVerification {
            reason: error.to_string(),
        })?;

    Ok(verification)
}

fn ensure_equal_str(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), QuickChainAnchorDryRunError> {
    if actual == expected {
        Ok(())
    } else {
        Err(QuickChainAnchorDryRunError::CommitmentMismatch { field })
    }
}

fn ensure_equal_u64(
    field: &'static str,
    actual: u64,
    expected: u64,
) -> Result<(), QuickChainAnchorDryRunError> {
    if actual == expected {
        Ok(())
    } else {
        Err(QuickChainAnchorDryRunError::CommitmentMismatch { field })
    }
}

fn ensure_equal_content_id(
    field: &'static str,
    actual: &ContentId,
    expected: &ContentId,
) -> Result<(), QuickChainAnchorDryRunError> {
    if actual == expected {
        Ok(())
    } else {
        Err(QuickChainAnchorDryRunError::CommitmentMismatch { field })
    }
}
