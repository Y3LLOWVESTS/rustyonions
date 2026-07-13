//! RO:WHAT — Internal ROC economics projection for deterministic reward planning.
//!
//! RO:WHY — ECON/GOV: svc-rewarder must consume ron-policy's validated economics model rather than duplicate its schema or acceptance rules.
//! RO:INTERACTS — ron-policy economics validation/hashing, `inputs::RewardPolicy`, and `core::compute_manifest`.
//! RO:INVARIANTS — one shared schema; no overlay/fallback; integer-only caps; no receipts or ledger mutation; bridge/staking remain inert.
//! RO:METRICS — callers may count config validation failures.
//! RO:CONFIG — parses one complete caller-provided canonical or development economics document.
//! RO:SECURITY — config-derived values and identity are planning inputs only, never wallet or ledger authority.
//! RO:TEST — Phase 14D economics and manifest-binding integration tests.

use ron_policy::economics::{
    internal_roc_economics_config_hash, load_internal_roc_economics_toml,
    InternalRocEconomicsConfig, InternalRocRemainderSink, InternalRocRoundingMode,
    INTERNAL_ROC_BPS_DENOMINATOR,
};

use crate::core::AmountMinor;
use crate::inputs::{
    policy_hash_is_canonical, validate_reward_policy, RewardFundingSource, RewardPolicy,
};
use crate::{Result, RewarderError};

const CANONICAL_ROC_ECONOMICS_TOML: &[u8] =
    include_bytes!("../../../../configs/roc-economics.toml");

/// One reward-category cap projected from validated economics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalRocRewardCategoryPlanningCap {
    /// Stable category label supplied by ron-policy.
    pub category: String,
    /// Category share of the available epoch pool.
    pub pool_bps: u16,
    /// Absolute category ceiling in ROC minor units.
    pub category_cap_minor: AmountMinor,
}

/// Reward-planning values projected from a validated Internal ROC
/// economics document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalRocRewardPlanningEconomics {
    /// Schema that supplied this projection.
    pub schema: String,
    /// Version that supplied this projection.
    pub version: u16,
    /// Explicit standalone economics profile.
    pub profile: String,
    /// Canonical BLAKE3 identity of the normalized economics model.
    pub economics_config_hash: String,
    /// Absolute epoch reward-pool cap.
    pub epoch_pool_cap_minor: AmountMinor,
    /// Category caps sorted by stable category label.
    pub category_caps: Vec<InternalRocRewardCategoryPlanningCap>,
    /// Maximum accepted events per account per epoch.
    pub max_events_per_account_per_epoch: u64,
    /// Maximum account reward per epoch.
    pub max_reward_minor_per_account_per_epoch: AmountMinor,
    /// Maximum content reward per epoch.
    pub max_reward_minor_per_content_per_epoch: AmountMinor,
    /// Deterministic rounding mode.
    pub rounding_mode: String,
    /// Explicit remainder sink label.
    pub remainder_sink: String,
    /// Whether the bridge placeholder remains inert.
    pub bridge_inert: bool,
    /// Whether the staking placeholder remains inert.
    pub staking_inert: bool,
}

impl InternalRocRewardPlanningEconomics {
    /// Find one category cap by exact validated label.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError::BadRequest` for an empty or unknown
    /// category or an invalid economics projection.
    pub fn category_cap(&self, category: &str) -> Result<&InternalRocRewardCategoryPlanningCap> {
        self.validate_binding()?;

        let category = category.trim();
        if category.is_empty() {
            return Err(RewarderError::BadRequest(
                "reward category must not be empty".into(),
            ));
        }

        self.category_caps
            .iter()
            .find(|cap| cap.category == category)
            .ok_or_else(|| {
                RewarderError::BadRequest(format!("unknown reward category: {category}"))
            })
    }

    /// Calculate the effective pool ceiling for one category.
    ///
    /// The result applies the epoch cap, category basis-point share,
    /// and absolute category cap.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError` for an unknown category or checked
    /// arithmetic failure.
    pub fn effective_category_pool_cap(
        &self,
        category: &str,
        available_pool: AmountMinor,
    ) -> Result<AmountMinor> {
        let cap = self.category_cap(category)?;

        let epoch_limited_pool = available_pool.min(self.epoch_pool_cap_minor);

        let bps_limited = crate::core::checked_mul_div_floor(
            epoch_limited_pool.get(),
            u128::from(cap.pool_bps),
            u128::from(INTERNAL_ROC_BPS_DENOMINATOR),
        )?;

        Ok(AmountMinor(bps_limited).min(cap.category_cap_minor))
    }

    /// Validate this projected economics binding before it enters a
    /// reward plan.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError::BadRequest` when required identity or
    /// inert-posture values are missing or malformed.
    pub fn validate_binding(&self) -> Result<()> {
        if self.schema.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "economics config schema must not be empty".into(),
            ));
        }

        if self.version == 0 {
            return Err(RewarderError::BadRequest(
                "economics config version must be > 0".into(),
            ));
        }

        if self.profile.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "economics config profile must not be empty".into(),
            ));
        }

        if !policy_hash_is_canonical(&self.economics_config_hash) {
            return Err(RewarderError::BadRequest(
                "economics_config_hash must be b3:<64 lowercase hex chars>".into(),
            ));
        }

        if self.epoch_pool_cap_minor.get() == 0 {
            return Err(RewarderError::BadRequest(
                "economics epoch pool cap must be > 0".into(),
            ));
        }

        if self.category_caps.is_empty() {
            return Err(RewarderError::BadRequest(
                "economics reward category caps must not be empty".into(),
            ));
        }

        let mut previous_category: Option<&str> = None;
        let mut category_bps_total = 0_u32;

        for cap in &self.category_caps {
            let category = cap.category.as_str();

            if category.trim().is_empty() || category.trim() != category {
                return Err(RewarderError::BadRequest(
                    "economics reward category must be canonical".into(),
                ));
            }

            if previous_category.is_some_and(|previous| previous >= category) {
                return Err(RewarderError::BadRequest(
                    "economics reward categories must be sorted and unique".into(),
                ));
            }

            if cap.pool_bps == 0 {
                return Err(RewarderError::BadRequest(format!(
                    "economics reward category {category} has zero pool_bps"
                )));
            }

            if cap.category_cap_minor.get() == 0
                || cap.category_cap_minor > self.epoch_pool_cap_minor
            {
                return Err(RewarderError::BadRequest(format!(
                    "economics reward category {category} has invalid absolute cap"
                )));
            }

            category_bps_total = category_bps_total
                .checked_add(u32::from(cap.pool_bps))
                .ok_or_else(|| {
                    RewarderError::BadRequest("economics reward category bps total overflow".into())
                })?;

            previous_category = Some(category);
        }

        if category_bps_total != u32::from(INTERNAL_ROC_BPS_DENOMINATOR) {
            return Err(RewarderError::BadRequest(format!(
                "economics reward category pool_bps must total {}",
                INTERNAL_ROC_BPS_DENOMINATOR
            )));
        }

        if self.max_events_per_account_per_epoch == 0 {
            return Err(RewarderError::BadRequest(
                "economics max events per account must be > 0".into(),
            ));
        }

        if self.max_reward_minor_per_account_per_epoch.get() == 0 {
            return Err(RewarderError::BadRequest(
                "economics account reward cap must be > 0".into(),
            ));
        }

        if self.max_reward_minor_per_content_per_epoch.get() == 0 {
            return Err(RewarderError::BadRequest(
                "economics content reward cap must be > 0".into(),
            ));
        }

        if !self.bridge_inert {
            return Err(RewarderError::BadRequest(
                "economics bridge posture must remain inert".into(),
            ));
        }

        if !self.staking_inert {
            return Err(RewarderError::BadRequest(
                "economics staking posture must remain inert".into(),
            ));
        }

        if self.rounding_mode != "floor" {
            return Err(RewarderError::BadRequest(
                "reward planning requires floor rounding".into(),
            ));
        }

        if self.remainder_sink.trim().is_empty() {
            return Err(RewarderError::BadRequest(
                "economics remainder sink must be explicit".into(),
            ));
        }

        Ok(())
    }

    /// Build a reward policy using config-derived payout caps.
    ///
    /// # Errors
    ///
    /// Returns `RewarderError::BadRequest` if the generated policy
    /// fails existing reward-policy validation.
    pub fn to_reward_policy(&self, policy_id: &str, policy_hash: &str) -> Result<RewardPolicy> {
        self.validate_binding()?;

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

/// Load the reviewed canonical economics profile used by the normal
/// rewarder compute path.
///
/// Development-profile selection remains explicit through
/// `load_internal_roc_planning_economics_toml`.
///
/// # Errors
///
/// Returns `RewarderError::BadRequest` if the checked-in canonical
/// profile no longer satisfies the shared schema.
pub fn load_canonical_internal_roc_planning_economics() -> Result<InternalRocRewardPlanningEconomics>
{
    load_internal_roc_planning_economics_toml(CANONICAL_ROC_ECONOMICS_TOML)
}

/// Parse and validate one complete Internal ROC economics document,
/// then project its reward-planning values.
///
/// Validation, profile parsing, nested paid-action validation,
/// normalization, and economics identity all come from `ron-policy`.
///
/// # Errors
///
/// Returns `RewarderError::BadRequest` when the supplied document
/// fails shared economics parsing, validation, hashing, or numeric
/// projection.
pub fn load_internal_roc_planning_economics_toml(
    bytes: &[u8],
) -> Result<InternalRocRewardPlanningEconomics> {
    let config = load_internal_roc_economics_toml(bytes)
        .map_err(|error| RewarderError::BadRequest(format!("invalid economics config: {error}")))?;

    project_reward_planning_economics(config)
}

/// Parse one complete Internal ROC economics TOML string and project
/// its reward-planning values.
///
/// # Errors
///
/// Returns `RewarderError::BadRequest` when shared economics
/// validation or numeric projection fails.
pub fn load_internal_roc_planning_economics_toml_str(
    raw: &str,
) -> Result<InternalRocRewardPlanningEconomics> {
    load_internal_roc_planning_economics_toml(raw.as_bytes())
}

fn project_reward_planning_economics(
    config: InternalRocEconomicsConfig,
) -> Result<InternalRocRewardPlanningEconomics> {
    let economics_config_hash = internal_roc_economics_config_hash(&config).map_err(|error| {
        RewarderError::BadRequest(format!("invalid economics identity: {error}"))
    })?;

    let epoch_pool_cap_minor = parse_validated_money(
        "reward_pools.epoch_pool_cap_minor",
        &config.reward_pools.epoch_pool_cap_minor,
    )?;

    let max_reward_minor_per_account_per_epoch = parse_validated_money(
        "anti_farming.max_reward_minor_per_account_per_epoch",
        &config.anti_farming.max_reward_minor_per_account_per_epoch,
    )?;

    let max_reward_minor_per_content_per_epoch = parse_validated_money(
        "anti_farming.max_reward_minor_per_content_per_epoch",
        &config.anti_farming.max_reward_minor_per_content_per_epoch,
    )?;

    let mut category_caps = config
        .reward_pools
        .category_caps
        .iter()
        .map(|cap| {
            Ok(InternalRocRewardCategoryPlanningCap {
                category: cap.category.clone(),
                pool_bps: cap.pool_bps,
                category_cap_minor: parse_validated_money(
                    &format!(
                        "reward_pools.category_caps.{}.category_cap_minor",
                        cap.category
                    ),
                    &cap.category_cap_minor,
                )?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    category_caps.sort_by(|left, right| left.category.cmp(&right.category));

    let rounding_mode = match config.rounding.mode {
        InternalRocRoundingMode::Floor => "floor",
    };

    let remainder_sink = match config.rounding.remainder_sink {
        InternalRocRemainderSink::Treasury => "treasury",
        InternalRocRemainderSink::Burn => "burn",
        InternalRocRemainderSink::StabilityBuffer => "stability_buffer",
        InternalRocRemainderSink::ConfiguredAccount => "configured_account",
    };

    let projected = InternalRocRewardPlanningEconomics {
        schema: config.schema,
        version: config.version,
        profile: config.profile.as_str().to_owned(),
        economics_config_hash,
        epoch_pool_cap_minor,
        category_caps,
        max_events_per_account_per_epoch: config.anti_farming.max_events_per_account_per_epoch,
        max_reward_minor_per_account_per_epoch,
        max_reward_minor_per_content_per_epoch,
        rounding_mode: rounding_mode.to_owned(),
        remainder_sink: remainder_sink.to_owned(),
        bridge_inert: !config.future_bridge.enabled,
        staking_inert: !config.future_staking.enabled,
    };

    projected.validate_binding()?;
    Ok(projected)
}

fn parse_validated_money(field: &str, value: &str) -> Result<AmountMinor> {
    value.parse::<u128>().map(AmountMinor).map_err(|error| {
        RewarderError::BadRequest(format!(
            "validated economics field {field} failed numeric projection: {error}"
        ))
    })
}
