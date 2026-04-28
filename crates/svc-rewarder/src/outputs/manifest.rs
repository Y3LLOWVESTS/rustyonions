//! RO:WHAT — Canonical reward manifest structs and commitment hashing.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/DX. Manifests are the audit object for reward epochs.
//! RO:INTERACTS — core::compute, outputs::intents, http DTOs.
//! RO:INVARIANTS — sorted payouts; string amounts; BLAKE3 `b3:<hex>` commitment over canonical JSON.
//! RO:METRICS — manifest status feeds reward_runs_total.
//! RO:CONFIG — idempotency salt influences run_key upstream.
//! RO:SECURITY — no raw input payloads or secrets embedded.
//! RO:TEST — idempotency and HTTP integration tests.

use serde::{Deserialize, Serialize};

use crate::core::algebra::AmountMinor;
use crate::core::invariants::InvariantReport;
use crate::outputs::attestation::Attestation;
use crate::outputs::intents::IntentResult;
use crate::{Result, RewarderError};

/// Manifest status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestStatus {
    /// Computed and invariant-clean.
    Ok,
    /// Quarantined before egress.
    Quarantined,
    /// Failed unexpectedly.
    Fail,
}

impl ManifestStatus {
    /// Stable label.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Quarantined => "quarantined",
            Self::Fail => "fail",
        }
    }
}

/// Per-recipient payout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardPayout {
    /// Recipient account.
    pub account: String,
    /// Payout amount.
    pub amount_minor_units: AmountMinor,
    /// Raw deterministic score used for allocation.
    pub score: u128,
}

/// Totals included in every response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardTotals {
    /// Reward pool.
    pub pool_minor_units: AmountMinor,
    /// Sum of payouts.
    pub payout_minor_units: AmountMinor,
    /// Unpaid residual/dust.
    pub residual_minor_units: AmountMinor,
}

/// Policy summary embedded in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySummary {
    /// Policy id.
    pub id: String,
    /// Policy hash.
    pub hash: String,
    /// Signature verification posture.
    pub signed: bool,
}

/// Ledger/wallet egress summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerSummary {
    /// True if a non-dry-run egress attempt was made.
    pub emitted: bool,
    /// Egress result.
    pub result: String,
}

/// Sealed reward manifest for an epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardManifest {
    /// Schema version.
    pub version: u16,
    /// Epoch id from the path.
    pub epoch_id: String,
    /// Deterministic run key from `(epoch_id, policy_hash, inputs_cid)`.
    pub run_key: String,
    /// Manifest commitment hash.
    pub commitment: String,
    /// Manifest status.
    pub status: ManifestStatus,
    /// Input CID.
    pub inputs_cid: String,
    /// Totals.
    pub totals: RewardTotals,
    /// Policy summary.
    pub policy: PolicySummary,
    /// Invariant flags.
    pub invariants: InvariantReport,
    /// Ledger/wallet egress summary.
    pub ledger: LedgerSummary,
    /// Per-recipient payouts.
    pub payouts: Vec<RewardPayout>,
    /// Optional attestation.
    pub attestation: Option<Attestation>,
}

impl RewardManifest {
    /// Recompute and set commitment.
    pub fn seal(mut self) -> Result<Self> {
        self.payouts.sort_by(|a, b| a.account.cmp(&b.account));
        self.commitment = commitment_for_manifest(&self)?;
        Ok(self)
    }
}

#[derive(Serialize)]
struct CommitmentView<'a> {
    version: u16,
    epoch_id: &'a str,
    run_key: &'a str,
    status: &'a ManifestStatus,
    inputs_cid: &'a str,
    totals: &'a RewardTotals,
    policy: &'a PolicySummary,
    invariants: &'a InvariantReport,
    ledger: &'a LedgerSummary,
    payouts: &'a [RewardPayout],
    attestation: &'a Option<Attestation>,
}

/// Compute canonical commitment, excluding the commitment field itself.
pub fn commitment_for_manifest(manifest: &RewardManifest) -> Result<String> {
    let view = CommitmentView {
        version: manifest.version,
        epoch_id: &manifest.epoch_id,
        run_key: &manifest.run_key,
        status: &manifest.status,
        inputs_cid: &manifest.inputs_cid,
        totals: &manifest.totals,
        policy: &manifest.policy,
        invariants: &manifest.invariants,
        ledger: &manifest.ledger,
        payouts: &manifest.payouts,
        attestation: &manifest.attestation,
    };
    let bytes = serde_json::to_vec(&view).map_err(|e| RewarderError::Internal(e.to_string()))?;
    Ok(format!("b3:{}", blake3::hash(&bytes).to_hex()))
}

impl From<&IntentResult> for LedgerSummary {
    fn from(value: &IntentResult) -> Self {
        Self {
            emitted: matches!(
                value,
                IntentResult::Accepted | IntentResult::Dup | IntentResult::Error
            ),
            result: value.as_str().into(),
        }
    }
}
