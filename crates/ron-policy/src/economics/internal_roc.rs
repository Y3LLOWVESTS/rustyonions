//! RO:WHAT — Internal ROC economics TOML validator for Phase 5.
//!
//! RO:WHY — ECON/GOV: policy validates tokenomics config without becoming balance, receipt, or payout truth.
//! RO:INTERACTS — `configs/roc-economics.toml`, svc-rewarder planning, and the svc-wallet execution boundary.
//! RO:INVARIANTS — no floats; integer minor-unit strings; exact bps totals; explicit remainder sink; bridge/staking inert.
//! RO:METRICS — callers may count validation failures by reason.
//! RO:CONFIG — validates caller-provided TOML bytes only; performs no file or network IO.
//! RO:SECURITY — no wallet mutation, no ledger mutation, no receipt truth, no paid entitlement truth.
//! RO:TEST — `tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs`.

use serde::{Deserialize, Serialize};

use super::{types::EconomicsPolicy, validate::validate as validate_paid_action_policy};

use crate::errors::Error;

/// Canonical internal ROC economics config schema.
pub const INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA: &str = "internal_roc.economics-config.v1";

/// Canonical internal ROC economics config version.
pub const INTERNAL_ROC_ECONOMICS_CONFIG_VERSION: u16 = 1;

/// Canonical bps denominator.
pub const INTERNAL_ROC_BPS_DENOMINATOR: u16 = 10_000;

const MAX_TOKEN_BYTES: usize = 256;
const MAX_MONEY_DIGITS: usize = 39;

/// Explicit standalone economics profile identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalRocEconomicsProfile {
    /// Reviewed non-development economics.
    Canonical,

    /// Explicit local/private-beta economics.
    Development,
}

impl InternalRocEconomicsProfile {
    /// Stable serialized profile label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Development => "development",
        }
    }
}

/// Internal ROC economics config validation marker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocEconomicsConfigValidation {
    /// Schema that was validated.
    pub schema: String,
    /// Version that was validated.
    pub version: u16,
    /// Explicit profile that was validated.
    pub profile: InternalRocEconomicsProfile,
    /// Whether the config is policy-validated only.
    pub policy_validated_only: bool,
    /// Whether bridge runtime remains disabled.
    pub bridge_inert: bool,
    /// Whether staking runtime remains disabled.
    pub staking_inert: bool,
}

/// Units and denominator metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocUnits {
    /// Money encoding label.
    pub money: String,
    /// Minor unit name.
    pub minor_unit_name: String,
    /// Minor units per display ROC.
    pub minor_units_per_roc: String,
    /// Basis-point denominator.
    pub basis_point_denominator: u16,
}

/// Basis-point split row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocBpsSplit {
    /// Split label.
    pub label: String,
    /// Account role.
    pub account_role: String,
    /// Split bps.
    pub bps: u16,
}

/// Paid-content economics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocPaidContentEconomics {
    /// Minimum price in ROC minor units.
    pub minimum_price_minor: String,
    /// Default split rows.
    pub default_splits: Vec<InternalRocBpsSplit>,
}

/// Reward category cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocRewardCategoryCap {
    /// Reward category.
    pub category: String,
    /// Category bps of pool.
    pub pool_bps: u16,
    /// Absolute category cap in ROC minor units.
    pub category_cap_minor: String,
}

/// Reward pool economics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocRewardPoolEconomics {
    /// Absolute epoch pool cap in ROC minor units.
    pub epoch_pool_cap_minor: String,
    /// Category cap rows.
    pub category_caps: Vec<InternalRocRewardCategoryCap>,
}

/// Anti-farming config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocAntiFarmingConfig {
    /// Max events per account per epoch.
    pub max_events_per_account_per_epoch: u64,
    /// Max reward per account per epoch.
    pub max_reward_minor_per_account_per_epoch: String,
    /// Max reward per content per epoch.
    pub max_reward_minor_per_content_per_epoch: String,
    /// Max reward planned for one probation Service Node per epoch.
    pub probation_reward_cap_minor_per_node_per_epoch: String,
}

/// Rounding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalRocRoundingMode {
    /// Floor rounding.
    Floor,
}

/// Remainder sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalRocRemainderSink {
    /// Treasury sink.
    Treasury,
    /// Burn sink.
    Burn,
    /// Stability buffer sink.
    StabilityBuffer,
    /// Configured account sink.
    ConfiguredAccount,
}

/// Rounding config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocRoundingConfig {
    /// Rounding mode.
    pub mode: InternalRocRoundingMode,
    /// Remainder sink.
    pub remainder_sink: InternalRocRemainderSink,
    /// Required for configured-account sink.
    #[serde(default)]
    pub remainder_sink_account: Option<String>,
}

/// Disabled placeholder for future runtime features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocFutureFeaturePlaceholder {
    /// Must remain false in the current beta phase.
    pub enabled: bool,
    /// Inert state label.
    pub state: String,
}

/// Canonical internal ROC economics config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocEconomicsConfig {
    /// Schema tag.
    pub schema: String,
    /// Schema version.
    pub version: u16,
    /// Explicit standalone profile identity.
    pub profile: InternalRocEconomicsProfile,
    /// Paid-action prices, limits, roles, and payout splits.
    pub paid_actions: EconomicsPolicy,
    /// Unit metadata.
    pub units: InternalRocUnits,
    /// Paid-content economics.
    pub paid_content: InternalRocPaidContentEconomics,
    /// Reward-pool economics.
    pub reward_pools: InternalRocRewardPoolEconomics,
    /// Anti-farming caps.
    pub anti_farming: InternalRocAntiFarmingConfig,
    /// Rounding behavior.
    pub rounding: InternalRocRoundingConfig,
    /// Future bridge placeholder.
    pub future_bridge: InternalRocFutureFeaturePlaceholder,
    /// Future staking placeholder.
    pub future_staking: InternalRocFutureFeaturePlaceholder,
}

/// Parse and validate canonical internal ROC economics TOML bytes.
///
/// # Errors
///
/// Returns `Error::Parse` for malformed UTF-8/TOML and `Error::Validation` for unsafe economics config.
pub fn load_internal_roc_economics_toml(bytes: &[u8]) -> Result<InternalRocEconomicsConfig, Error> {
    let raw = std::str::from_utf8(bytes).map_err(|err| Error::Parse(err.to_string()))?;
    load_internal_roc_economics_toml_str(raw)
}

/// Parse one complete economics document and require a selected profile.
///
/// # Errors
///
/// Returns an error when parsing or validation fails, or when the
/// document's explicit profile differs from `expected_profile`.
pub fn load_internal_roc_economics_toml_for_profile(
    bytes: &[u8],
    expected_profile: InternalRocEconomicsProfile,
) -> Result<InternalRocEconomicsConfig, Error> {
    let config = load_internal_roc_economics_toml(bytes)?;

    if config.profile != expected_profile {
        return Err(Error::Validation(format!(
            "economics profile mismatch: expected {}, got {}",
            expected_profile.as_str(),
            config.profile.as_str()
        )));
    }

    Ok(config)
}

/// Parse and validate canonical internal ROC economics TOML text.
///
/// # Errors
///
/// Returns `Error::Parse` for malformed TOML and `Error::Validation` for unsafe economics config.
pub fn load_internal_roc_economics_toml_str(
    raw: &str,
) -> Result<InternalRocEconomicsConfig, Error> {
    let config = toml::from_str::<InternalRocEconomicsConfig>(raw)
        .map_err(|err| Error::Parse(err.to_string()))?;
    validate_internal_roc_economics_config(&config)?;
    Ok(config)
}

/// Validate a parsed internal ROC economics config.
///
/// # Errors
///
/// Returns `Error::Validation` for invalid schema/version, malformed money, invalid bps totals,
/// missing remainder sink data, or enabled bridge/staking placeholders.
pub fn validate_internal_roc_economics_config(
    config: &InternalRocEconomicsConfig,
) -> Result<InternalRocEconomicsConfigValidation, Error> {
    if config.schema != INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA {
        return Err(Error::Validation(
            "invalid internal ROC economics config schema".into(),
        ));
    }
    if config.version != INTERNAL_ROC_ECONOMICS_CONFIG_VERSION {
        return Err(Error::Validation(
            "invalid internal ROC economics config version".into(),
        ));
    }

    validate_bound_paid_actions(config)?;
    validate_token("units.money", &config.units.money)?;
    validate_token("units.minor_unit_name", &config.units.minor_unit_name)?;
    validate_positive_money(
        "units.minor_units_per_roc",
        &config.units.minor_units_per_roc,
    )?;
    if config.units.basis_point_denominator != INTERNAL_ROC_BPS_DENOMINATOR {
        return Err(Error::Validation(
            "units.basis_point_denominator must be 10000".into(),
        ));
    }

    validate_positive_money(
        "paid_content.minimum_price_minor",
        &config.paid_content.minimum_price_minor,
    )?;
    validate_split_rows(
        "paid_content.default_splits",
        &config.paid_content.default_splits,
    )?;

    validate_positive_money(
        "reward_pools.epoch_pool_cap_minor",
        &config.reward_pools.epoch_pool_cap_minor,
    )?;
    validate_category_caps(&config.reward_pools.category_caps)?;

    validate_anti_farming(&config.anti_farming)?;

    validate_rounding(&config.rounding)?;

    validate_future_placeholder("future_bridge", &config.future_bridge)?;
    validate_future_placeholder("future_staking", &config.future_staking)?;

    Ok(InternalRocEconomicsConfigValidation {
        schema: config.schema.clone(),
        version: config.version,
        profile: config.profile,
        policy_validated_only: true,
        bridge_inert: !config.future_bridge.enabled,
        staking_inert: !config.future_staking.enabled,
    })
}

/// Return a validated copy with semantically unordered rows sorted.
///
/// This normalization makes config identity independent of TOML
/// comments, whitespace, and the order of split/category rows.
///
/// # Errors
///
/// Returns `Error::Validation` when the supplied model violates the
/// existing canonical Internal ROC economics schema.
pub fn normalized_internal_roc_economics_config(
    config: &InternalRocEconomicsConfig,
) -> Result<InternalRocEconomicsConfig, Error> {
    validate_internal_roc_economics_config(config)?;

    let mut normalized = config.clone();

    normalized
        .paid_content
        .default_splits
        .sort_by(|left, right| {
            (left.label.as_str(), left.account_role.as_str(), left.bps).cmp(&(
                right.label.as_str(),
                right.account_role.as_str(),
                right.bps,
            ))
        });

    normalized
        .reward_pools
        .category_caps
        .sort_by(|left, right| {
            (
                left.category.as_str(),
                left.pool_bps,
                left.category_cap_minor.as_str(),
            )
                .cmp(&(
                    right.category.as_str(),
                    right.pool_bps,
                    right.category_cap_minor.as_str(),
                ))
        });

    for action in normalized.paid_actions.actions.values_mut() {
        action.splits.sort_by(|left, right| {
            (left.to.as_str(), left.bps).cmp(&(right.to.as_str(), right.bps))
        });
    }

    Ok(normalized)
}

/// Canonical JSON bytes for a validated Internal ROC economics config.
///
/// # Errors
///
/// Returns `Error::Validation` when validation or deterministic
/// serialization fails.
pub fn canonical_internal_roc_economics_bytes(
    config: &InternalRocEconomicsConfig,
) -> Result<Vec<u8>, Error> {
    let normalized = normalized_internal_roc_economics_config(config)?;

    serde_json::to_vec(&normalized).map_err(|error| {
        Error::Validation(format!("economics canonical serialization failed: {error}"))
    })
}

/// BLAKE3 identity of the validated normalized economics model.
///
/// The returned value uses canonical `b3:<64 lowercase hex>` form.
///
/// # Errors
///
/// Returns `Error::Validation` when validation or canonical
/// serialization fails.
pub fn internal_roc_economics_config_hash(
    config: &InternalRocEconomicsConfig,
) -> Result<String, Error> {
    let bytes = canonical_internal_roc_economics_bytes(config)?;

    Ok(format!("b3:{}", blake3::hash(&bytes).to_hex()))
}

fn validate_anti_farming(config: &InternalRocAntiFarmingConfig) -> Result<(), Error> {
    if config.max_events_per_account_per_epoch == 0 {
        return Err(Error::Validation(
            "anti_farming.max_events_per_account_per_epoch must be > 0".into(),
        ));
    }

    validate_positive_money(
        "anti_farming.max_reward_minor_per_account_per_epoch",
        &config.max_reward_minor_per_account_per_epoch,
    )?;
    validate_positive_money(
        "anti_farming.max_reward_minor_per_content_per_epoch",
        &config.max_reward_minor_per_content_per_epoch,
    )?;
    validate_positive_money(
        "anti_farming.probation_reward_cap_minor_per_node_per_epoch",
        &config.probation_reward_cap_minor_per_node_per_epoch,
    )?;

    let account_reward_cap = config
        .max_reward_minor_per_account_per_epoch
        .parse::<u128>()
        .map_err(|error| {
            Error::Validation(format!(
                "anti_farming.max_reward_minor_per_account_per_epoch must fit u128: {error}"
            ))
        })?;

    let probation_reward_cap = config
        .probation_reward_cap_minor_per_node_per_epoch
        .parse::<u128>()
        .map_err(|error| {
            Error::Validation(format!(
                "anti_farming.probation_reward_cap_minor_per_node_per_epoch must fit u128: {error}"
            ))
        })?;

    if probation_reward_cap > account_reward_cap {
        return Err(Error::Validation(
            "anti_farming probation reward cap must not exceed the normal account reward cap"
                .into(),
        ));
    }

    Ok(())
}

fn validate_rounding(config: &InternalRocRoundingConfig) -> Result<(), Error> {
    match config.remainder_sink {
        InternalRocRemainderSink::ConfiguredAccount => {
            let account = config.remainder_sink_account.as_deref().ok_or_else(|| {
                Error::Validation(
                    "rounding.remainder_sink_account required for configured_account".into(),
                )
            })?;

            validate_token("rounding.remainder_sink_account", account)
        }
        InternalRocRemainderSink::Treasury
        | InternalRocRemainderSink::Burn
        | InternalRocRemainderSink::StabilityBuffer => {
            if config.remainder_sink_account.is_some() {
                return Err(Error::Validation(
                    "rounding.remainder_sink_account is only allowed for configured_account".into(),
                ));
            }

            Ok(())
        }
    }
}

fn validate_bound_paid_actions(config: &InternalRocEconomicsConfig) -> Result<(), Error> {
    if config.paid_actions.version != u32::from(config.version) {
        return Err(Error::Validation(
            "paid_actions.version must match config version".into(),
        ));
    }

    if config.paid_actions.unit != config.units.minor_unit_name {
        return Err(Error::Validation(
            "paid_actions.unit must match units.minor_unit_name".into(),
        ));
    }

    validate_paid_action_policy(&config.paid_actions)
}

fn validate_split_rows(field: &str, rows: &[InternalRocBpsSplit]) -> Result<(), Error> {
    if rows.is_empty() {
        return Err(Error::Validation(format!("{field} must not be empty")));
    }

    let mut total = 0_u32;
    for row in rows {
        validate_token("split.label", &row.label)?;
        validate_token("split.account_role", &row.account_role)?;
        validate_positive_bps("split.bps", row.bps)?;
        total = total
            .checked_add(u32::from(row.bps))
            .ok_or_else(|| Error::Validation(format!("{field} bps total overflowed")))?;
    }

    validate_bps_total(field, total)
}

fn validate_category_caps(field: &[InternalRocRewardCategoryCap]) -> Result<(), Error> {
    if field.is_empty() {
        return Err(Error::Validation(
            "reward_pools.category_caps must not be empty".into(),
        ));
    }

    let mut total = 0_u32;
    for row in field {
        validate_token("reward_pools.category_caps.category", &row.category)?;
        validate_positive_bps("reward_pools.category_caps.pool_bps", row.pool_bps)?;
        validate_positive_money(
            "reward_pools.category_caps.category_cap_minor",
            &row.category_cap_minor,
        )?;
        total = total.checked_add(u32::from(row.pool_bps)).ok_or_else(|| {
            Error::Validation("reward_pools.category_caps bps total overflowed".into())
        })?;
    }

    validate_bps_total("reward_pools.category_caps", total)
}

fn validate_bps_total(field: &str, total: u32) -> Result<(), Error> {
    let expected = u32::from(INTERNAL_ROC_BPS_DENOMINATOR);
    if total != expected {
        return Err(Error::Validation(format!(
            "{field} bps must total {expected}; got {total}"
        )));
    }
    Ok(())
}

fn validate_positive_bps(field: &str, bps: u16) -> Result<(), Error> {
    if bps == 0 || bps > INTERNAL_ROC_BPS_DENOMINATOR {
        return Err(Error::Validation(format!(
            "{field} must be in 1..={INTERNAL_ROC_BPS_DENOMINATOR}"
        )));
    }
    Ok(())
}

fn validate_positive_money(field: &str, value: &str) -> Result<(), Error> {
    validate_money(field, value)?;
    if value == "0" {
        return Err(Error::Validation(format!("{field} must be > 0")));
    }
    Ok(())
}

fn validate_money(field: &str, value: &str) -> Result<(), Error> {
    if value.is_empty() {
        return Err(Error::Validation(format!("{field} must not be empty")));
    }
    if value.len() > MAX_MONEY_DIGITS {
        return Err(Error::Validation(format!(
            "{field} exceeds u128 decimal width"
        )));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(Error::Validation(format!(
            "{field} must not have leading zeroes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Validation(format!(
            "{field} must be decimal integer minor units"
        )));
    }
    value
        .parse::<u128>()
        .map_err(|err| Error::Validation(format!("{field} must fit u128: {err}")))?;
    Ok(())
}

fn validate_future_placeholder(
    field: &str,
    value: &InternalRocFutureFeaturePlaceholder,
) -> Result<(), Error> {
    if value.enabled {
        return Err(Error::Validation(format!(
            "{field}.enabled must remain false/inert"
        )));
    }
    validate_token(field, &value.state)
}

fn validate_token(field: &str, value: &str) -> Result<(), Error> {
    if value.is_empty() {
        return Err(Error::Validation(format!("{field} must not be empty")));
    }
    if value.len() > MAX_TOKEN_BYTES {
        return Err(Error::Validation(format!(
            "{field} exceeds max token bytes"
        )));
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return Err(Error::Validation(format!(
            "{field} contains unsupported characters"
        )));
    }
    Ok(())
}
