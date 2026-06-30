//! RO:WHAT — Internal ROC economics config DTOs for Phase 5 TOML/schema hardening.
//! RO:WHY — ECON/GOV: tokenomics values must be integer-safe data before policy/rewarder planning.
//! RO:INTERACTS — configs/roc-economics.toml, ron-policy validation, svc-rewarder planning.
//! RO:INVARIANTS — DTO-only; no IO; no floats; bps totals explicit; bridge/staking placeholders inert.
//! RO:METRICS — none.
//! RO:CONFIG — mirrors the canonical config shape without reading files.
//! RO:SECURITY — config data cannot mint ROC, mutate ledger, create receipt truth, or unlock paid content.
//! RO:TEST — tests/internal_roc_beta_phase5_economics_config_dto.rs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical schema tag for the internal ROC economics config DTO.
pub const INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA: &str = "internal_roc.economics-config.v1";

/// Current internal ROC economics config DTO version.
pub const INTERNAL_ROC_ECONOMICS_CONFIG_VERSION: u16 = 1;

/// Basis-point denominator used by internal ROC split and pool config.
pub const INTERNAL_ROC_BPS_DENOMINATOR: u16 = 10_000;

const MAX_REF_BYTES: usize = 256;
const MAX_MONEY_DIGITS: usize = 39;
const MAX_SPLITS: usize = 32;
const MAX_CATEGORIES: usize = 64;

/// Validation errors for the internal ROC economics config DTO.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum InternalRocEconomicsConfigValidationError {
    /// Schema tag does not match the canonical beta schema.
    #[error("invalid internal ROC economics config schema")]
    InvalidSchema,

    /// Version does not match the canonical beta config version.
    #[error("invalid internal ROC economics config version")]
    InvalidVersion,

    /// A required text field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Field name.
        field: &'static str,
    },

    /// A bounded text field exceeds its maximum byte length.
    #[error("{field} exceeds max bytes")]
    OverlongField {
        /// Field name.
        field: &'static str,
        /// Maximum allowed bytes.
        max: usize,
        /// Actual bytes.
        actual: usize,
    },

    /// A text field contains unsupported characters.
    #[error("{field} contains unsupported characters")]
    InvalidToken {
        /// Field name.
        field: &'static str,
    },

    /// A money field is not a canonical integer minor-unit string.
    #[error("{field} must be a canonical integer minor-unit string")]
    InvalidMoney {
        /// Field name.
        field: &'static str,
        /// Reason.
        reason: &'static str,
    },

    /// A basis-point field is outside allowed bounds.
    #[error("{field} has invalid bps")]
    InvalidBps {
        /// Field name.
        field: &'static str,
    },

    /// A basis-point list does not sum to exactly 10000 bps.
    #[error("{field} bps total mismatch")]
    InvalidBpsTotal {
        /// Field name.
        field: &'static str,
        /// Expected bps.
        expected_bps: u32,
        /// Actual bps.
        actual_bps: u32,
    },

    /// A bounded list is empty.
    #[error("{field} must not be empty")]
    EmptyList {
        /// Field name.
        field: &'static str,
    },

    /// A bounded list contains too many items.
    #[error("{field} contains too many items")]
    TooManyItems {
        /// Field name.
        field: &'static str,
        /// Maximum item count.
        max: usize,
        /// Actual item count.
        actual: usize,
    },

    /// A count/cap field that must be positive was zero.
    #[error("{field} must be greater than zero")]
    ZeroCap {
        /// Field name.
        field: &'static str,
    },

    /// A configured-account remainder sink requires a sink account.
    #[error("configured_account remainder sink requires an account")]
    MissingRemainderSinkAccount,

    /// A non-configured-account remainder sink must not carry a sink account.
    #[error("non-configured-account remainder sink must not carry an account")]
    UnexpectedRemainderSinkAccount,

    /// A future feature placeholder was enabled before authorization.
    #[error("{field} must remain disabled and inert")]
    FutureFeatureEnabled {
        /// Field name.
        field: &'static str,
    },
}

/// Units and denominator metadata for internal ROC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocUnitsV1 {
    /// Human-readable money encoding label, e.g. `integer_minor_units`.
    pub money: String,
    /// Name for the smallest internal ROC unit.
    pub minor_unit_name: String,
    /// Number of minor units in one display ROC as a canonical integer string.
    pub minor_units_per_roc: String,
    /// Basis-point denominator; must be 10000.
    pub basis_point_denominator: u16,
}

impl InternalRocUnitsV1 {
    /// Validate units metadata.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        validate_token("units.money", &self.money, MAX_REF_BYTES)?;
        validate_token(
            "units.minor_unit_name",
            &self.minor_unit_name,
            MAX_REF_BYTES,
        )?;
        validate_positive_minor_units("units.minor_units_per_roc", &self.minor_units_per_roc)?;

        if self.basis_point_denominator != INTERNAL_ROC_BPS_DENOMINATOR {
            return Err(InternalRocEconomicsConfigValidationError::InvalidBps {
                field: "units.basis_point_denominator",
            });
        }

        Ok(())
    }
}

/// One named basis-point split line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocBpsSplitV1 {
    /// Stable split label.
    pub label: String,
    /// Role/account label to be resolved by policy/rewarder/wallet layers.
    pub account_role: String,
    /// Integer basis points for this split.
    pub bps: u16,
}

impl InternalRocBpsSplitV1 {
    /// Validate a split row.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        validate_token("split.label", &self.label, MAX_REF_BYTES)?;
        validate_token("split.account_role", &self.account_role, MAX_REF_BYTES)?;
        validate_positive_bps("split.bps", self.bps)
    }
}

/// Paid content economics defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocPaidContentEconomicsV1 {
    /// Minimum paid-content price in ROC minor units.
    pub minimum_price_minor: String,
    /// Default paid-content split. Must total exactly 10000 bps.
    pub default_splits: Vec<InternalRocBpsSplitV1>,
}

impl InternalRocPaidContentEconomicsV1 {
    /// Validate paid content economics.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        validate_positive_minor_units(
            "paid_content.minimum_price_minor",
            &self.minimum_price_minor,
        )?;
        validate_nonempty_bounded_list(
            "paid_content.default_splits",
            self.default_splits.len(),
            MAX_SPLITS,
        )?;

        let mut total = 0_u32;
        for split in &self.default_splits {
            split.validate()?;
            total = total.checked_add(u32::from(split.bps)).ok_or(
                InternalRocEconomicsConfigValidationError::InvalidBpsTotal {
                    field: "paid_content.default_splits",
                    expected_bps: u32::from(INTERNAL_ROC_BPS_DENOMINATOR),
                    actual_bps: u32::MAX,
                },
            )?;
        }

        validate_exact_bps_total("paid_content.default_splits", total)
    }
}

/// Per-category reward pool cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocRewardCategoryCapV1 {
    /// Stable category label.
    pub category: String,
    /// Share of epoch pool allocated to this category.
    pub pool_bps: u16,
    /// Absolute category cap in ROC minor units.
    pub category_cap_minor: String,
}

impl InternalRocRewardCategoryCapV1 {
    /// Validate a category cap.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        validate_token(
            "reward_pools.category_caps.category",
            &self.category,
            MAX_REF_BYTES,
        )?;
        validate_positive_bps("reward_pools.category_caps.pool_bps", self.pool_bps)?;
        validate_positive_minor_units(
            "reward_pools.category_caps.category_cap_minor",
            &self.category_cap_minor,
        )
    }
}

/// Reward pool economics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocRewardPoolEconomicsV1 {
    /// Absolute epoch pool cap in ROC minor units.
    pub epoch_pool_cap_minor: String,
    /// Reward category caps. `pool_bps` must total exactly 10000.
    pub category_caps: Vec<InternalRocRewardCategoryCapV1>,
}

impl InternalRocRewardPoolEconomicsV1 {
    /// Validate reward pool economics.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        validate_positive_minor_units(
            "reward_pools.epoch_pool_cap_minor",
            &self.epoch_pool_cap_minor,
        )?;
        validate_nonempty_bounded_list(
            "reward_pools.category_caps",
            self.category_caps.len(),
            MAX_CATEGORIES,
        )?;

        let mut total = 0_u32;
        for category in &self.category_caps {
            category.validate()?;
            total = total.checked_add(u32::from(category.pool_bps)).ok_or(
                InternalRocEconomicsConfigValidationError::InvalidBpsTotal {
                    field: "reward_pools.category_caps",
                    expected_bps: u32::from(INTERNAL_ROC_BPS_DENOMINATOR),
                    actual_bps: u32::MAX,
                },
            )?;
        }

        validate_exact_bps_total("reward_pools.category_caps", total)
    }
}

/// Anti-farming caps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocAntiFarmingConfigV1 {
    /// Maximum eligible events per account per epoch.
    pub max_events_per_account_per_epoch: u64,
    /// Maximum reward per account per epoch in ROC minor units.
    pub max_reward_minor_per_account_per_epoch: String,
    /// Maximum reward per content item per epoch in ROC minor units.
    pub max_reward_minor_per_content_per_epoch: String,
}

impl InternalRocAntiFarmingConfigV1 {
    /// Validate anti-farming caps.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        if self.max_events_per_account_per_epoch == 0 {
            return Err(InternalRocEconomicsConfigValidationError::ZeroCap {
                field: "anti_farming.max_events_per_account_per_epoch",
            });
        }

        validate_positive_minor_units(
            "anti_farming.max_reward_minor_per_account_per_epoch",
            &self.max_reward_minor_per_account_per_epoch,
        )?;
        validate_positive_minor_units(
            "anti_farming.max_reward_minor_per_content_per_epoch",
            &self.max_reward_minor_per_content_per_epoch,
        )
    }
}

/// Deterministic rounding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InternalRocRoundingModeV1 {
    /// Floor and route every remainder to the configured sink.
    Floor,
}

/// Deterministic remainder sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InternalRocRemainderSinkV1 {
    /// Route remainders to treasury.
    Treasury,
    /// Burn remainders.
    Burn,
    /// Route remainders to a stability buffer.
    StabilityBuffer,
    /// Route remainders to the configured account.
    ConfiguredAccount,
}

/// Rounding and remainder handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocRoundingConfigV1 {
    /// Deterministic rounding mode.
    pub mode: InternalRocRoundingModeV1,
    /// Explicit sink for all remainders.
    pub remainder_sink: InternalRocRemainderSinkV1,
    /// Required only when `remainder_sink` is `configured_account`.
    #[serde(default)]
    pub remainder_sink_account: Option<String>,
}

impl InternalRocRoundingConfigV1 {
    /// Validate rounding/remainder configuration.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        match self.remainder_sink {
            InternalRocRemainderSinkV1::ConfiguredAccount => {
                let account = self.remainder_sink_account.as_deref().ok_or(
                    InternalRocEconomicsConfigValidationError::MissingRemainderSinkAccount,
                )?;
                validate_token("rounding.remainder_sink_account", account, MAX_REF_BYTES)
            }
            InternalRocRemainderSinkV1::Treasury
            | InternalRocRemainderSinkV1::Burn
            | InternalRocRemainderSinkV1::StabilityBuffer => {
                if self.remainder_sink_account.is_some() {
                    return Err(
                        InternalRocEconomicsConfigValidationError::UnexpectedRemainderSinkAccount,
                    );
                }
                Ok(())
            }
        }
    }
}

/// Disabled placeholder for future authorized features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocFutureFeaturePlaceholderV1 {
    /// Must remain false unless a later reviewed decision gate authorizes runtime behavior.
    pub enabled: bool,
    /// Human-readable inert state marker.
    pub state: String,
}

impl InternalRocFutureFeaturePlaceholderV1 {
    /// Validate a disabled future feature placeholder.
    pub fn validate(
        &self,
        field: &'static str,
    ) -> Result<(), InternalRocEconomicsConfigValidationError> {
        if self.enabled {
            return Err(InternalRocEconomicsConfigValidationError::FutureFeatureEnabled { field });
        }

        validate_token(field, &self.state, MAX_REF_BYTES)
    }
}

/// Internal ROC economics config DTO.
///
/// This is policy/reward planning input data only. It is not wallet authority,
/// ledger truth, receipt truth, entitlement truth, bridge runtime, or staking runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalRocEconomicsConfigV1 {
    /// Schema tag.
    pub schema: String,
    /// Version.
    pub version: u16,
    /// Units metadata.
    pub units: InternalRocUnitsV1,
    /// Paid content split defaults.
    pub paid_content: InternalRocPaidContentEconomicsV1,
    /// Reward pool caps.
    pub reward_pools: InternalRocRewardPoolEconomicsV1,
    /// Anti-farming caps.
    pub anti_farming: InternalRocAntiFarmingConfigV1,
    /// Rounding and remainder sink.
    pub rounding: InternalRocRoundingConfigV1,
    /// Future bridge placeholder; must be disabled/inert.
    pub future_bridge: InternalRocFutureFeaturePlaceholderV1,
    /// Future staking placeholder; must be disabled/inert.
    pub future_staking: InternalRocFutureFeaturePlaceholderV1,
}

impl InternalRocEconomicsConfigV1 {
    /// Validate the economics config DTO without reading files or mutating state.
    pub fn validate(&self) -> Result<(), InternalRocEconomicsConfigValidationError> {
        if self.schema != INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA {
            return Err(InternalRocEconomicsConfigValidationError::InvalidSchema);
        }

        if self.version != INTERNAL_ROC_ECONOMICS_CONFIG_VERSION {
            return Err(InternalRocEconomicsConfigValidationError::InvalidVersion);
        }

        self.units.validate()?;
        self.paid_content.validate()?;
        self.reward_pools.validate()?;
        self.anti_farming.validate()?;
        self.rounding.validate()?;
        self.future_bridge.validate("future_bridge.enabled")?;
        self.future_staking.validate("future_staking.enabled")?;

        Ok(())
    }
}

fn validate_exact_bps_total(
    field: &'static str,
    actual_bps: u32,
) -> Result<(), InternalRocEconomicsConfigValidationError> {
    let expected_bps = u32::from(INTERNAL_ROC_BPS_DENOMINATOR);
    if actual_bps == expected_bps {
        return Ok(());
    }

    Err(InternalRocEconomicsConfigValidationError::InvalidBpsTotal {
        field,
        expected_bps,
        actual_bps,
    })
}

fn validate_positive_bps(
    field: &'static str,
    bps: u16,
) -> Result<(), InternalRocEconomicsConfigValidationError> {
    if bps == 0 || bps > INTERNAL_ROC_BPS_DENOMINATOR {
        return Err(InternalRocEconomicsConfigValidationError::InvalidBps { field });
    }

    Ok(())
}

fn validate_nonempty_bounded_list(
    field: &'static str,
    len: usize,
    max: usize,
) -> Result<(), InternalRocEconomicsConfigValidationError> {
    if len == 0 {
        return Err(InternalRocEconomicsConfigValidationError::EmptyList { field });
    }

    if len > max {
        return Err(InternalRocEconomicsConfigValidationError::TooManyItems {
            field,
            max,
            actual: len,
        });
    }

    Ok(())
}

fn validate_positive_minor_units(
    field: &'static str,
    value: &str,
) -> Result<(), InternalRocEconomicsConfigValidationError> {
    validate_minor_units(field, value)?;
    if value == "0" {
        return Err(InternalRocEconomicsConfigValidationError::InvalidMoney {
            field,
            reason: "must be greater than zero",
        });
    }

    Ok(())
}

fn validate_minor_units(
    field: &'static str,
    value: &str,
) -> Result<(), InternalRocEconomicsConfigValidationError> {
    if value.is_empty() {
        return Err(InternalRocEconomicsConfigValidationError::InvalidMoney {
            field,
            reason: "must not be empty",
        });
    }

    if value.len() > MAX_MONEY_DIGITS {
        return Err(InternalRocEconomicsConfigValidationError::InvalidMoney {
            field,
            reason: "must not exceed u128 decimal width",
        });
    }

    if value.len() > 1 && value.starts_with('0') {
        return Err(InternalRocEconomicsConfigValidationError::InvalidMoney {
            field,
            reason: "must be canonical decimal without leading zeroes",
        });
    }

    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(InternalRocEconomicsConfigValidationError::InvalidMoney {
            field,
            reason: "must contain decimal digits only",
        });
    }

    value.parse::<u128>().map_err(
        |_| InternalRocEconomicsConfigValidationError::InvalidMoney {
            field,
            reason: "must fit in u128 minor units",
        },
    )?;

    Ok(())
}

fn validate_token(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), InternalRocEconomicsConfigValidationError> {
    if value.is_empty() {
        return Err(InternalRocEconomicsConfigValidationError::EmptyField { field });
    }

    if value.len() > max_bytes {
        return Err(InternalRocEconomicsConfigValidationError::OverlongField {
            field,
            max: max_bytes,
            actual: value.len(),
        });
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/' | b'@')
    }) {
        return Err(InternalRocEconomicsConfigValidationError::InvalidToken { field });
    }

    Ok(())
}
