//! RO:WHAT — Strict QuickChain Phase 5 Round 2 DA/archive/challenge fallback DTOs.
//! RO:WHY — ECON/GOV: model data-retention and missing-data challenge fallback before any pruning is allowed.
//! RO:INTERACTS — checkpoint headers, anchor commitments, future archive artifacts, ron-ledger read-only fallback checks.
//! RO:INVARIANTS — DTO/validation only; dry-run only; no pruning grant; no bridge; no settlement; wallet/ledger truth remains canonical.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fallback evidence grants no spend, balance, unlock, finality, settlement, or external truth authority.
//! RO:TEST — tests/quickchain_phase5_da_fallback.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    validate_chain_id, validate_epoch_id, validate_ref, validate_schema, validate_version,
    QuickChainResult, QuickChainValidationError,
};

/// Schema tag for one DA/archive chunk commitment.
pub const QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA: &str = "quickchain.da-chunk-commitment.v1";

/// Schema tag for a Phase 5 Round 2 DA/archive/challenge fallback plan.
pub const QUICKCHAIN_DA_FALLBACK_PLAN_SCHEMA: &str = "quickchain.da-fallback-plan.v1";

/// Schema tag for a Phase 5 Round 2 missing-data challenge report.
pub const QUICKCHAIN_DA_CHALLENGE_REPORT_SCHEMA: &str = "quickchain.da-challenge-report.v1";

/// Schema tag for a Phase 5 Round 2 DA fallback verification artifact.
pub const QUICKCHAIN_DA_FALLBACK_VERIFICATION_SCHEMA: &str =
    "quickchain.da-fallback-verification.v1";

/// Maximum chunk commitments carried by one fallback plan DTO.
pub const MAX_QUICKCHAIN_DA_CHUNKS: usize = 128;

/// Maximum byte length for one DA/archive chunk in this DTO model.
///
/// This is not a transport limit. It only prevents accidental unbounded
/// commitment manifests during the pre-live fallback design round.
pub const MAX_QUICKCHAIN_DA_CHUNK_BYTES: u64 = 256 * 1024 * 1024;

/// Phase 5 Round 2 fallback mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainDaFallbackModeV1 {
    /// Archive-first dry run; pruning remains disabled.
    ArchiveFirstNoPruning,
    /// Missing-data challenge dry run; pruning remains disabled.
    MissingDataChallengeDryRun,
    /// Restore-from-archive dry run; pruning remains disabled.
    RestoreFromArchiveDryRun,
}

/// Kind of data committed into DA/archive fallback material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainDaChunkKindV1 {
    /// Wallet/ledger accepted economic receipt batch.
    EconomicReceiptBatch,
    /// Ledger transition evidence batch.
    LedgerTransitionBatch,
    /// Account/state proof material batch.
    AccountStateProofBatch,
    /// Accounting snapshot artifact.
    AccountingSnapshot,
    /// Reward planning manifest artifact.
    RewardManifest,
    /// Policy/config snapshot artifact.
    PolicySnapshot,
    /// Chain parameter snapshot artifact.
    ChainParamsSnapshot,
    /// Analytics-only summary; never balance or payout truth.
    AnalyticsSummary,
}

/// Retention class for one committed chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainDaRetentionClassV1 {
    /// Recent material all full nodes should retain in early phases.
    Hot,
    /// Recent epoch material pruned nodes may retain later, after proof gates.
    Warm,
    /// Older material assigned to archive/storage roles later.
    Cold,
    /// Full archive scope.
    Permanent,
    /// Header/proof-only light material.
    Light,
}

/// Missing-data challenge report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainDaChallengeStatusV1 {
    /// No missing data was found for the checked chunk.
    NoMissingData,
    /// Missing data challenge window is open.
    MissingDataChallengeOpen,
    /// Missing data was restored from archive in the dry run.
    MissingDataRestored,
    /// Missing data was not restored in the dry run.
    MissingDataFailedDryRun,
}

/// One DA/archive chunk commitment.
///
/// This is an inert DTO. It does not write, fetch, pin, restore, or prove bytes.
/// It only names reviewed commitment material that a later DA/challenge system
/// may require.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainDaChunkCommitmentV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Stable caller-assigned chunk id.
    pub chunk_id: String,
    /// Data class carried by this chunk.
    pub chunk_kind: QuickChainDaChunkKindV1,
    /// Content-addressed chunk commitment.
    pub chunk_cid: ContentId,
    /// Declared chunk length in bytes.
    pub byte_len: u64,
    /// Retention class.
    pub retention_class: QuickChainDaRetentionClassV1,
    /// Archive reference label; not a network location authority.
    pub archive_ref: String,
    /// Restore proof/reference label; not proof by itself.
    pub restore_ref: String,
    /// Whether this chunk must be available for missing-data challenges.
    pub required_for_challenge: bool,
    /// Whether the caller's reviewed material says the chunk is available.
    pub available: bool,
    /// Whether a restore path was dry-run tested by the caller.
    pub restore_tested: bool,
    /// Analytics-only marker. Analytics-only chunks must not be economic truth.
    pub analytics_only: bool,
}

impl QuickChainDaChunkCommitmentV1 {
    /// Validate one chunk commitment.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainDaChunkCommitmentV1.schema",
            &self.schema,
            QUICKCHAIN_DA_CHUNK_COMMITMENT_SCHEMA,
        )?;
        validate_version("QuickChainDaChunkCommitmentV1.version", self.version)?;
        validate_ref("chunk_id", &self.chunk_id)?;
        validate_ref("archive_ref", &self.archive_ref)?;
        validate_ref("restore_ref", &self.restore_ref)?;

        if self.byte_len == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "byte_len",
                reason: "DA/archive chunk byte length must be greater than zero",
            });
        }

        if self.byte_len > MAX_QUICKCHAIN_DA_CHUNK_BYTES {
            return Err(QuickChainValidationError::InvalidField {
                field: "byte_len",
                reason: "DA/archive chunk byte length exceeds Phase 5 Round 2 DTO cap",
            });
        }

        if self.restore_tested && !self.available {
            return Err(QuickChainValidationError::InvalidField {
                field: "restore_tested",
                reason: "restore_tested requires the reviewed chunk to be available",
            });
        }

        if self.analytics_only && self.required_for_challenge {
            return Err(QuickChainValidationError::InvalidField {
                field: "analytics_only",
                reason:
                    "analytics-only material must not be required settlement challenge material",
            });
        }

        if self.analytics_only
            && matches!(
                self.chunk_kind,
                QuickChainDaChunkKindV1::EconomicReceiptBatch
                    | QuickChainDaChunkKindV1::LedgerTransitionBatch
                    | QuickChainDaChunkKindV1::AccountStateProofBatch
            )
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "analytics_only",
                reason: "economic/proof chunks must not be marked analytics-only",
            });
        }

        if !self.analytics_only
            && matches!(self.chunk_kind, QuickChainDaChunkKindV1::AnalyticsSummary)
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "chunk_kind",
                reason: "analytics summary chunks must be marked analytics_only",
            });
        }

        Ok(())
    }
}

/// Phase 5 Round 2 DA/archive/challenge fallback plan.
///
/// This DTO intentionally blocks pruning. It models the evidence that must exist
/// before pruning can ever be considered in a later phase; it does not grant that
/// later permission itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainDaFallbackPlanV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id.
    pub epoch_id: String,
    /// Caller-assigned fallback plan id.
    pub fallback_plan_id: String,
    /// Checkpoint height being covered.
    pub checkpoint_height: u64,
    /// Reviewed checkpoint hash supplied by caller.
    pub checkpoint_hash: ContentId,
    /// Data-availability root from checkpoint header.
    pub data_availability_root: ContentId,
    /// Round 2 fallback mode.
    pub fallback_mode: QuickChainDaFallbackModeV1,
    /// Challenge window start, supplied by caller.
    pub challenge_window_start_ms: u64,
    /// Challenge window end, supplied by caller.
    pub challenge_window_end_ms: u64,
    /// Plan production timestamp, supplied by caller.
    pub produced_at_ms: u64,
    /// Reviewed chunks covered by this fallback plan.
    pub chunks: Vec<QuickChainDaChunkCommitmentV1>,
    /// Must remain false in this round.
    pub pruning_allowed: bool,
    /// Must remain true in this round.
    pub archive_fallback_required: bool,
    /// Must remain true in this round.
    pub missing_data_challenge_supported: bool,
    /// Must remain true in this round.
    pub restore_from_archive_tested: bool,
    /// Must remain true in this round.
    pub dry_run_only: bool,
    /// Must remain false: normal nodes are not forced to become full archives.
    pub normal_node_full_archive_required: bool,
    /// Must remain false: external DA artifacts are not internal ROC truth.
    pub external_da_truth: bool,
    /// Must remain false: this DTO cannot authorize external settlement.
    pub external_settlement_authorized: bool,
    /// Must remain false: this DTO cannot authorize bridge behavior.
    pub bridge_authorized: bool,
    /// Must remain false: this DTO cannot authorize balance mutation.
    pub balance_mutation_authorized: bool,
    /// Must remain false: this DTO cannot replace wallet/ledger truth.
    pub wallet_ledger_truth_replaced: bool,
    /// Must remain false: this DTO cannot claim finality.
    pub finality_claimed: bool,
}

impl QuickChainDaFallbackPlanV1 {
    /// Validate fallback plan shape and authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainDaFallbackPlanV1.schema",
            &self.schema,
            QUICKCHAIN_DA_FALLBACK_PLAN_SCHEMA,
        )?;
        validate_version("QuickChainDaFallbackPlanV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("fallback_plan_id", &self.fallback_plan_id)?;
        validate_da_time_window(
            self.challenge_window_start_ms,
            self.challenge_window_end_ms,
            self.produced_at_ms,
        )?;
        validate_da_chunks(&self.chunks)?;

        if matches!(
            self.fallback_mode,
            QuickChainDaFallbackModeV1::ArchiveFirstNoPruning
        ) && self.chunks.iter().any(|chunk| !chunk.available)
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "fallback_mode",
                reason: "archive-first no-pruning mode requires all reviewed chunks available",
            });
        }

        if self.pruning_allowed {
            return Err(QuickChainValidationError::InvalidField {
                field: "pruning_allowed",
                reason: "Phase 5 Round 2 models fallback only; pruning remains forbidden",
            });
        }

        if !self.archive_fallback_required {
            return Err(QuickChainValidationError::InvalidField {
                field: "archive_fallback_required",
                reason: "archive fallback must be required before pruning can ever be considered",
            });
        }

        if !self.missing_data_challenge_supported {
            return Err(QuickChainValidationError::InvalidField {
                field: "missing_data_challenge_supported",
                reason: "missing-data challenge support is required in this round",
            });
        }

        if !self.restore_from_archive_tested {
            return Err(QuickChainValidationError::InvalidField {
                field: "restore_from_archive_tested",
                reason: "restore-from-archive dry-run proof is required in this round",
            });
        }

        if !self.dry_run_only {
            return Err(QuickChainValidationError::InvalidField {
                field: "dry_run_only",
                reason: "Phase 5 Round 2 fallback plans must remain dry-run only",
            });
        }

        if self.normal_node_full_archive_required {
            return Err(QuickChainValidationError::InvalidField {
                field: "normal_node_full_archive_required",
                reason: "normal nodes must not be forced to become full archive nodes",
            });
        }

        if self.external_da_truth {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_da_truth",
                reason: "external DA artifacts must not become internal ROC truth",
            });
        }

        if self.external_settlement_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_settlement_authorized",
                reason: "DA fallback must not authorize external settlement",
            });
        }

        if self.bridge_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "bridge_authorized",
                reason: "DA fallback must not authorize bridge behavior",
            });
        }

        if self.balance_mutation_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "balance_mutation_authorized",
                reason: "DA fallback must not authorize balance mutation",
            });
        }

        if self.wallet_ledger_truth_replaced {
            return Err(QuickChainValidationError::InvalidField {
                field: "wallet_ledger_truth_replaced",
                reason: "DA fallback must not replace wallet/ledger truth",
            });
        }

        if self.finality_claimed {
            return Err(QuickChainValidationError::InvalidField {
                field: "finality_claimed",
                reason: "DA fallback must not claim finality",
            });
        }

        Ok(())
    }
}

/// Missing-data challenge report for one fallback-plan chunk.
///
/// This is report-only. It can block pruning in a dry run, but it cannot punish,
/// pay, mutate, settle, unlock, or finalize anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainDaChallengeReportV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id.
    pub epoch_id: String,
    /// Fallback plan id being challenged.
    pub fallback_plan_id: String,
    /// Challenge id.
    pub challenge_id: String,
    /// Checkpoint hash under review.
    pub checkpoint_hash: ContentId,
    /// Chunk id under review.
    pub challenged_chunk_id: String,
    /// Challenge reporter/reference id.
    pub challenger_ref: String,
    /// Evidence artifact CID.
    pub evidence_cid: ContentId,
    /// Caller-supplied submission time.
    pub submitted_at_ms: u64,
    /// Challenge status.
    pub status: QuickChainDaChallengeStatusV1,
    /// Must remain true in this round.
    pub dry_run_only: bool,
    /// Whether the challenge window remains open.
    pub challenge_window_open: bool,
    /// Must remain true in this round.
    pub pruning_blocked: bool,
    /// Must remain false: this report cannot authorize penalties.
    pub penalty_authorized: bool,
    /// Must remain false: this report cannot authorize archive/provider rewards.
    pub archive_reward_authorized: bool,
    /// Must remain false: this report cannot mutate balances.
    pub balance_mutation_authorized: bool,
    /// Must remain false: this report cannot authorize settlement.
    pub external_settlement_authorized: bool,
    /// Must remain false: this report cannot claim finality.
    pub finality_claimed: bool,
}

impl QuickChainDaChallengeReportV1 {
    /// Validate challenge report shape and authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainDaChallengeReportV1.schema",
            &self.schema,
            QUICKCHAIN_DA_CHALLENGE_REPORT_SCHEMA,
        )?;
        validate_version("QuickChainDaChallengeReportV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("fallback_plan_id", &self.fallback_plan_id)?;
        validate_ref("challenge_id", &self.challenge_id)?;
        validate_ref("challenged_chunk_id", &self.challenged_chunk_id)?;
        validate_ref("challenger_ref", &self.challenger_ref)?;

        if self.submitted_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "submitted_at_ms",
                reason: "challenge report submission time must be caller supplied and non-zero",
            });
        }

        match self.status {
            QuickChainDaChallengeStatusV1::MissingDataChallengeOpen => {
                if !self.challenge_window_open || !self.pruning_blocked {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason:
                            "open missing-data challenge requires open window and blocked pruning",
                    });
                }
            }
            QuickChainDaChallengeStatusV1::NoMissingData
            | QuickChainDaChallengeStatusV1::MissingDataRestored
            | QuickChainDaChallengeStatusV1::MissingDataFailedDryRun => {
                if self.challenge_window_open {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "challenge_window_open",
                        reason: "non-open report status must not keep the challenge window open",
                    });
                }
            }
        }

        if !self.dry_run_only {
            return Err(QuickChainValidationError::InvalidField {
                field: "dry_run_only",
                reason: "Phase 5 Round 2 challenge reports must remain dry-run only",
            });
        }

        if !self.pruning_blocked {
            return Err(QuickChainValidationError::InvalidField {
                field: "pruning_blocked",
                reason: "missing-data challenge reports must block pruning in this round",
            });
        }

        if self.penalty_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "penalty_authorized",
                reason: "missing-data challenge reports must not authorize penalties",
            });
        }

        if self.archive_reward_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "archive_reward_authorized",
                reason: "missing-data challenge reports must not authorize rewards",
            });
        }

        if self.balance_mutation_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "balance_mutation_authorized",
                reason: "missing-data challenge reports must not authorize balance mutation",
            });
        }

        if self.external_settlement_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_settlement_authorized",
                reason: "missing-data challenge reports must not authorize external settlement",
            });
        }

        if self.finality_claimed {
            return Err(QuickChainValidationError::InvalidField {
                field: "finality_claimed",
                reason: "missing-data challenge reports must not claim finality",
            });
        }

        Ok(())
    }
}

/// Read-only verification result for one fallback plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainDaFallbackVerificationV1 {
    /// DTO schema.
    pub schema: String,
    /// DTO version.
    pub version: u16,
    /// Internal ROC chain id.
    pub chain_id: String,
    /// Internal epoch id.
    pub epoch_id: String,
    /// Verified fallback plan id.
    pub fallback_plan_id: String,
    /// Checkpoint height verified.
    pub checkpoint_height: u64,
    /// Checkpoint hash verified.
    pub checkpoint_hash: ContentId,
    /// Data-availability root verified.
    pub data_availability_root: ContentId,
    /// Number of chunk commitments checked.
    pub checked_chunk_count: u32,
    /// Must remain true in this round.
    pub pruning_blocked: bool,
    /// Must remain true in this round.
    pub archive_fallback_checked: bool,
    /// Must remain true in this round.
    pub missing_data_challenge_checked: bool,
    /// Must remain true in this round.
    pub restore_path_checked: bool,
    /// Must remain true in this round.
    pub dry_run_only: bool,
    /// Must remain false: verification cannot claim external DA truth.
    pub external_da_truth: bool,
    /// Must remain false: verification cannot authorize settlement.
    pub external_settlement_authorized: bool,
    /// Must remain false: verification cannot authorize bridge behavior.
    pub bridge_authorized: bool,
    /// Must remain false: verification cannot mutate balances.
    pub balance_mutation_authorized: bool,
    /// Must remain false: verification cannot replace wallet/ledger truth.
    pub wallet_ledger_truth_replaced: bool,
    /// Must remain false: verification cannot claim finality.
    pub finality_claimed: bool,
}

impl QuickChainDaFallbackVerificationV1 {
    /// Validate verification shape and authority boundaries.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainDaFallbackVerificationV1.schema",
            &self.schema,
            QUICKCHAIN_DA_FALLBACK_VERIFICATION_SCHEMA,
        )?;
        validate_version("QuickChainDaFallbackVerificationV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_epoch_id(&self.epoch_id)?;
        validate_ref("fallback_plan_id", &self.fallback_plan_id)?;

        if self.checked_chunk_count == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "checked_chunk_count",
                reason: "verification must check at least one DA/archive chunk",
            });
        }

        if !self.pruning_blocked {
            return Err(QuickChainValidationError::InvalidField {
                field: "pruning_blocked",
                reason: "DA fallback verification must keep pruning blocked",
            });
        }

        if !self.archive_fallback_checked {
            return Err(QuickChainValidationError::InvalidField {
                field: "archive_fallback_checked",
                reason: "DA fallback verification must check archive fallback",
            });
        }

        if !self.missing_data_challenge_checked {
            return Err(QuickChainValidationError::InvalidField {
                field: "missing_data_challenge_checked",
                reason: "DA fallback verification must check missing-data challenge support",
            });
        }

        if !self.restore_path_checked {
            return Err(QuickChainValidationError::InvalidField {
                field: "restore_path_checked",
                reason: "DA fallback verification must check restore path",
            });
        }

        if !self.dry_run_only {
            return Err(QuickChainValidationError::InvalidField {
                field: "dry_run_only",
                reason: "DA fallback verification must remain dry-run only",
            });
        }

        if self.external_da_truth {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_da_truth",
                reason: "DA fallback verification must not claim external DA truth",
            });
        }

        if self.external_settlement_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "external_settlement_authorized",
                reason: "DA fallback verification must not authorize settlement",
            });
        }

        if self.bridge_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "bridge_authorized",
                reason: "DA fallback verification must not authorize bridge behavior",
            });
        }

        if self.balance_mutation_authorized {
            return Err(QuickChainValidationError::InvalidField {
                field: "balance_mutation_authorized",
                reason: "DA fallback verification must not authorize balance mutation",
            });
        }

        if self.wallet_ledger_truth_replaced {
            return Err(QuickChainValidationError::InvalidField {
                field: "wallet_ledger_truth_replaced",
                reason: "DA fallback verification must not replace wallet/ledger truth",
            });
        }

        if self.finality_claimed {
            return Err(QuickChainValidationError::InvalidField {
                field: "finality_claimed",
                reason: "DA fallback verification must not claim finality",
            });
        }

        Ok(())
    }
}

fn validate_da_time_window(
    challenge_window_start_ms: u64,
    challenge_window_end_ms: u64,
    produced_at_ms: u64,
) -> QuickChainResult<()> {
    if challenge_window_start_ms == 0 || challenge_window_end_ms == 0 || produced_at_ms == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field: "challenge_window",
            reason: "challenge window and production timestamps must be non-zero",
        });
    }

    if challenge_window_end_ms <= challenge_window_start_ms {
        return Err(QuickChainValidationError::InvalidField {
            field: "challenge_window_end_ms",
            reason: "challenge window end must be after start",
        });
    }

    if produced_at_ms < challenge_window_start_ms || produced_at_ms > challenge_window_end_ms {
        return Err(QuickChainValidationError::InvalidField {
            field: "produced_at_ms",
            reason: "plan production timestamp must be inside the challenge window",
        });
    }

    Ok(())
}

fn validate_da_chunks(chunks: &[QuickChainDaChunkCommitmentV1]) -> QuickChainResult<()> {
    if chunks.is_empty() {
        return Err(QuickChainValidationError::InvalidField {
            field: "chunks",
            reason: "fallback plan must contain at least one DA/archive chunk",
        });
    }

    if chunks.len() > MAX_QUICKCHAIN_DA_CHUNKS {
        return Err(QuickChainValidationError::TooManyItems {
            field: "chunks",
            max: MAX_QUICKCHAIN_DA_CHUNKS,
            actual: chunks.len(),
        });
    }

    let mut previous_chunk_id: Option<&str> = None;
    let mut required_count = 0_usize;

    for chunk in chunks {
        chunk.validate()?;

        if chunk.required_for_challenge {
            required_count += 1;
        }

        if let Some(previous) = previous_chunk_id {
            if previous >= chunk.chunk_id.as_str() {
                return Err(QuickChainValidationError::InvalidField {
                    field: "chunks.chunk_id",
                    reason: "DA/archive chunks must be strictly sorted by chunk_id",
                });
            }
        }

        previous_chunk_id = Some(chunk.chunk_id.as_str());
    }

    if required_count == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field: "chunks.required_for_challenge",
            reason: "fallback plan must include at least one challenge-required chunk",
        });
    }

    Ok(())
}
