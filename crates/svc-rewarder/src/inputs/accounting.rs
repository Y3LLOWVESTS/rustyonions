//! RO:WHAT — Deterministic accounting snapshot input DTOs and validation helpers for reward computation.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/GOV. Rewarder consumes sealed counters without becoming accounting truth.
//! RO:INTERACTS — core::compute, http DTOs, outputs::manifest, future ron-accounting adapter.
//! RO:INVARIANTS — DTO hygiene; canonical account order; unsigned counters; checked score arithmetic; CID binds snapshot.
//! RO:METRICS — invalid snapshots counted by caller as bad_request or invariant rejects.
//! RO:CONFIG — future adapters use ingress.accounting_base_url and cache TTL.
//! RO:SECURITY — snapshots contain aggregate counters only; no auth tokens or raw private payloads.
//! RO:TEST — tests/unit/accounting_policy.rs, tests/unit/accounting_interop.rs, and integration/http_compute.rs.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::core::algebra::AmountMinor;
use crate::inputs::ContentCid;
use crate::{Result, RewarderError};

/// Sealed accounting window used by one reward epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingSnapshot {
    /// Snapshot production timestamp in unix milliseconds.
    pub produced_at_millis: u64,
    /// Total reward pool available for this epoch.
    pub pool_minor_units: AmountMinor,
    /// Per-account contribution counters.
    pub contributions: Vec<AccountContribution>,
}

impl AccountingSnapshot {
    /// Normalize accounts and sort contributions into canonical order.
    pub fn canonicalize(&mut self) {
        for contribution in &mut self.contributions {
            contribution.account = contribution.account.trim().to_string();
        }
        self.contributions.sort_by(|a, b| a.account.cmp(&b.account));
    }

    /// Validate accounting snapshot shape before compute.
    pub fn validate(&self) -> Result<()> {
        let mut seen = HashSet::<String>::new();
        let mut total_score = 0_u128;

        for contribution in &self.contributions {
            if contribution.account.trim().is_empty() {
                return Err(RewarderError::BadRequest(
                    "contribution account cannot be empty".into(),
                ));
            }

            if !seen.insert(contribution.account.clone()) {
                return Err(RewarderError::BadRequest(format!(
                    "duplicate contribution account: {}",
                    contribution.account
                )));
            }

            let score = contribution
                .score()
                .ok_or_else(|| RewarderError::Quarantined("contribution score overflow".into()))?;
            total_score = total_score
                .checked_add(score)
                .ok_or_else(|| RewarderError::Quarantined("total score overflow".into()))?;
        }

        Ok(())
    }
}

/// Per-recipient contribution counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountContribution {
    /// Reward destination account.
    pub account: String,
    /// Bytes stored during the epoch.
    pub bytes_stored: u64,
    /// Bytes served during the epoch.
    pub bytes_served: u64,
    /// Uptime seconds credited during the epoch.
    pub uptime_seconds: u64,
}

impl AccountContribution {
    /// Deterministic default score for batch 2.
    ///
    /// Current formula:
    /// - stored bytes count at 1x
    /// - served bytes count at 0.25x using integer division
    /// - uptime seconds count at 1x
    pub fn score(&self) -> Option<u128> {
        let stored = u128::from(self.bytes_stored);
        let served = u128::from(self.bytes_served / 4);
        let uptime = u128::from(self.uptime_seconds);
        stored.checked_add(served)?.checked_add(uptime)
    }

    /// True when this contribution has at least one credited activity counter.
    #[must_use]
    pub fn has_activity(&self) -> bool {
        self.bytes_stored > 0 || self.bytes_served > 0 || self.uptime_seconds > 0
    }
}

/// Resolve and verify the accounting snapshot for a reward run.
///
/// This still uses inline snapshots for deterministic local development, but
/// the snapshot is now cryptographically bound to `inputs_cid`. This is the
/// same integrity rule the future `ron-accounting` fetch adapter must preserve:
///
/// ```text
/// inputs_cid == canonical_snapshot_cid(snapshot)
/// ```
pub fn resolve_accounting_snapshot(
    inputs_cid: &ContentCid,
    inline: Option<AccountingSnapshot>,
) -> Result<AccountingSnapshot> {
    let mut snapshot = inline.ok_or_else(|| {
        RewarderError::BadRequest(
            "inline snapshot is required until the ron-accounting adapter is wired".into(),
        )
    })?;

    snapshot.canonicalize();
    snapshot.validate()?;

    let computed_cid = canonical_snapshot_cid(snapshot.clone())?;
    if computed_cid != inputs_cid.as_str() {
        return Err(RewarderError::BadRequest(format!(
            "inputs_cid mismatch: expected {computed_cid}, got {}",
            inputs_cid.as_str()
        )));
    }

    Ok(snapshot)
}

/// Compute the canonical CID for an inline accounting snapshot.
///
/// The CID is the BLAKE3 digest over the canonical JSON representation after
/// account trimming, duplicate detection, and deterministic contribution sorting.
pub fn canonical_snapshot_cid(mut snapshot: AccountingSnapshot) -> Result<String> {
    snapshot.canonicalize();
    snapshot.validate()?;
    let bytes =
        serde_json::to_vec(&snapshot).map_err(|err| RewarderError::Internal(err.to_string()))?;
    Ok(format!("b3:{}", blake3::hash(&bytes).to_hex()))
}
