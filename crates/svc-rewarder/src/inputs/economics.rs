//! RO:WHAT — Internal ROC economics config projection for reward planning.
//!
//! RO:WHY — ECON/GOV: svc-rewarder consumes validated tokenomics config for planning only.
//! RO:INTERACTS — `configs/roc-economics.toml`, `inputs::RewardPolicy`, and `core::compute_manifest`.
//! RO:INVARIANTS — no IO; no floats; no receipts; no ledger mutation; bridge/staking placeholders stay inert.
//! RO:METRICS — callers may count config validation failures.
//! RO:CONFIG — parses caller-provided canonical internal ROC economics TOML.
//! RO:SECURITY — config-derived values are planning caps only, never wallet/ledger authority.
//! RO:TEST — `tests/internal_roc_beta_phase5_config_driven_planning.rs`.

use serde::Deserialize;

use crate::core::AmountMinor;
use crate::inputs::{validate_reward_policy, RewardFundingSource, RewardPolicy};
use crate::{Result, RewarderError};

const INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA: &str = "internal_roc.economics-config.v1";
const INTERNAL_ROC_ECONOMICS_CONFIG_VERSION: u16 = 1;
const MAX_MONEY_DIGITS: usize = 39;

/// Reward planning economics projected from canonical internal ROC config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalRocRewardPlanningEconomics {
    /// Schema that supplied this projection.
    pub schema: String,
    /// Version that supplied this projection.
    pub version: u16,
    /// Absolute epoch reward pool cap.
    pub epoch_pool_cap_minor: AmountMinor,
    /// Max account reward cap.
    pub max_reward_minor_per_account_per_epoch: AmountMinor,
    /// Max content reward cap.
    pub max_reward_minor_per_content_per_epoch: AmountMinor,
    /// Deterministic rounding mode.
    pub rounding_mode: String,
    /// Explicit remainder sink.
    pub remainder_sink: String,
    /// Whether bridge placeholder is inert.
    pub bridge_inert: bool,
    /// Whether staking placeholder is inert.
    pub staking_inert: bool,
}

impl InternalRocRewardPlanningEconomics {
    /// Build a reward policy using config-derived payout caps.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError::BadRequest` if the generated policy fails existing policy validation.
    pub fn to_reward_policy(&self, policy_id: &str, policy_hash: &str) -> Result<RewardPolicy> {
        let policy = RewardPolicy {
            id: policy_id.to_owned(),
            hash: policy_hash.to_owned(),
            signed: true,
            funding_source: RewardFundingSource::ProtocolPool,
            max_payout_minor_units: self.epoch_pool_cap_minor,
            min_payout_minor_units: AmountMinor(1),
            weight_bps: 10_000,
            rounding: self.rounding_mode.clone(),
        };

        validate_reward_policy(&policy, policy_id, policy_hash)?;
        Ok(policy)
    }
}

/// Parse canonical internal ROC economics TOML into reward-planning economics.
///
/// # Errors
///
/// Returns `RewarderError::BadRequest` for invalid UTF-8, TOML, schema, caps, rounding, or enabled future features.
pub fn load_internal_roc_planning_economics_toml(
    bytes: &[u8],
) -> Result<InternalRocRewardPlanningEconomics> {
    let raw = std::str::from_utf8(bytes)
        .map_err(|err| RewarderError::BadRequest(format!("invalid economics UTF-8: {err}")))?;
    load_internal_roc_planning_economics_toml_str(raw)
}

/// Parse canonical internal ROC economics TOML string into reward-planning economics.
///
/// # Errors
///
/// Returns `RewarderError::BadRequest` for invalid TOML, schema, caps, rounding, or enabled future features.
pub fn load_internal_roc_planning_economics_toml_str(
    raw: &str,
) -> Result<InternalRocRewardPlanningEconomics> {
    let parsed = toml::from_str::<InternalRocEconomicsToml>(raw)
        .map_err(|err| RewarderError::BadRequest(format!("invalid economics TOML: {err}")))?;

    parsed.into_planning()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InternalRocEconomicsToml {
    schema: String,
    version: u16,
    units: toml::Value,
    paid_content: toml::Value,
    reward_pools: RewardPoolsToml,
    anti_farming: AntiFarmingToml,
    rounding: RoundingToml,
    future_bridge: FutureFeatureToml,
    future_staking: FutureFeatureToml,
}

impl InternalRocEconomicsToml {
    fn into_planning(self) -> Result<InternalRocRewardPlanningEconomics> {
        if self.schema != INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA {
            return Err(RewarderError::BadRequest(
                "invalid internal ROC economics config schema".into(),
            ));
        }
        if self.version != INTERNAL_ROC_ECONOMICS_CONFIG_VERSION {
            return Err(RewarderError::BadRequest(
                "invalid internal ROC economics config version".into(),
            ));
        }

        if self.reward_pools.category_caps.is_empty() {
            return Err(RewarderError::BadRequest(
                "reward_pools.category_caps must not be empty".into(),
            ));
        }
        if self.anti_farming.max_events_per_account_per_epoch == 0 {
            return Err(RewarderError::BadRequest(
                "anti_farming.max_events_per_account_per_epoch must be > 0".into(),
            ));
        }

        if self.future_bridge.enabled {
            return Err(RewarderError::BadRequest(
                "future_bridge.enabled must remain false/inert".into(),
            ));
        }
        if self.future_bridge.state.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "future_bridge.state must be explicit".into(),
            ));
        }
        if self.future_staking.enabled {
            return Err(RewarderError::BadRequest(
                "future_staking.enabled must remain false/inert".into(),
            ));
        }
        if self.future_staking.state.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "future_staking.state must be explicit".into(),
            ));
        }
        if self.rounding.mode != "floor" {
            return Err(RewarderError::BadRequest(
                "rounding.mode must be floor for beta planning".into(),
            ));
        }
        if self.rounding.remainder_sink.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "rounding.remainder_sink must be explicit".into(),
            ));
        }
        if self.rounding.remainder_sink == "configured_account" {
            let account = self
                .rounding
                .remainder_sink_account
                .as_deref()
                .unwrap_or("")
                .trim();
            if account.is_empty() {
                return Err(RewarderError::BadRequest(
                    "rounding.remainder_sink_account required for configured_account".into(),
                ));
            }
        } else if self.rounding.remainder_sink_account.is_some() {
            return Err(RewarderError::BadRequest(
                "rounding.remainder_sink_account is only allowed for configured_account".into(),
            ));
        }

        let _ = self.units;
        let _ = self.paid_content;

        Ok(InternalRocRewardPlanningEconomics {
            schema: self.schema,
            version: self.version,
            epoch_pool_cap_minor: AmountMinor(parse_positive_money(
                "reward_pools.epoch_pool_cap_minor",
                &self.reward_pools.epoch_pool_cap_minor,
            )?),
            max_reward_minor_per_account_per_epoch: AmountMinor(parse_positive_money(
                "anti_farming.max_reward_minor_per_account_per_epoch",
                &self.anti_farming.max_reward_minor_per_account_per_epoch,
            )?),
            max_reward_minor_per_content_per_epoch: AmountMinor(parse_positive_money(
                "anti_farming.max_reward_minor_per_content_per_epoch",
                &self.anti_farming.max_reward_minor_per_content_per_epoch,
            )?),
            rounding_mode: self.rounding.mode,
            remainder_sink: self.rounding.remainder_sink,
            bridge_inert: !self.future_bridge.enabled,
            staking_inert: !self.future_staking.enabled,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RewardPoolsToml {
    epoch_pool_cap_minor: String,
    category_caps: Vec<toml::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AntiFarmingToml {
    max_events_per_account_per_epoch: u64,
    max_reward_minor_per_account_per_epoch: String,
    max_reward_minor_per_content_per_epoch: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoundingToml {
    mode: String,
    remainder_sink: String,
    #[serde(default)]
    remainder_sink_account: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FutureFeatureToml {
    enabled: bool,
    state: String,
}

fn parse_positive_money(field: &str, value: &str) -> Result<u128> {
    if value.is_empty() {
        return Err(RewarderError::BadRequest(format!(
            "{field} must not be empty"
        )));
    }
    if value.len() > MAX_MONEY_DIGITS {
        return Err(RewarderError::BadRequest(format!(
            "{field} exceeds u128 decimal width"
        )));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(RewarderError::BadRequest(format!(
            "{field} must not have leading zeroes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RewarderError::BadRequest(format!(
            "{field} must be decimal integer minor units"
        )));
    }

    let parsed = value
        .parse::<u128>()
        .map_err(|err| RewarderError::BadRequest(format!("{field} must fit u128: {err}")))?;
    if parsed == 0 {
        return Err(RewarderError::BadRequest(format!("{field} must be > 0")));
    }
    Ok(parsed)
}
