//! RO:WHAT — Settlement intent DTOs and idempotent in-memory emitter seam.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Rewarder emits deterministic wallet intents instead of mutating ledger directly.
//! RO:INTERACTS — outputs::manifest, future svc-wallet client, http handlers.
//! RO:INVARIANTS — same run_key emits at most once; duplicate is dup; dry-run emits nothing; wallet idem keys <=64 bytes.
//! RO:METRICS — ledger_intents_total{result} and settlement_intents_planned_total updated by callers.
//! RO:CONFIG — future endpoint from ingress.ledger_base_url / wallet base URL.
//! RO:SECURITY — no ambient credentials; future egress macaroon must be attenuated.
//! RO:TEST — tests/unit/settlement.rs and tests/integration/egress_dedupe.rs.

use std::collections::HashSet;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::core::algebra::AmountMinor;
use crate::inputs::RewardFundingSource;
use crate::outputs::manifest::RewardManifest;
use crate::{Result, RewarderError};

/// Internal ROC asset code.
///
/// Keep this lowercase because `svc-wallet` currently uses `asset = "roc"` by default.
pub const ROC_ASSET: &str = "roc";

/// Relative wallet issue route used by the future HTTP wallet client.
pub const WALLET_ISSUE_PATH: &str = "/v1/issue";

/// One wallet/ledger issuance intent produced by a reward manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementIntent {
    /// Batch/run idempotency key, normally the run_key.
    pub run_key: String,
    /// Per-recipient idempotency key accepted by svc-wallet.
    pub idempotency_key: String,
    /// Epoch id.
    pub epoch_id: String,
    /// Manifest commitment this intent belongs to.
    pub manifest_commitment: String,
    /// Declared funding provenance inherited from the manifest policy.
    pub funding_source: RewardFundingSource,
    /// Recipient account.
    pub to: String,
    /// Asset code, currently ROC.
    pub asset: String,
    /// Amount to issue.
    pub amount_minor_units: AmountMinor,
    /// Human-readable deterministic memo.
    pub memo: String,
}

impl SettlementIntent {
    /// Convert this rewarder intent into the exact wallet issue request DTO shape.
    #[must_use]
    pub fn to_wallet_issue_request(&self) -> WalletIssueRequest {
        WalletIssueRequest {
            to: self.to.clone(),
            asset: self.asset.clone(),
            amount_minor: self.amount_minor_units.get().to_string(),
            idempotency_key: Some(self.idempotency_key.clone()),
            memo: Some(self.memo.clone()),
        }
    }
}

/// Deterministic batch of wallet issuance intents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementBatch {
    /// Batch idempotency key.
    pub run_key: String,
    /// Epoch id.
    pub epoch_id: String,
    /// Manifest commitment.
    pub manifest_commitment: String,
    /// Declared funding provenance for this settlement plan.
    pub funding_source: RewardFundingSource,
    /// Total amount represented by all intents.
    pub total_minor_units: AmountMinor,
    /// Per-recipient issuance intents.
    pub intents: Vec<SettlementIntent>,
}

impl SettlementBatch {
    /// Build deterministic issuance intents from a sealed manifest.
    pub fn from_manifest(manifest: &RewardManifest) -> Result<Self> {
        if manifest.commitment.trim().is_empty() {
            return Err(RewarderError::Internal(
                "cannot plan settlement intents for an unsealed manifest".into(),
            ));
        }

        let mut intents = Vec::with_capacity(manifest.payouts.len());
        for payout in &manifest.payouts {
            if payout.amount_minor_units.get() == 0 {
                return Err(RewarderError::Quarantined(
                    "zero payout cannot become settlement intent".into(),
                ));
            }

            let idempotency_key = intent_idempotency_key(
                &manifest.run_key,
                &manifest.commitment,
                &payout.account,
                payout.amount_minor_units,
            );

            intents.push(SettlementIntent {
                run_key: manifest.run_key.clone(),
                idempotency_key,
                epoch_id: manifest.epoch_id.clone(),
                manifest_commitment: manifest.commitment.clone(),
                funding_source: manifest.policy.funding_source,
                to: payout.account.clone(),
                asset: ROC_ASSET.into(),
                amount_minor_units: payout.amount_minor_units,
                memo: format!("svc-rewarder:{}:{}", manifest.epoch_id, payout.account),
            });
        }

        intents.sort_by(|a, b| a.to.cmp(&b.to));

        let total = intents.iter().try_fold(AmountMinor::ZERO, |acc, intent| {
            acc.checked_add(intent.amount_minor_units)
        })?;

        if total != manifest.totals.payout_minor_units {
            return Err(RewarderError::Quarantined(
                "settlement intent total does not equal manifest payout total".into(),
            ));
        }

        Ok(Self {
            run_key: manifest.run_key.clone(),
            epoch_id: manifest.epoch_id.clone(),
            manifest_commitment: manifest.commitment.clone(),
            funding_source: manifest.policy.funding_source,
            total_minor_units: total,
            intents,
        })
    }

    /// Convert the settlement plan into wallet issue requests.
    #[must_use]
    pub fn to_wallet_issue_batch(&self) -> WalletIssueBatch {
        WalletIssueBatch {
            run_key: self.run_key.clone(),
            epoch_id: self.epoch_id.clone(),
            manifest_commitment: self.manifest_commitment.clone(),
            funding_source: self.funding_source,
            wallet_path: WALLET_ISSUE_PATH.into(),
            total_minor_units: self.total_minor_units.get().to_string(),
            requests: self
                .intents
                .iter()
                .map(SettlementIntent::to_wallet_issue_request)
                .collect(),
        }
    }
}

/// Public preview DTO for wallet issue egress.
///
/// This intentionally mirrors `svc-wallet`'s `/v1/issue` request shape:
/// `{ to, asset, amount_minor, idempotency_key, memo }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletIssueRequest {
    /// Recipient account.
    pub to: String,
    /// Asset code.
    pub asset: String,
    /// String-encoded minor units, matching wallet DTO rules.
    pub amount_minor: String,
    /// Wallet idempotency key.
    pub idempotency_key: Option<String>,
    /// Deterministic memo.
    pub memo: Option<String>,
}

/// Batch preview of wallet issue requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletIssueBatch {
    /// Rewarder run key.
    pub run_key: String,
    /// Epoch id.
    pub epoch_id: String,
    /// Manifest commitment.
    pub manifest_commitment: String,
    /// Declared funding provenance for the batch. Not forwarded inside wallet issue requests.
    pub funding_source: RewardFundingSource,
    /// Wallet path these requests target.
    pub wallet_path: String,
    /// Total issue amount as string-encoded minor units.
    pub total_minor_units: String,
    /// Wallet issue requests.
    pub requests: Vec<WalletIssueRequest>,
}

/// Convenience facade for intent planning.
pub fn plan_settlement_intents(manifest: &RewardManifest) -> Result<SettlementBatch> {
    SettlementBatch::from_manifest(manifest)
}

/// Ledger/wallet egress result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentResult {
    /// Intent was accepted for the first time.
    Accepted,
    /// Intent was a duplicate of prior run_key.
    Dup,
    /// No intent emitted because request is dry-run.
    DryRun,
    /// Intent failed.
    Error,
}

impl IntentResult {
    /// Label for metrics and manifests.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Dup => "dup",
            Self::DryRun => "dry_run",
            Self::Error => "error",
        }
    }
}

/// Batch-2/3 in-memory idempotency seam for settlement egress.
#[derive(Debug, Default)]
pub struct IntentStore {
    seen: Mutex<HashSet<String>>,
}

impl IntentStore {
    /// Emit a run key once; duplicate returns `Dup`.
    pub fn emit_once(&self, run_key: &str, dry_run: bool) -> IntentResult {
        if dry_run {
            return IntentResult::DryRun;
        }
        let mut seen = self.seen.lock();
        if seen.insert(run_key.to_string()) {
            IntentResult::Accepted
        } else {
            IntentResult::Dup
        }
    }

    /// Emit a settlement batch once; duplicate returns `Dup`.
    pub fn emit_batch_once(&self, batch: &SettlementBatch, dry_run: bool) -> IntentResult {
        self.emit_once(&batch.run_key, dry_run)
    }
}

fn intent_idempotency_key(
    run_key: &str,
    manifest_commitment: &str,
    account: &str,
    amount: AmountMinor,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"svc-rewarder|settlement-intent|v1");
    hasher.update(b"|");
    hasher.update(run_key.as_bytes());
    hasher.update(b"|");
    hasher.update(manifest_commitment.as_bytes());
    hasher.update(b"|");
    hasher.update(account.as_bytes());
    hasher.update(b"|");
    hasher.update(amount.get().to_string().as_bytes());

    // svc-wallet currently validates Idempotency-Key as <=64 bytes.
    // Use a b3-tagged truncated digest: "b3:" + 60 lowercase hex chars = 63 bytes.
    let hex = hasher.finalize().to_hex().to_string();
    format!("b3:{}", &hex[..60])
}
