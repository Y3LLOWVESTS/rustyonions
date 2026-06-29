//! RO:WHAT — Phase 5 Round 3 read-only external-posture reports for ron-accounting.
//! RO:WHY — Accounting snapshots may be referenced by the selected anchor-only posture, but accounting must not become balance, payout, reward, finality, unlock, market, or outside-truth authority.
//! RO:INTERACTS — RewardSnapshotExport canonical CID helpers and QuickChain Phase 5 Round 3 boundary tests.
//! RO:INVARIANTS — report-only; evidence-only; anchor-only; no wallet/ledger mutation; no payout execution; no paid unlock; no public market.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — tests/quickchain_phase5_external_posture_report_boundary.rs.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::RewardSnapshotExport,
    errors::{Error, Result},
};

/// Schema label for a read-only accounting external-posture report.
pub const RON_ACCOUNTING_QUICKCHAIN_EXTERNAL_POSTURE_REPORT_SCHEMA: &str =
    "ron-accounting.quickchain-external-posture-report.v1";

const MAX_EXTERNAL_POSTURE_REPORT_TOKEN_BYTES: usize = 160;

/// Read-only accounting external-posture report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainExternalPostureReportStatus {
    /// Anchor-only evidence; no accounting authority is granted.
    AnchorOnlyEvidenceOnly,
}

/// Read-only accounting report that relates an accounting snapshot artifact CID
/// to the selected Phase 5 Round 3 external posture.
///
/// This is report metadata only. It is not balance truth, payout truth, reward
/// truth, terminality truth, paid unlock authority, market authority, or
/// outside-claim truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainExternalPostureReport {
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
    /// Reviewed external posture identifier.
    pub posture_id: String,
    /// Reviewed checkpoint commitment hash.
    pub checkpoint_hash: String,
    /// Canonical CID of the referenced accounting snapshot artifact.
    pub accounting_snapshot_cid: String,
    /// Snapshot production timestamp copied from the referenced artifact.
    pub snapshot_produced_at_millis: u64,
    /// Number of contribution rows copied from the referenced artifact.
    pub snapshot_contribution_count: usize,
    /// Honest status for this report.
    pub status: QuickChainExternalPostureReportStatus,
    /// Must remain true: anchor-only was selected.
    pub anchor_only_selected: bool,
    /// Must remain true: this artifact is a report only.
    pub report_only: bool,
    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,
    /// Must remain true: wallet/ledger truth remains canonical.
    pub wallet_ledger_truth_canonical: bool,
    /// Must remain false: accounting is not balance truth.
    pub balance_truth: bool,
    /// Must remain false: accounting does not affect wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting does not affect ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting does not execute payouts.
    pub payout_side_effect: bool,
    /// Must remain false: posture evidence does not create reward truth.
    pub reward_truth: bool,
    /// Must remain false: posture evidence does not create terminality truth.
    pub terminality_truth: bool,
    /// Must remain false: posture evidence does not create outside-claim truth.
    pub external_claim_truth: bool,
    /// Must remain false: posture evidence does not unlock paid content.
    pub paid_unlock_authority: bool,
    /// Must remain false: posture evidence does not create outside settlement.
    pub outside_settlement: bool,
    /// Must remain false: posture evidence does not create bridge authority.
    pub bridge_authority: bool,
    /// Must remain false: posture evidence does not create outside program authority.
    pub outside_program_authority: bool,
    /// Must remain false: posture evidence does not create listing authority.
    pub listing_authority: bool,
    /// Must remain false: posture evidence does not create a public market.
    pub public_market: bool,
    /// Must remain false: posture evidence does not create liquidity behavior.
    pub liquidity_enabled: bool,
    /// Must remain false: posture evidence does not create bonded-economy authority.
    pub bonded_economy_authority: bool,
}

impl QuickChainExternalPostureReport {
    /// Build a read-only external-posture report for an existing reward/accounting snapshot artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_snapshot(
        produced_at_ms: u64,
        chain_id: impl Into<String>,
        epoch_id: impl Into<String>,
        report_source: impl Into<String>,
        posture_id: impl Into<String>,
        checkpoint_hash: impl Into<String>,
        snapshot: &RewardSnapshotExport,
    ) -> Result<Self> {
        let canonical_snapshot = snapshot.canonicalized()?;
        let report = Self {
            schema: RON_ACCOUNTING_QUICKCHAIN_EXTERNAL_POSTURE_REPORT_SCHEMA.to_owned(),
            produced_at_ms,
            chain_id: chain_id.into(),
            epoch_id: epoch_id.into(),
            report_source: report_source.into(),
            posture_id: posture_id.into(),
            checkpoint_hash: checkpoint_hash.into(),
            accounting_snapshot_cid: canonical_snapshot.canonical_cid()?,
            snapshot_produced_at_millis: canonical_snapshot.produced_at_millis,
            snapshot_contribution_count: canonical_snapshot.contribution_count(),
            status: QuickChainExternalPostureReportStatus::AnchorOnlyEvidenceOnly,
            anchor_only_selected: true,
            report_only: true,
            evidence_only: true,
            wallet_ledger_truth_canonical: true,
            balance_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            reward_truth: false,
            terminality_truth: false,
            external_claim_truth: false,
            paid_unlock_authority: false,
            outside_settlement: false,
            bridge_authority: false,
            outside_program_authority: false,
            listing_authority: false,
            public_market: false,
            liquidity_enabled: false,
            bonded_economy_authority: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate report shape and no-authority flags.
    pub fn validate(&self) -> Result<()> {
        if self.schema != RON_ACCOUNTING_QUICKCHAIN_EXTERNAL_POSTURE_REPORT_SCHEMA {
            return Err(Error::schema(
                "invalid QuickChain external-posture report schema",
            ));
        }

        if self.produced_at_ms == 0 {
            return Err(Error::schema(
                "external-posture report produced_at_ms must be nonzero",
            ));
        }

        if self.snapshot_produced_at_millis == 0 {
            return Err(Error::schema(
                "external-posture report snapshot_produced_at_millis must be nonzero",
            ));
        }

        if self.snapshot_contribution_count == 0 {
            return Err(Error::schema(
                "external-posture report must reference a nonempty accounting snapshot",
            ));
        }

        validate_external_posture_report_token("chain_id", &self.chain_id)?;
        validate_external_posture_report_token("epoch_id", &self.epoch_id)?;
        validate_external_posture_report_token("report_source", &self.report_source)?;
        validate_external_posture_report_token("posture_id", &self.posture_id)?;

        validate_b3("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3("accounting_snapshot_cid", &self.accounting_snapshot_cid)?;

        if self.status != QuickChainExternalPostureReportStatus::AnchorOnlyEvidenceOnly {
            return Err(Error::schema(
                "external-posture report must remain anchor-only evidence",
            ));
        }

        if !self.anchor_only_selected {
            return Err(Error::schema(
                "external-posture report must select anchor-only posture",
            ));
        }

        if !self.report_only {
            return Err(Error::schema(
                "external-posture report must remain report-only",
            ));
        }

        if !self.evidence_only {
            return Err(Error::schema(
                "external-posture report must remain evidence-only",
            ));
        }

        if !self.wallet_ledger_truth_canonical {
            return Err(Error::schema(
                "external-posture report must keep wallet/ledger truth canonical",
            ));
        }

        if self.balance_truth {
            return Err(Error::schema(
                "external-posture report must not claim balance truth",
            ));
        }

        if self.wallet_side_effect {
            return Err(Error::schema(
                "external-posture report must not claim wallet side effect",
            ));
        }

        if self.ledger_side_effect {
            return Err(Error::schema(
                "external-posture report must not claim ledger side effect",
            ));
        }

        if self.payout_side_effect {
            return Err(Error::schema(
                "external-posture report must not claim payout side effect",
            ));
        }

        if self.reward_truth {
            return Err(Error::schema(
                "external-posture report must not claim reward truth",
            ));
        }

        if self.terminality_truth {
            return Err(Error::schema(
                "external-posture report must not claim terminality truth",
            ));
        }

        if self.external_claim_truth {
            return Err(Error::schema(
                "external-posture report must not claim outside-claim truth",
            ));
        }

        if self.paid_unlock_authority {
            return Err(Error::schema(
                "external-posture report must not claim paid unlock authority",
            ));
        }

        if self.outside_settlement {
            return Err(Error::schema(
                "external-posture report must not claim outside settlement",
            ));
        }

        if self.bridge_authority {
            return Err(Error::schema(
                "external-posture report must not claim bridge authority",
            ));
        }

        if self.outside_program_authority {
            return Err(Error::schema(
                "external-posture report must not claim outside program authority",
            ));
        }

        if self.listing_authority {
            return Err(Error::schema(
                "external-posture report must not claim listing authority",
            ));
        }

        if self.public_market {
            return Err(Error::schema(
                "external-posture report must not claim public market behavior",
            ));
        }

        if self.liquidity_enabled {
            return Err(Error::schema(
                "external-posture report must not claim liquidity behavior",
            ));
        }

        if self.bonded_economy_authority {
            return Err(Error::schema(
                "external-posture report must not claim bonded-economy authority",
            ));
        }

        Ok(())
    }
}

fn validate_external_posture_report_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_EXTERNAL_POSTURE_REPORT_TOKEN_BYTES {
        return Err(Error::schema(format!(
            "{field} must be 1..={MAX_EXTERNAL_POSTURE_REPORT_TOKEN_BYTES} bytes"
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
