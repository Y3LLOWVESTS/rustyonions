//! RO:WHAT — Phase 5 Round 2 read-only DA/archive/challenge fallback reports for ron-accounting.
//! RO:WHY — Accounting snapshots may be referenced by DA fallback metadata, but accounting must not become balance, payout, reward, finality, deletion, or outside-truth authority.
//! RO:INTERACTS — RewardSnapshotExport canonical CID helpers and QuickChain Phase 5 Round 2 boundary tests.
//! RO:INVARIANTS — report-only; evidence-only; no wallet/ledger mutation; no payout execution; no paid unlock; no deletion authority; no outside truth.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — tests/quickchain_phase5_da_fallback_report_boundary.rs.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::RewardSnapshotExport,
    errors::{Error, Result},
};

/// Schema label for a read-only accounting DA fallback report.
pub const RON_ACCOUNTING_QUICKCHAIN_DA_FALLBACK_REPORT_SCHEMA: &str =
    "ron-accounting.quickchain-da-fallback-report.v1";

const MAX_DA_FALLBACK_REPORT_TOKEN_BYTES: usize = 160;

/// Read-only accounting report that relates an accounting snapshot artifact CID
/// to DA/archive/challenge fallback evidence.
///
/// This is report metadata only. It is not balance truth, payout truth, reward
/// truth, terminality truth, paid unlock authority, deletion authority, or
/// outside-claim truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainDaFallbackReport {
    /// Report schema.
    pub schema: String,
    /// Report production timestamp supplied by the caller.
    pub produced_at_ms: u64,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Explicit epoch id context.
    pub epoch_id: String,
    /// Reporting source label.
    pub report_source: String,
    /// Reviewed fallback plan identifier.
    pub fallback_plan_id: String,
    /// Reviewed checkpoint commitment hash.
    pub checkpoint_hash: String,
    /// Reviewed data availability root.
    pub data_availability_root: String,
    /// Optional challenged chunk identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenged_chunk_id: Option<String>,
    /// Canonical CID of the referenced accounting snapshot artifact.
    pub accounting_snapshot_cid: String,
    /// Snapshot production timestamp copied from the referenced artifact.
    pub snapshot_produced_at_millis: u64,
    /// Number of contribution rows copied from the referenced artifact.
    pub snapshot_contribution_count: usize,
    /// Must remain true: this artifact is a report only.
    pub report_only: bool,
    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,
    /// Must remain true: archive fallback was checked.
    pub archive_fallback_checked: bool,
    /// Must remain true: missing-data challenge handling was checked.
    pub missing_data_challenge_checked: bool,
    /// Must remain true: restore path was checked.
    pub restore_path_checked: bool,
    /// Must remain true: this report blocks deletion/unavailability shortcuts.
    pub pruning_blocked: bool,
    /// Must remain false: accounting is not balance truth.
    pub balance_truth: bool,
    /// Must remain false: accounting does not affect wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting does not affect ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting does not execute payouts.
    pub payout_side_effect: bool,
    /// Must remain false: fallback evidence does not create reward truth.
    pub reward_truth: bool,
    /// Must remain false: fallback evidence does not create terminality truth.
    pub terminality_truth: bool,
    /// Must remain false: fallback evidence does not create outside-claim truth.
    pub external_claim_truth: bool,
    /// Must remain false: fallback evidence does not unlock paid content.
    pub paid_unlock_authority: bool,
    /// Must remain false: accounting does not authorize deletion/unavailability shortcuts.
    pub pruning_authority: bool,
    /// Must remain false: outside DA material is not accounting truth.
    pub outside_data_availability_truth: bool,
    /// Must remain false: fallback evidence does not create outside settlement.
    pub outside_settlement: bool,
}

impl QuickChainDaFallbackReport {
    /// Build a read-only DA fallback report for an existing reward/accounting snapshot artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_snapshot(
        produced_at_ms: u64,
        chain_id: impl Into<String>,
        epoch_id: impl Into<String>,
        report_source: impl Into<String>,
        fallback_plan_id: impl Into<String>,
        checkpoint_hash: impl Into<String>,
        data_availability_root: impl Into<String>,
        challenged_chunk_id: Option<String>,
        snapshot: &RewardSnapshotExport,
    ) -> Result<Self> {
        let canonical_snapshot = snapshot.canonicalized()?;
        let report = Self {
            schema: RON_ACCOUNTING_QUICKCHAIN_DA_FALLBACK_REPORT_SCHEMA.to_owned(),
            produced_at_ms,
            chain_id: chain_id.into(),
            epoch_id: epoch_id.into(),
            report_source: report_source.into(),
            fallback_plan_id: fallback_plan_id.into(),
            checkpoint_hash: checkpoint_hash.into(),
            data_availability_root: data_availability_root.into(),
            challenged_chunk_id,
            accounting_snapshot_cid: canonical_snapshot.canonical_cid()?,
            snapshot_produced_at_millis: canonical_snapshot.produced_at_millis,
            snapshot_contribution_count: canonical_snapshot.contribution_count(),
            report_only: true,
            evidence_only: true,
            archive_fallback_checked: true,
            missing_data_challenge_checked: true,
            restore_path_checked: true,
            pruning_blocked: true,
            balance_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            reward_truth: false,
            terminality_truth: false,
            external_claim_truth: false,
            paid_unlock_authority: false,
            pruning_authority: false,
            outside_data_availability_truth: false,
            outside_settlement: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate report shape and no-authority flags.
    pub fn validate(&self) -> Result<()> {
        if self.schema != RON_ACCOUNTING_QUICKCHAIN_DA_FALLBACK_REPORT_SCHEMA {
            return Err(Error::schema(
                "invalid QuickChain DA fallback report schema",
            ));
        }

        if self.produced_at_ms == 0 {
            return Err(Error::schema(
                "DA fallback report produced_at_ms must be nonzero",
            ));
        }

        if self.snapshot_produced_at_millis == 0 {
            return Err(Error::schema(
                "DA fallback report snapshot_produced_at_millis must be nonzero",
            ));
        }

        if self.snapshot_contribution_count == 0 {
            return Err(Error::schema(
                "DA fallback report must reference a nonempty accounting snapshot",
            ));
        }

        validate_da_fallback_report_token("chain_id", &self.chain_id)?;
        validate_da_fallback_report_token("epoch_id", &self.epoch_id)?;
        validate_da_fallback_report_token("report_source", &self.report_source)?;
        validate_da_fallback_report_token("fallback_plan_id", &self.fallback_plan_id)?;

        if let Some(challenged_chunk_id) = self.challenged_chunk_id.as_deref() {
            validate_da_fallback_report_token("challenged_chunk_id", challenged_chunk_id)?;
        }

        validate_b3("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3("data_availability_root", &self.data_availability_root)?;
        validate_b3("accounting_snapshot_cid", &self.accounting_snapshot_cid)?;

        if !self.report_only {
            return Err(Error::schema("DA fallback report must remain report-only"));
        }

        if !self.evidence_only {
            return Err(Error::schema(
                "DA fallback report must remain evidence-only",
            ));
        }

        if !self.archive_fallback_checked {
            return Err(Error::schema(
                "DA fallback report must check archive fallback",
            ));
        }

        if !self.missing_data_challenge_checked {
            return Err(Error::schema(
                "DA fallback report must check missing-data challenge handling",
            ));
        }

        if !self.restore_path_checked {
            return Err(Error::schema("DA fallback report must check restore path"));
        }

        if !self.pruning_blocked {
            return Err(Error::schema(
                "DA fallback report must keep pruning blocked",
            ));
        }

        if self.balance_truth {
            return Err(Error::schema(
                "DA fallback report must not claim balance truth",
            ));
        }

        if self.wallet_side_effect {
            return Err(Error::schema(
                "DA fallback report must not claim wallet side effect",
            ));
        }

        if self.ledger_side_effect {
            return Err(Error::schema(
                "DA fallback report must not claim ledger side effect",
            ));
        }

        if self.payout_side_effect {
            return Err(Error::schema(
                "DA fallback report must not claim payout side effect",
            ));
        }

        if self.reward_truth {
            return Err(Error::schema(
                "DA fallback report must not claim reward truth",
            ));
        }

        if self.terminality_truth {
            return Err(Error::schema(
                "DA fallback report must not claim terminality truth",
            ));
        }

        if self.external_claim_truth {
            return Err(Error::schema(
                "DA fallback report must not claim outside-claim truth",
            ));
        }

        if self.paid_unlock_authority {
            return Err(Error::schema(
                "DA fallback report must not claim paid unlock authority",
            ));
        }

        if self.pruning_authority {
            return Err(Error::schema(
                "DA fallback report must not claim pruning authority",
            ));
        }

        if self.outside_data_availability_truth {
            return Err(Error::schema(
                "DA fallback report must not claim outside DA truth",
            ));
        }

        if self.outside_settlement {
            return Err(Error::schema(
                "DA fallback report must not claim outside settlement",
            ));
        }

        Ok(())
    }
}

fn validate_da_fallback_report_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_DA_FALLBACK_REPORT_TOKEN_BYTES {
        return Err(Error::schema(format!(
            "{field} must be 1..={MAX_DA_FALLBACK_REPORT_TOKEN_BYTES} bytes"
        )));
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/' | '@'))
    {
        return Err(Error::schema(format!(
            "{field} contains unsupported characters"
        )));
    }

    Ok(())
}

fn validate_b3(field: &str, value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(Error::schema(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    };

    if hex.len() != 64 || !hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(Error::schema(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    }

    Ok(())
}
