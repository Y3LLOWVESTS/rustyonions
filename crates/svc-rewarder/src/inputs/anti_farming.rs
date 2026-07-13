//! RO:WHAT — Anti-farming and event-class gates for Internal ROC rewarder inputs.
//! RO:WHY — ECON/SEC: rewarder must consume only verified/capped planning inputs and must never turn raw engagement into protocol ROC.
//! RO:INTERACTS — accounting snapshots, reward policy funding source, `AccountContribution`.
//! RO:INVARIANTS — no wallet mutation; no ledger mutation; no direct protocol ROC allocation from analytics or metering.
//! RO:METRICS — callers may count rejected/capped candidates.
//! RO:CONFIG — caps come from validated economics/policy config in later wiring.
//! RO:SECURITY — ad-budgeted material requires explicit non-protocol budget.
//! RO:TEST — `tests/internal_roc_beta_phase5_antifarming_event_gates.rs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    inputs::{
        economics::InternalRocRewardPlanningEconomics, AccountContribution, RewardFundingSource,
    },
    Result, RewarderError,
};

/// Rewarder-side copy of the Internal ROC event-class lane supplied by validated accounting/policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardInputEventClass {
    /// Usage/metering facts only; never direct payout input.
    Metering,
    /// Candidate material after accounting/policy marks it for verification/caps.
    ProofEligible,
    /// Explicit payer-authorized budget lane.
    AdBudgeted,
    /// Product analytics only; never reward input.
    AnalyticsOnly,
}

/// Anti-farming caps applied before constructing rewarder `AccountContribution` inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AntiFarmingCapPolicy {
    /// Max stored bytes contribution per account/content/epoch candidate.
    pub max_bytes_stored: u64,
    /// Max served bytes contribution per account/content/epoch candidate.
    pub max_bytes_served: u64,
    /// Max uptime seconds contribution per account/content/epoch candidate.
    pub max_uptime_seconds: u64,
    /// Max deterministic score after all caps.
    pub max_score_per_account: u128,
}

impl AntiFarmingCapPolicy {
    /// Validate cap values.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError::BadRequest` when any cap is zero.
    pub fn validate(&self) -> Result<()> {
        if self.max_bytes_stored == 0 {
            return Err(RewarderError::BadRequest(
                "anti-farming max_bytes_stored must be > 0".into(),
            ));
        }
        if self.max_bytes_served == 0 {
            return Err(RewarderError::BadRequest(
                "anti-farming max_bytes_served must be > 0".into(),
            ));
        }
        if self.max_uptime_seconds == 0 {
            return Err(RewarderError::BadRequest(
                "anti-farming max_uptime_seconds must be > 0".into(),
            ));
        }
        if self.max_score_per_account == 0 {
            return Err(RewarderError::BadRequest(
                "anti-farming max_score_per_account must be > 0".into(),
            ));
        }
        Ok(())
    }
}

/// One pre-snapshot candidate after accounting/policy classification but before rewarder caps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CappedRewardInputCandidate {
    /// Recipient account.
    pub account: String,
    /// Event class lane supplied by validated upstream gates.
    pub event_class: RewardInputEventClass,
    /// Verification/caps/policy gate result.
    pub verified: bool,
    /// Explicit budget authorization for ad-budgeted material.
    pub explicit_budget_authorized: bool,
    /// Whether ron-policy has declaratively approved this candidate for reward planning.
    pub policy_gate_passed: bool,
    /// Stored bytes candidate amount.
    pub bytes_stored: u64,
    /// Served bytes candidate amount.
    pub bytes_served: u64,
    /// Uptime seconds candidate amount.
    pub uptime_seconds: u64,
}

impl CappedRewardInputCandidate {
    /// Convert a candidate into a capped `AccountContribution` if it is eligible.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError::BadRequest` when a raw, analytics-only, unverified,
    /// or unfunded ad-budgeted candidate attempts to enter reward planning.
    pub fn into_capped_contribution(
        self,
        caps: &AntiFarmingCapPolicy,
        funding_source: RewardFundingSource,
    ) -> Result<AccountContribution> {
        caps.validate()?;

        let account = self.account.trim().to_owned();
        if account.is_empty() {
            return Err(RewarderError::BadRequest(
                "reward input account cannot be empty".into(),
            ));
        }

        if !self.policy_gate_passed {
            return Err(RewarderError::BadRequest(
                "reward input requires ron-policy gate before planning".into(),
            ));
        }

        match self.event_class {
            RewardInputEventClass::AnalyticsOnly => {
                return Err(RewarderError::BadRequest(
                    "analytics_only material is not reward-planning input".into(),
                ));
            }
            RewardInputEventClass::Metering => {
                return Err(RewarderError::BadRequest(
                    "metering material is not direct reward-planning input".into(),
                ));
            }
            RewardInputEventClass::ProofEligible => {
                if !self.verified {
                    return Err(RewarderError::BadRequest(
                        "proof_eligible reward input requires verification/caps/policy".into(),
                    ));
                }
            }
            RewardInputEventClass::AdBudgeted => {
                if !self.verified || !self.explicit_budget_authorized {
                    return Err(RewarderError::BadRequest(
                        "ad_budgeted reward input requires verification and explicit budget".into(),
                    ));
                }
                if funding_source == RewardFundingSource::ProtocolPool {
                    return Err(RewarderError::BadRequest(
                        "ad_budgeted reward input must not use protocol-pool emission".into(),
                    ));
                }
            }
        }

        let mut contribution = AccountContribution {
            account,
            bytes_stored: self.bytes_stored.min(caps.max_bytes_stored),
            bytes_served: self.bytes_served.min(caps.max_bytes_served),
            uptime_seconds: self.uptime_seconds.min(caps.max_uptime_seconds),
        };

        while contribution
            .score()
            .ok_or_else(|| RewarderError::Quarantined("capped score overflow".into()))?
            > caps.max_score_per_account
        {
            contribution = reduce_largest_counter(contribution);
        }

        Ok(contribution)
    }
}

/// Build capped contributions from candidate inputs.
///
/// This compatibility path applies the existing explicit counter and
/// score caps without imposing a separate event-count ceiling.
///
/// # Errors
///
/// Returns `RewarderError` if any candidate attempts to bypass class,
/// verification, policy, or budget gates.
pub fn capped_contributions_from_candidates(
    candidates: Vec<CappedRewardInputCandidate>,
    caps: &AntiFarmingCapPolicy,
    funding_source: RewardFundingSource,
) -> Result<Vec<AccountContribution>> {
    capped_contributions_from_candidates_with_event_limit(
        candidates,
        caps,
        funding_source,
        u64::MAX,
    )
}

/// Build capped contributions using the event-count ceiling from one
/// validated economics projection.
///
/// Each candidate counts as one account event. Multiple eligible
/// events for the same canonical account are deterministically
/// aggregated into one `AccountContribution`.
///
/// # Errors
///
/// Returns `RewarderError` when economics validation fails, an
/// account exceeds its configured epoch event limit, arithmetic
/// overflows, or a candidate bypasses an eligibility gate.
pub fn capped_contributions_from_candidates_with_economics(
    candidates: Vec<CappedRewardInputCandidate>,
    caps: &AntiFarmingCapPolicy,
    funding_source: RewardFundingSource,
    economics: &InternalRocRewardPlanningEconomics,
) -> Result<Vec<AccountContribution>> {
    economics.validate_binding()?;

    capped_contributions_from_candidates_with_event_limit(
        candidates,
        caps,
        funding_source,
        economics.max_events_per_account_per_epoch,
    )
}

fn capped_contributions_from_candidates_with_event_limit(
    candidates: Vec<CappedRewardInputCandidate>,
    caps: &AntiFarmingCapPolicy,
    funding_source: RewardFundingSource,
    max_events_per_account_per_epoch: u64,
) -> Result<Vec<AccountContribution>> {
    caps.validate()?;

    if max_events_per_account_per_epoch == 0 {
        return Err(RewarderError::BadRequest(
            "anti-farming max events per account must be > 0".into(),
        ));
    }

    let mut event_counts = BTreeMap::<String, u64>::new();
    let mut aggregated = BTreeMap::<String, AccountContribution>::new();

    for candidate in candidates {
        let contribution = candidate.into_capped_contribution(caps, funding_source)?;

        let account = contribution.account.clone();

        let event_count = event_counts.entry(account.clone()).or_insert(0);

        *event_count = event_count
            .checked_add(1)
            .ok_or_else(|| RewarderError::Quarantined("account event count overflow".into()))?;

        if *event_count > max_events_per_account_per_epoch {
            return Err(RewarderError::BadRequest(format!(
                "reward input account {account} exceeds economics max events per account per epoch: {max_events_per_account_per_epoch}"
            )));
        }

        let aggregate = aggregated
            .entry(account.clone())
            .or_insert_with(|| AccountContribution {
                account,
                bytes_stored: 0,
                bytes_served: 0,
                uptime_seconds: 0,
            });

        aggregate.bytes_stored = checked_accumulate_counter(
            "bytes_stored",
            aggregate.bytes_stored,
            contribution.bytes_stored,
            caps.max_bytes_stored,
        )?;

        aggregate.bytes_served = checked_accumulate_counter(
            "bytes_served",
            aggregate.bytes_served,
            contribution.bytes_served,
            caps.max_bytes_served,
        )?;

        aggregate.uptime_seconds = checked_accumulate_counter(
            "uptime_seconds",
            aggregate.uptime_seconds,
            contribution.uptime_seconds,
            caps.max_uptime_seconds,
        )?;
    }

    let mut contributions = aggregated.into_values().collect::<Vec<_>>();

    for contribution in &mut contributions {
        while contribution
            .score()
            .ok_or_else(|| RewarderError::Quarantined("aggregated capped score overflow".into()))?
            > caps.max_score_per_account
        {
            *contribution = reduce_largest_counter(contribution.clone());
        }
    }

    Ok(contributions)
}

fn checked_accumulate_counter(field: &str, current: u64, additional: u64, cap: u64) -> Result<u64> {
    current
        .checked_add(additional)
        .map(|total| total.min(cap))
        .ok_or_else(|| RewarderError::Quarantined(format!("{field} aggregation overflow")))
}

fn reduce_largest_counter(mut contribution: AccountContribution) -> AccountContribution {
    if contribution.bytes_stored >= contribution.bytes_served
        && contribution.bytes_stored >= contribution.uptime_seconds
    {
        contribution.bytes_stored /= 2;
    } else if contribution.bytes_served >= contribution.uptime_seconds {
        contribution.bytes_served /= 2;
    } else {
        contribution.uptime_seconds /= 2;
    }

    contribution
}
