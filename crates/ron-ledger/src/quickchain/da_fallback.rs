//! RO:WHAT — Inert QuickChain Phase 5 Round 2 DA/archive/challenge fallback export and verification helpers.
//! RO:WHY — ECON/GOV: ledger can package checkpoint DA fallback plans while pruning remains blocked.
//! RO:INTERACTS — ron-proto DA fallback/checkpoint DTOs and prior reviewed checkpoint material.
//! RO:INVARIANTS — no IO, clocks, services, balance mutation, wallet mutation, bridge behavior, settlement, or external truth.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through quickchain-preflight.
//! RO:SECURITY — exports/verifications are evidence-only and grant no spend, unlock, finality, pruning, or settlement authority.
//! RO:TEST — tests/quickchain_phase5_da_fallback.rs.

use ron_proto::{
    quickchain::{
        QuickChainCheckpointHeaderV1, QuickChainDaChallengeReportV1, QuickChainDaChunkCommitmentV1,
        QuickChainDaFallbackModeV1, QuickChainDaFallbackPlanV1, QuickChainDaFallbackVerificationV1,
        QUICKCHAIN_DA_FALLBACK_PLAN_SCHEMA, QUICKCHAIN_DA_FALLBACK_VERIFICATION_SCHEMA,
        QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};
use thiserror::Error;

/// Explicit caller-supplied context for one DA/archive/challenge fallback plan.
///
/// This context contains no wall-clock reads, network calls, file writes, wallet
/// authority, pruning authority, or external truth. It only carries labels and
/// timestamps supplied by the caller/test harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainDaFallbackExportContext {
    fallback_plan_id: String,
    fallback_mode: QuickChainDaFallbackModeV1,
    challenge_window_start_ms: u64,
    challenge_window_end_ms: u64,
    produced_at_ms: u64,
}

impl QuickChainDaFallbackExportContext {
    /// Create caller-supplied fallback export context.
    #[must_use]
    pub fn new(
        fallback_plan_id: impl Into<String>,
        fallback_mode: QuickChainDaFallbackModeV1,
        challenge_window_start_ms: u64,
        challenge_window_end_ms: u64,
        produced_at_ms: u64,
    ) -> Self {
        Self {
            fallback_plan_id: fallback_plan_id.into(),
            fallback_mode,
            challenge_window_start_ms,
            challenge_window_end_ms,
            produced_at_ms,
        }
    }

    /// Fallback plan id.
    #[must_use]
    pub fn fallback_plan_id(&self) -> &str {
        &self.fallback_plan_id
    }

    /// Fallback mode.
    #[must_use]
    pub const fn fallback_mode(&self) -> QuickChainDaFallbackModeV1 {
        self.fallback_mode
    }

    /// Challenge window start.
    #[must_use]
    pub const fn challenge_window_start_ms(&self) -> u64 {
        self.challenge_window_start_ms
    }

    /// Challenge window end.
    #[must_use]
    pub const fn challenge_window_end_ms(&self) -> u64 {
        self.challenge_window_end_ms
    }

    /// Caller-supplied production timestamp.
    #[must_use]
    pub const fn produced_at_ms(&self) -> u64 {
        self.produced_at_ms
    }
}

/// Failure while exporting or verifying DA fallback evidence.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainDaFallbackError {
    /// Checkpoint DTO failed strict validation before export/verification.
    #[error("invalid QuickChain checkpoint header: {reason}")]
    InvalidCheckpoint {
        /// Bounded validation reason.
        reason: String,
    },

    /// Fallback plan DTO failed strict validation after assembly.
    #[error("invalid QuickChain DA fallback plan: {reason}")]
    InvalidPlan {
        /// Bounded validation reason.
        reason: String,
    },

    /// Fallback verification DTO failed strict validation after assembly.
    #[error("invalid QuickChain DA fallback verification: {reason}")]
    InvalidVerification {
        /// Bounded validation reason.
        reason: String,
    },

    /// Challenge report DTO failed strict validation before comparison.
    #[error("invalid QuickChain DA challenge report: {reason}")]
    InvalidChallengeReport {
        /// Bounded validation reason.
        reason: String,
    },

    /// Plan did not match reviewed checkpoint input.
    #[error("QuickChain DA fallback plan mismatch: {field}")]
    PlanMismatch {
        /// Field that failed deterministic comparison.
        field: &'static str,
    },

    /// Challenge report did not match reviewed plan input.
    #[error("QuickChain DA challenge report mismatch: {field}")]
    ChallengeReportMismatch {
        /// Field that failed deterministic comparison.
        field: &'static str,
    },
}

/// Export one strict DA/archive/challenge fallback plan from a validated checkpoint header.
///
/// This helper does not compute roots, write artifacts, contact a network,
/// mutate a wallet, mutate ledger state, grant pruning, or assign settlement
/// meaning.
pub fn export_da_fallback_plan(
    checkpoint: &QuickChainCheckpointHeaderV1,
    checkpoint_hash: ContentId,
    context: &QuickChainDaFallbackExportContext,
    chunks: Vec<QuickChainDaChunkCommitmentV1>,
) -> Result<QuickChainDaFallbackPlanV1, QuickChainDaFallbackError> {
    checkpoint
        .validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidCheckpoint {
            reason: error.to_string(),
        })?;

    let plan = QuickChainDaFallbackPlanV1 {
        schema: QUICKCHAIN_DA_FALLBACK_PLAN_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: checkpoint.chain_id.clone(),
        epoch_id: checkpoint.epoch_id.clone(),
        fallback_plan_id: context.fallback_plan_id().to_string(),
        checkpoint_height: checkpoint.height,
        checkpoint_hash,
        data_availability_root: checkpoint.data_availability_root.clone(),
        fallback_mode: context.fallback_mode(),
        challenge_window_start_ms: context.challenge_window_start_ms(),
        challenge_window_end_ms: context.challenge_window_end_ms(),
        produced_at_ms: context.produced_at_ms(),
        chunks,
        pruning_allowed: false,
        archive_fallback_required: true,
        missing_data_challenge_supported: true,
        restore_from_archive_tested: true,
        dry_run_only: true,
        normal_node_full_archive_required: false,
        external_da_truth: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        balance_mutation_authorized: false,
        wallet_ledger_truth_replaced: false,
        finality_claimed: false,
    };

    plan.validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidPlan {
            reason: error.to_string(),
        })?;

    Ok(plan)
}

/// Verify that a DA fallback plan matches reviewed checkpoint input.
///
/// Verification is field equality over inert DTO material only. It does not read
/// archive systems, restore chunks, mutate balances, unlock paid content, claim
/// external truth, grant pruning, or produce finality.
pub fn verify_da_fallback_plan(
    plan: &QuickChainDaFallbackPlanV1,
    checkpoint: &QuickChainCheckpointHeaderV1,
    checkpoint_hash: &ContentId,
) -> Result<QuickChainDaFallbackVerificationV1, QuickChainDaFallbackError> {
    checkpoint
        .validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidCheckpoint {
            reason: error.to_string(),
        })?;
    plan.validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidPlan {
            reason: error.to_string(),
        })?;

    ensure_equal_str("chain_id", &plan.chain_id, &checkpoint.chain_id)?;
    ensure_equal_str("epoch_id", &plan.epoch_id, &checkpoint.epoch_id)?;
    ensure_equal_u64(
        "checkpoint_height",
        plan.checkpoint_height,
        checkpoint.height,
    )?;
    ensure_equal_content_id("checkpoint_hash", &plan.checkpoint_hash, checkpoint_hash)?;
    ensure_equal_content_id(
        "data_availability_root",
        &plan.data_availability_root,
        &checkpoint.data_availability_root,
    )?;

    let checked_chunk_count =
        u32::try_from(plan.chunks.len()).map_err(|_| QuickChainDaFallbackError::InvalidPlan {
            reason: "chunk count does not fit u32".to_string(),
        })?;

    let verification = QuickChainDaFallbackVerificationV1 {
        schema: QUICKCHAIN_DA_FALLBACK_VERIFICATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: plan.chain_id.clone(),
        epoch_id: plan.epoch_id.clone(),
        fallback_plan_id: plan.fallback_plan_id.clone(),
        checkpoint_height: plan.checkpoint_height,
        checkpoint_hash: plan.checkpoint_hash.clone(),
        data_availability_root: plan.data_availability_root.clone(),
        checked_chunk_count,
        pruning_blocked: true,
        archive_fallback_checked: plan.archive_fallback_required,
        missing_data_challenge_checked: plan.missing_data_challenge_supported,
        restore_path_checked: plan.restore_from_archive_tested,
        dry_run_only: true,
        external_da_truth: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        balance_mutation_authorized: false,
        wallet_ledger_truth_replaced: false,
        finality_claimed: false,
    };

    verification
        .validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidVerification {
            reason: error.to_string(),
        })?;

    Ok(verification)
}

/// Verify that a missing-data challenge report targets a reviewed fallback plan.
///
/// This is report-only comparison. It does not punish, pay, prune, settle,
/// unlock, mutate, or finalize anything.
pub fn verify_da_challenge_report_against_plan(
    report: &QuickChainDaChallengeReportV1,
    plan: &QuickChainDaFallbackPlanV1,
) -> Result<(), QuickChainDaFallbackError> {
    report
        .validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidChallengeReport {
            reason: error.to_string(),
        })?;
    plan.validate()
        .map_err(|error| QuickChainDaFallbackError::InvalidPlan {
            reason: error.to_string(),
        })?;

    ensure_equal_str("chain_id", &report.chain_id, &plan.chain_id)?;
    ensure_equal_str("epoch_id", &report.epoch_id, &plan.epoch_id)?;
    ensure_equal_str(
        "fallback_plan_id",
        &report.fallback_plan_id,
        &plan.fallback_plan_id,
    )?;
    ensure_equal_content_id(
        "checkpoint_hash",
        &report.checkpoint_hash,
        &plan.checkpoint_hash,
    )?;

    if !plan
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_id == report.challenged_chunk_id)
    {
        return Err(QuickChainDaFallbackError::ChallengeReportMismatch {
            field: "challenged_chunk_id",
        });
    }

    if !plan.missing_data_challenge_supported {
        return Err(QuickChainDaFallbackError::ChallengeReportMismatch {
            field: "missing_data_challenge_supported",
        });
    }

    Ok(())
}

fn ensure_equal_str(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), QuickChainDaFallbackError> {
    if actual == expected {
        Ok(())
    } else {
        Err(QuickChainDaFallbackError::PlanMismatch { field })
    }
}

fn ensure_equal_u64(
    field: &'static str,
    actual: u64,
    expected: u64,
) -> Result<(), QuickChainDaFallbackError> {
    if actual == expected {
        Ok(())
    } else {
        Err(QuickChainDaFallbackError::PlanMismatch { field })
    }
}

fn ensure_equal_content_id(
    field: &'static str,
    actual: &ContentId,
    expected: &ContentId,
) -> Result<(), QuickChainDaFallbackError> {
    if actual == expected {
        Ok(())
    } else {
        Err(QuickChainDaFallbackError::PlanMismatch { field })
    }
}
