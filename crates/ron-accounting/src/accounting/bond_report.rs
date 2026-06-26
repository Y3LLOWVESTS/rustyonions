//! RO:WHAT — Phase 4 Round 1 read-only bond report DTOs for ron-accounting.
//! RO:WHY — Accounting may summarize bond-model facts for reports, but it must
//! not become bond truth, balance truth, wallet authority, ledger authority, or
//! payout side effect.
//! RO:INTERACTS — accounting module exports and QuickChain Phase 4 boundary tests.
//! RO:INVARIANTS — report-only; integer minor-unit strings; no wallet/ledger
//! mutation; no receipts; no public market; no liquidity.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — tests/quickchain_phase4_bond_report_boundary.rs.

use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};

/// Schema label for a read-only accounting bond report.
pub const RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA: &str =
    "ron-accounting.quickchain-bond-report.v1";

const MAX_REPORT_TOKEN_BYTES: usize = 128;

/// Read-only accounting report for Phase 4 bond-model totals.
///
/// This is a reporting artifact only. It is not balance truth, not a wallet
/// receipt, not ledger state, not payout side effect, and not terminal or external-claim truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondReport {
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
    /// Number of internal bond-model accounts summarized.
    pub account_count: u64,
    /// Total locked amount as integer minor-unit string.
    pub locked_minor: String,
    /// Total pending unlock amount as integer minor-unit string.
    pub pending_unlock_minor: String,
    /// Total evidence-reserved amount as integer minor-unit string.
    pub evidence_reserved_minor: String,
    /// Must remain true: this artifact is a report only.
    pub report_only: bool,
    /// Must remain false: accounting is not balance truth.
    pub balance_truth: bool,
    /// Must remain false: accounting does not mutate wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting does not mutate ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting does not execute payouts.
    pub payout_side_effect: bool,
}

impl QuickChainBondReport {
    /// Build a read-only bond report with all authority flags disabled.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        produced_at_ms: u64,
        chain_id: impl Into<String>,
        epoch_id: impl Into<String>,
        report_source: impl Into<String>,
        account_count: u64,
        locked_minor: impl Into<String>,
        pending_unlock_minor: impl Into<String>,
        evidence_reserved_minor: impl Into<String>,
    ) -> Result<Self> {
        let report = Self {
            schema: RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA.to_owned(),
            produced_at_ms,
            chain_id: chain_id.into(),
            epoch_id: epoch_id.into(),
            report_source: report_source.into(),
            account_count,
            locked_minor: locked_minor.into(),
            pending_unlock_minor: pending_unlock_minor.into(),
            evidence_reserved_minor: evidence_reserved_minor.into(),
            report_only: true,
            balance_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate read-only report shape.
    pub fn validate(&self) -> Result<()> {
        if self.schema != RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA {
            return Err(Error::schema("invalid QuickChain bond report schema"));
        }

        if self.produced_at_ms == 0 {
            return Err(Error::schema("bond report produced_at_ms must be nonzero"));
        }

        validate_report_token("chain_id", &self.chain_id)?;
        validate_report_token("epoch_id", &self.epoch_id)?;
        validate_report_token("report_source", &self.report_source)?;

        let locked = parse_minor_units("locked_minor", &self.locked_minor)?;
        let pending = parse_minor_units("pending_unlock_minor", &self.pending_unlock_minor)?;
        let evidence_reserved =
            parse_minor_units("evidence_reserved_minor", &self.evidence_reserved_minor)?;

        let components = pending
            .checked_add(evidence_reserved)
            .ok_or_else(|| Error::schema("bond report component overflow"))?;

        if components > locked {
            return Err(Error::schema(
                "bond report pending/evidence components exceed locked amount",
            ));
        }

        if self.account_count == 0 && locked != 0 {
            return Err(Error::schema(
                "empty bond report must not carry nonzero locked amount",
            ));
        }

        if !self.report_only {
            return Err(Error::schema("bond report must remain report-only"));
        }

        if self.balance_truth {
            return Err(Error::schema("bond report must not claim balance truth"));
        }

        if self.wallet_side_effect {
            return Err(Error::schema(
                "bond report must not claim wallet side effect",
            ));
        }

        if self.ledger_side_effect {
            return Err(Error::schema(
                "bond report must not claim ledger side effect",
            ));
        }

        if self.payout_side_effect {
            return Err(Error::schema(
                "bond report must not claim payout side effect",
            ));
        }

        Ok(())
    }
}

/// Schema label for a read-only accounting disputed-bond report.
pub const RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA: &str =
    "ron-accounting.quickchain-bond-dispute-report.v1";

/// Read-only disputed-bond report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainBondDisputeReportStatus {
    /// Challenge window is open and no frozen amount is reported.
    ChallengeOpen,
    /// A simulated freeze is pending appeal.
    FrozenPendingAppeal,
    /// Appeal window is open for a simulated frozen amount.
    AppealOpen,
    /// Dispute resolved with no penalty effect.
    ResolvedNoPenalty,
    /// Irreversible penalty execution was rejected.
    ResolvedPenaltyRejected,
}

/// Read-only accounting report for disputed-bond simulation state.
///
/// This is a reporting artifact only. It is not balance truth, not wallet truth,
/// not ledger truth, not payout authority, not terminal or external-claim truth, and not a
/// live enforcement decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainBondDisputeReport {
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
    /// Dispute identifier.
    pub dispute_id: String,
    /// Bond account identifier.
    pub bond_account_id: String,
    /// Read-only dispute status.
    pub status: QuickChainBondDisputeReportStatus,
    /// Disputed amount as integer minor-unit string.
    pub disputed_minor: String,
    /// Simulated frozen amount as integer minor-unit string.
    pub frozen_minor: String,
    /// Whether the report says a challenge window is open.
    pub challenge_window_open: bool,
    /// Whether the report says an appeal window is open.
    pub appeal_window_open: bool,
    /// Must remain true: this artifact is a report only.
    pub report_only: bool,
    /// Must remain false: accounting is not balance truth.
    pub balance_truth: bool,
    /// Must remain false: accounting does not affect wallets.
    pub wallet_side_effect: bool,
    /// Must remain false: accounting does not affect ledger truth.
    pub ledger_side_effect: bool,
    /// Must remain false: accounting does not execute payouts.
    pub payout_side_effect: bool,
    /// Must remain false: accounting does not create terminality truth.
    pub terminality_truth: bool,
    /// Must remain false: accounting does not create external-claim truth.
    pub external_claim_truth: bool,
}

impl QuickChainBondDisputeReport {
    /// Build a read-only disputed-bond report with all authority flags disabled.
    #[allow(clippy::too_many_arguments)]
    pub fn new_dispute_report(
        produced_at_ms: u64,
        chain_id: impl Into<String>,
        epoch_id: impl Into<String>,
        report_source: impl Into<String>,
        dispute_id: impl Into<String>,
        bond_account_id: impl Into<String>,
        status: QuickChainBondDisputeReportStatus,
        disputed_minor: impl Into<String>,
        frozen_minor: impl Into<String>,
        challenge_window_open: bool,
        appeal_window_open: bool,
    ) -> Result<Self> {
        let report = Self {
            schema: RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA.to_owned(),
            produced_at_ms,
            chain_id: chain_id.into(),
            epoch_id: epoch_id.into(),
            report_source: report_source.into(),
            dispute_id: dispute_id.into(),
            bond_account_id: bond_account_id.into(),
            status,
            disputed_minor: disputed_minor.into(),
            frozen_minor: frozen_minor.into(),
            challenge_window_open,
            appeal_window_open,
            report_only: true,
            balance_truth: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            terminality_truth: false,
            external_claim_truth: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate read-only disputed-bond report shape.
    pub fn validate(&self) -> Result<()> {
        if self.schema != RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA {
            return Err(Error::schema(
                "invalid QuickChain bond dispute report schema",
            ));
        }

        if self.produced_at_ms == 0 {
            return Err(Error::schema(
                "bond dispute report produced_at_ms must be nonzero",
            ));
        }

        validate_report_token("chain_id", &self.chain_id)?;
        validate_report_token("epoch_id", &self.epoch_id)?;
        validate_report_token("report_source", &self.report_source)?;
        validate_report_token("dispute_id", &self.dispute_id)?;
        validate_report_token("bond_account_id", &self.bond_account_id)?;

        let disputed = parse_minor_units("disputed_minor", &self.disputed_minor)?;
        let frozen = parse_minor_units("frozen_minor", &self.frozen_minor)?;

        if disputed == 0 {
            return Err(Error::schema(
                "bond dispute report disputed_minor must be nonzero",
            ));
        }

        if frozen > disputed {
            return Err(Error::schema(
                "bond dispute report frozen_minor exceeds disputed_minor",
            ));
        }

        match self.status {
            QuickChainBondDisputeReportStatus::ChallengeOpen => {
                if !self.challenge_window_open || self.appeal_window_open || frozen != 0 {
                    return Err(Error::schema(
                        "challenge_open report requires challenge window only and zero frozen amount",
                    ));
                }
            }
            QuickChainBondDisputeReportStatus::FrozenPendingAppeal
            | QuickChainBondDisputeReportStatus::AppealOpen => {
                if !self.appeal_window_open || frozen == 0 {
                    return Err(Error::schema(
                        "appeal/frozen report requires open appeal window and nonzero frozen amount",
                    ));
                }
            }
            QuickChainBondDisputeReportStatus::ResolvedNoPenalty
            | QuickChainBondDisputeReportStatus::ResolvedPenaltyRejected => {
                if self.challenge_window_open || self.appeal_window_open || frozen != 0 {
                    return Err(Error::schema(
                        "terminal dispute report must close windows and carry zero frozen amount",
                    ));
                }
            }
        }

        if !self.report_only {
            return Err(Error::schema("bond dispute report must remain report-only"));
        }

        if self.balance_truth {
            return Err(Error::schema(
                "bond dispute report must not claim balance truth",
            ));
        }

        if self.wallet_side_effect {
            return Err(Error::schema(
                "bond dispute report must not claim wallet side effect",
            ));
        }

        if self.ledger_side_effect {
            return Err(Error::schema(
                "bond dispute report must not claim ledger side effect",
            ));
        }

        if self.payout_side_effect {
            return Err(Error::schema(
                "bond dispute report must not claim payout side effect",
            ));
        }

        if self.terminality_truth {
            return Err(Error::schema(
                "bond dispute report must not claim terminality truth",
            ));
        }

        if self.external_claim_truth {
            return Err(Error::schema(
                "bond dispute report must not claim external-claim truth",
            ));
        }

        Ok(())
    }
}
fn validate_report_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_REPORT_TOKEN_BYTES {
        return Err(Error::schema(format!(
            "{field} must be 1..={MAX_REPORT_TOKEN_BYTES} bytes"
        )));
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'))
    {
        return Err(Error::schema(format!(
            "{field} contains unsupported characters"
        )));
    }

    Ok(())
}

fn parse_minor_units(field: &str, value: &str) -> Result<u128> {
    if value.is_empty() {
        return Err(Error::schema(format!("{field} must not be empty")));
    }

    if value.len() > 1 && value.starts_with('0') {
        return Err(Error::schema(format!(
            "{field} must be canonical integer minor units"
        )));
    }

    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(Error::schema(format!(
            "{field} must be integer minor units"
        )));
    }

    value
        .parse::<u128>()
        .map_err(|err| Error::schema(format!("{field} is not a u128 minor-unit value: {err}")))
}
