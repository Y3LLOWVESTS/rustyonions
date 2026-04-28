//! RO:WHAT — Economic invariant checks for reward manifests.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV. Invariant failures quarantine before any intent leaves the service.
//! RO:INTERACTS — core::compute, outputs::manifest, http error mapping.
//! RO:INVARIANTS — Σ payouts ≤ pool; residual = pool - payouts; no zero/negative payout entries.
//! RO:METRICS — quarantine paths are counted by handlers as rejected_total{reason="invariant"}.
//! RO:CONFIG — reward policy min/max influences payout set.
//! RO:SECURITY — prevents inflation/double-issue caused by malformed inputs.
//! RO:TEST — tests/unit/invariants.rs.

use serde::{Deserialize, Serialize};

use crate::core::algebra::AmountMinor;
use crate::outputs::manifest::RewardPayout;
use crate::{Result, RewarderError};

/// Boolean invariant report embedded in manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvariantReport {
    /// Payouts do not exceed pool.
    pub conservation: bool,
    /// No arithmetic overflow was observed.
    pub overflow: bool,
    /// Same inputs produce same run key and manifest commitment.
    pub idempotent: bool,
}

impl InvariantReport {
    /// All green report.
    #[must_use]
    pub fn ok() -> Self {
        Self {
            conservation: true,
            overflow: false,
            idempotent: true,
        }
    }
}

/// Validate payout conservation and return residual.
pub fn validate_payouts(pool: AmountMinor, payouts: &[RewardPayout]) -> Result<AmountMinor> {
    let mut sum = AmountMinor::ZERO;
    for payout in payouts {
        if payout.amount_minor_units.get() == 0 {
            return Err(RewarderError::Quarantined(
                "zero payout entry escaped dust filter".into(),
            ));
        }
        if payout.account.trim().is_empty() {
            return Err(RewarderError::Quarantined(
                "empty payout account escaped validation".into(),
            ));
        }
        sum = sum.checked_add(payout.amount_minor_units)?;
    }
    pool.checked_sub(sum)
}
