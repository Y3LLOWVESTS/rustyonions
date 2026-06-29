//! RO:WHAT — Phase 5 Round 1 read-only anchor evidence reports for ron-accounting.
//! RO:WHY — Accounting snapshots may be referenced by anchor dry-run metadata, but accounting must not become settlement, payout, or balance truth.
//! RO:INTERACTS — RewardSnapshotExport canonical CID helpers and QuickChain Phase 5 boundary tests.
//! RO:INVARIANTS — report-only; evidence-only; no wallet/ledger mutation; no payout execution; no finality or outside-claim truth.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — tests/quickchain_phase5_anchor_report_boundary.rs.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::RewardSnapshotExport,
    errors::{Error, Result},
};

/// Schema label for a read-only accounting anchor report.
pub const RON_ACCOUNTING_QUICKCHAIN_ANCHOR_REPORT_SCHEMA: &str =
    "ron-accounting.quickchain-anchor-report.v1";

const MAX_ANCHOR_REPORT_TOKEN_BYTES: usize = 160;

/// Read-only accounting report that relates an accounting snapshot artifact CID
/// to an anchor dry-run checkpoint commitment.
///
/// This is report metadata only. It is not balance truth, payout truth, reward
/// truth, terminality truth, paid unlock authority, or outside-claim truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainAnchorReport {
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
    /// Dry-run anchor identifier.
    pub anchor_id: String,
    /// Reviewed checkpoint commitment hash.
    pub checkpoint_hash: String,
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
    /// Must remain false: accounting is not balance truth.
    pub balance_truth: bool,
    /// Must remain false: accounting does not affect wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting does not affect ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting does not execute payouts.
    pub payout_side_effect: bool,
    /// Must remain false: anchor evidence does not create reward truth.
    pub reward_truth: bool,
    /// Must remain false: anchor evidence does not create terminality truth.
    pub terminality_truth: bool,
    /// Must remain false: anchor evidence does not create outside-claim truth.
    pub external_claim_truth: bool,
    /// Must remain false: anchor evidence does not unlock paid content.
    pub paid_unlock_authority: bool,
}

impl QuickChainAnchorReport {
    /// Build a read-only anchor report for an existing reward/accounting snapshot artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_snapshot(
        produced_at_ms: u64,
        chain_id: impl Into<String>,
        epoch_id: impl Into<String>,
        report_source: impl Into<String>,
        anchor_id: impl Into<String>,
        checkpoint_hash: impl Into<String>,
        snapshot: &RewardSnapshotExport,
    ) -> Result<Self> {
        let canonical_snapshot = snapshot.canonicalized()?;
        let report = Self {
            schema: RON_ACCOUNTING_QUICKCHAIN_ANCHOR_REPORT_SCHEMA.to_owned(),
            produced_at_ms,
            chain_id: chain_id.into(),
            epoch_id: epoch_id.into(),
            report_source: report_source.into(),
            anchor_id: anchor_id.into(),
            checkpoint_hash: checkpoint_hash.into(),
            accounting_snapshot_cid: canonical_snapshot.canonical_cid()?,
            snapshot_produced_at_millis: canonical_snapshot.produced_at_millis,
            snapshot_contribution_count: canonical_snapshot.contribution_count(),
            report_only: true,
            evidence_only: true,
            balance_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            reward_truth: false,
            terminality_truth: false,
            external_claim_truth: false,
            paid_unlock_authority: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate report shape and no-authority flags.
    pub fn validate(&self) -> Result<()> {
        if self.schema != RON_ACCOUNTING_QUICKCHAIN_ANCHOR_REPORT_SCHEMA {
            return Err(Error::schema("invalid QuickChain anchor report schema"));
        }

        if self.produced_at_ms == 0 {
            return Err(Error::schema(
                "anchor report produced_at_ms must be nonzero",
            ));
        }

        if self.snapshot_produced_at_millis == 0 {
            return Err(Error::schema(
                "anchor report snapshot_produced_at_millis must be nonzero",
            ));
        }

        if self.snapshot_contribution_count == 0 {
            return Err(Error::schema(
                "anchor report must reference a nonempty accounting snapshot",
            ));
        }

        validate_anchor_report_token("chain_id", &self.chain_id)?;
        validate_anchor_report_token("epoch_id", &self.epoch_id)?;
        validate_anchor_report_token("report_source", &self.report_source)?;
        validate_anchor_report_token("anchor_id", &self.anchor_id)?;
        validate_b3("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3("accounting_snapshot_cid", &self.accounting_snapshot_cid)?;

        if !self.report_only {
            return Err(Error::schema("anchor report must remain report-only"));
        }

        if !self.evidence_only {
            return Err(Error::schema("anchor report must remain evidence-only"));
        }

        if self.balance_truth {
            return Err(Error::schema("anchor report must not claim balance truth"));
        }

        if self.wallet_side_effect {
            return Err(Error::schema(
                "anchor report must not claim wallet side effect",
            ));
        }

        if self.ledger_side_effect {
            return Err(Error::schema(
                "anchor report must not claim ledger side effect",
            ));
        }

        if self.payout_side_effect {
            return Err(Error::schema(
                "anchor report must not claim payout side effect",
            ));
        }

        if self.reward_truth {
            return Err(Error::schema("anchor report must not claim reward truth"));
        }

        if self.terminality_truth {
            return Err(Error::schema(
                "anchor report must not claim terminality truth",
            ));
        }

        if self.external_claim_truth {
            return Err(Error::schema(
                "anchor report must not claim outside-claim truth",
            ));
        }

        if self.paid_unlock_authority {
            return Err(Error::schema(
                "anchor report must not claim paid unlock authority",
            ));
        }

        Ok(())
    }
}

fn validate_anchor_report_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_ANCHOR_REPORT_TOKEN_BYTES {
        return Err(Error::schema(format!(
            "{field} must be 1..={MAX_ANCHOR_REPORT_TOKEN_BYTES} bytes"
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
