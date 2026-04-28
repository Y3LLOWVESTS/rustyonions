//! RO:WHAT — Pure deterministic reward calculation pipeline.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/GOV. Same sealed inputs must always produce same payout set.
//! RO:INTERACTS — inputs::AccountingSnapshot, inputs::RewardPolicy, outputs::manifest.
//! RO:INVARIANTS — no IO; no floats; checked arithmetic; canonical account order.
//! RO:METRICS — compute latency measured by handlers.
//! RO:CONFIG — idempotency salt is used by run_key helper.
//! RO:SECURITY — rejects malformed policy hashes and empty account destinations.
//! RO:TEST — tests/unit/invariants.rs and HTTP idempotency tests.

use crate::core::algebra::{checked_mul_div_floor, AmountMinor};
use crate::core::invariants::{validate_payouts, InvariantReport};
use crate::inputs::{AccountingSnapshot, ContentCid, RewardPolicy};
use crate::outputs::intents::IntentResult;
use crate::outputs::manifest::{
    LedgerSummary, ManifestStatus, PolicySummary, RewardManifest, RewardPayout, RewardTotals,
};
use crate::{Result, RewarderError};

/// Compute input bundle after DTO validation.
#[derive(Debug, Clone)]
pub struct ComputeInput {
    /// Epoch id.
    pub epoch_id: String,
    /// Input CID.
    pub inputs_cid: ContentCid,
    /// Policy.
    pub policy: RewardPolicy,
    /// Accounting snapshot.
    pub snapshot: AccountingSnapshot,
    /// Whether to skip settlement egress.
    pub dry_run: bool,
    /// Domain separator.
    pub idempotency_salt: String,
}

/// Build the deterministic run key for an epoch triple.
pub fn run_key(epoch_id: &str, policy_hash: &str, inputs_cid: &str, salt: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(salt.as_bytes());
    hasher.update(b"|");
    hasher.update(epoch_id.as_bytes());
    hasher.update(b"|");
    hasher.update(policy_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(inputs_cid.as_bytes());
    format!("b3:{}", hasher.finalize().to_hex())
}

/// Compute a reward manifest using a supplied egress outcome.
pub fn compute_manifest(input: ComputeInput, egress: IntentResult) -> Result<RewardManifest> {
    validate_policy(&input.policy)?;
    if input.policy.id.trim().is_empty() {
        return Err(RewarderError::BadRequest(
            "policy_id cannot be empty".into(),
        ));
    }
    if input.policy.hash != input.policy.hash.to_ascii_lowercase() {
        return Err(RewarderError::BadRequest(
            "policy_hash must be lowercase canonical b3 hex".into(),
        ));
    }

    let mut snapshot = input.snapshot;
    snapshot.canonicalize();
    let pool = if snapshot.pool_minor_units <= input.policy.max_payout_minor_units {
        snapshot.pool_minor_units
    } else {
        input.policy.max_payout_minor_units
    };

    let mut scored = Vec::<(String, u128)>::new();
    for contribution in &snapshot.contributions {
        let account = contribution.account.trim();
        if account.is_empty() {
            return Err(RewarderError::BadRequest(
                "contribution account cannot be empty".into(),
            ));
        }
        let raw_score = contribution
            .score()
            .ok_or_else(|| RewarderError::Quarantined("contribution score overflow".into()))?;
        let weighted = raw_score
            .checked_mul(u128::from(input.policy.weight_bps))
            .ok_or_else(|| RewarderError::Quarantined("weighted score overflow".into()))?;
        if weighted > 0 {
            scored.push((account.to_string(), weighted));
        }
    }

    let total_score = scored.iter().try_fold(0_u128, |acc, (_, score)| {
        acc.checked_add(*score)
            .ok_or_else(|| RewarderError::Quarantined("total score overflow".into()))
    })?;

    let mut payouts = Vec::new();
    if total_score > 0 && pool.get() > 0 {
        for (account, score) in scored {
            let amount = checked_mul_div_floor(pool.get(), score, total_score)?;
            let amount = AmountMinor(amount);
            if amount >= input.policy.min_payout_minor_units && amount.get() > 0 {
                payouts.push(RewardPayout {
                    account,
                    amount_minor_units: amount,
                    score,
                });
            }
        }
    }
    payouts.sort_by(|a, b| a.account.cmp(&b.account));

    let residual = validate_payouts(pool, &payouts)?;
    let payout_total = pool.checked_sub(residual)?;
    let key = run_key(
        &input.epoch_id,
        &input.policy.hash,
        input.inputs_cid.as_str(),
        &input.idempotency_salt,
    );

    RewardManifest {
        version: 1,
        epoch_id: input.epoch_id,
        run_key: key,
        commitment: String::new(),
        status: ManifestStatus::Ok,
        inputs_cid: input.inputs_cid.to_string(),
        totals: RewardTotals {
            pool_minor_units: pool,
            payout_minor_units: payout_total,
            residual_minor_units: residual,
        },
        policy: PolicySummary {
            id: input.policy.id,
            hash: input.policy.hash,
            signed: input.policy.signed,
        },
        invariants: InvariantReport::ok(),
        ledger: LedgerSummary::from(&egress),
        payouts,
        attestation: None,
    }
    .seal()
}

fn validate_policy(policy: &RewardPolicy) -> Result<()> {
    if !policy.hash.starts_with("b3:") || policy.hash.len() != 67 {
        return Err(RewarderError::BadRequest(
            "policy_hash must be b3:<64 lowercase hex chars>".into(),
        ));
    }
    let hex = &policy.hash[3..];
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RewarderError::BadRequest(
            "policy_hash must contain only hex chars".into(),
        ));
    }
    if policy.weight_bps == 0 {
        return Err(RewarderError::BadRequest(
            "policy weight_bps must be > 0".into(),
        ));
    }
    if policy.rounding != "floor" {
        return Err(RewarderError::BadRequest(
            "batch 1 supports only rounding=floor".into(),
        ));
    }
    Ok(())
}
