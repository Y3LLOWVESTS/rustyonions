//! RO:WHAT — Stable reward-snapshot export DTO consumed by svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/RES. Turns sealed usage signals into deterministic reward inputs.
//! RO:INTERACTS — svc-rewarder AccountingSnapshot contract, utils::{encode,hashing}, accounting windows.
//! RO:INVARIANTS — integer-only counters; canonical account order; no duplicate accounts; b3 CID over canonical bytes.
//! RO:METRICS — callers increment accounting_snapshot_exports_total and accounting_snapshot_bytes.
//! RO:CONFIG — no runtime config; canonical byte cap is enforced by utils::encode.
//! RO:SECURITY — account IDs are bounded and charset-limited; no external-chain/ROX behavior.
//! RO:TEST — unit: reward_snapshot_tests; future interop vector shared with svc-rewarder.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    errors::{Error, Result},
    utils::{encode::to_canonical_bytes, hashing::b3_hex},
};

/// Stable DTO shape for rewarder-compatible accounting snapshots.
///
/// This intentionally mirrors the shape expected by `svc-rewarder`:
///
/// ```text
/// {
///   produced_at_millis,
///   pool_minor_units,
///   contributions: [account, bytes_stored, bytes_served, uptime_seconds]
/// }
/// ```
///
/// `pool_minor_units` is a string at the wire boundary to avoid any accidental
/// JSON-number precision drift in non-Rust consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardSnapshotExport {
    /// Unix timestamp in milliseconds when this export was produced.
    pub produced_at_millis: u64,

    /// Reward pool in ROC minor units, string-encoded at the JSON boundary.
    pub pool_minor_units: String,

    /// Per-account contribution counters.
    pub contributions: Vec<RewardContributionExport>,
}

impl RewardSnapshotExport {
    /// Construct, canonicalize, and validate a reward snapshot.
    pub fn new(
        produced_at_millis: u64,
        pool_minor_units: impl Into<String>,
        contributions: Vec<RewardContributionExport>,
    ) -> Result<Self> {
        let snapshot = Self {
            produced_at_millis,
            pool_minor_units: pool_minor_units.into(),
            contributions,
        };

        snapshot.canonicalized()
    }

    /// Return a canonicalized copy with trimmed accounts and sorted contributions.
    pub fn canonicalized(&self) -> Result<Self> {
        let mut canonical = Self {
            produced_at_millis: self.produced_at_millis,
            pool_minor_units: self.pool_minor_units.trim().to_string(),
            contributions: self
                .contributions
                .iter()
                .map(RewardContributionExport::canonicalized)
                .collect(),
        };

        canonical
            .contributions
            .sort_by(|left, right| left.account.cmp(&right.account));

        canonical.validate()?;
        Ok(canonical)
    }

    /// Validate snapshot schema and arithmetic invariants.
    pub fn validate(&self) -> Result<()> {
        parse_pool_minor_units(&self.pool_minor_units)?;

        if self.contributions.is_empty() {
            return Err(Error::schema(
                "reward snapshot must contain at least one contribution",
            ));
        }

        let mut seen = BTreeSet::new();
        for contribution in &self.contributions {
            contribution.validate()?;

            if !seen.insert(contribution.account.as_str()) {
                return Err(Error::schema(format!(
                    "duplicate reward account: {}",
                    contribution.account
                )));
            }

            contribution.reward_score()?;
        }

        Ok(())
    }

    /// Parse `pool_minor_units` into a checked integer.
    pub fn pool_minor_units_as_u128(&self) -> Result<u128> {
        parse_pool_minor_units(&self.pool_minor_units)
    }

    /// Return deterministic compact JSON bytes for this canonical snapshot.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let canonical = self.canonicalized()?;
        to_canonical_bytes(&canonical)
    }

    /// Return the canonical `b3:<hex>` CID over `canonical_bytes()`.
    pub fn canonical_cid(&self) -> Result<String> {
        Ok(b3_hex(&self.canonical_bytes()?))
    }

    /// Return the checked total reward score across all contributions.
    pub fn total_score(&self) -> Result<u64> {
        self.contributions.iter().try_fold(0_u64, |acc, item| {
            acc.checked_add(item.reward_score()?)
                .ok_or_else(|| Error::schema("reward snapshot total score overflow"))
        })
    }

    /// Number of contribution rows.
    pub fn contribution_count(&self) -> usize {
        self.contributions.len()
    }
}

/// Per-account contribution counters exported to `svc-rewarder`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardContributionExport {
    /// Canonical account string, for example `acct_a` or `t:1/u:alice/w`.
    pub account: String,

    /// Stored-byte contribution counter.
    pub bytes_stored: u64,

    /// Served-byte contribution counter.
    pub bytes_served: u64,

    /// Uptime contribution counter in seconds.
    pub uptime_seconds: u64,
}

impl RewardContributionExport {
    /// Construct a contribution row.
    pub fn new(
        account: impl Into<String>,
        bytes_stored: u64,
        bytes_served: u64,
        uptime_seconds: u64,
    ) -> Self {
        Self {
            account: account.into(),
            bytes_stored,
            bytes_served,
            uptime_seconds,
        }
    }

    /// Return a canonicalized contribution with a trimmed account string.
    pub fn canonicalized(&self) -> Self {
        Self {
            account: self.account.trim().to_string(),
            bytes_stored: self.bytes_stored,
            bytes_served: self.bytes_served,
            uptime_seconds: self.uptime_seconds,
        }
    }

    /// Validate a contribution row.
    pub fn validate(&self) -> Result<()> {
        validate_account(&self.account)?;
        Ok(())
    }

    /// Compute the same simple score shape used by the current rewarder policy baseline:
    ///
    /// ```text
    /// score = bytes_stored + bytes_served / 4 + uptime_seconds
    /// ```
    ///
    /// This is not a payout decision; it is only a checked arithmetic helper for
    /// validating that the snapshot can be consumed safely.
    pub fn reward_score(&self) -> Result<u64> {
        self.bytes_stored
            .checked_add(self.bytes_served / 4)
            .and_then(|value| value.checked_add(self.uptime_seconds))
            .ok_or_else(|| Error::schema("reward contribution score overflow"))
    }
}

/// Compute the canonical CID for a reward snapshot.
pub fn canonical_snapshot_cid(snapshot: &RewardSnapshotExport) -> Result<String> {
    snapshot.canonical_cid()
}

/// Compute canonical bytes for a reward snapshot.
pub fn canonical_snapshot_bytes(snapshot: &RewardSnapshotExport) -> Result<Vec<u8>> {
    snapshot.canonical_bytes()
}

fn parse_pool_minor_units(value: &str) -> Result<u128> {
    let value = value.trim();

    if value.is_empty() {
        return Err(Error::schema("pool_minor_units must not be empty"));
    }
    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(Error::schema(
            "pool_minor_units must be a base-10 unsigned integer string",
        ));
    }

    value
        .parse::<u128>()
        .map_err(|err| Error::schema(format!("invalid pool_minor_units: {err}")))
}

fn validate_account(value: &str) -> Result<()> {
    let value = value.trim();

    if value.is_empty() {
        return Err(Error::schema("reward account must not be empty"));
    }
    if value.len() > 160 {
        return Err(Error::schema("reward account exceeds 160 bytes"));
    }
    if value != value.trim() {
        return Err(Error::schema("reward account must be pre-trimmed"));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/'))
    {
        return Err(Error::schema(
            "reward account contains an unsupported character",
        ));
    }

    Ok(())
}
