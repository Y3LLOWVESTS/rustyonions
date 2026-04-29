//! RO:WHAT — Strict DTOs for ROC economics policy.
//!
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/DX. Stable config shape for pricing, caps, and splits.
//!
//! RO:INTERACTS — `economics::{load,validate}` and future paid-action consumers.
//!
//! RO:INVARIANTS — no floats; all money values are integer minor units; percentages are basis points.
//!
//! RO:METRICS — none directly.
//!
//! RO:CONFIG — mirrors `configs/roc-economics.toml`.
//!
//! RO:SECURITY — policy data only; no secrets or wallet authority.
//!
//! RO:TEST — `economics_policy.rs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Top-level ROC economics policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomicsPolicy {
    /// Config schema version.
    pub version: u32,
    /// Unit label. Expected: `roc_minor`.
    pub unit: String,
    /// Default asset symbol. Expected: `roc`.
    pub default_asset: String,
    /// Rounding behavior for deterministic math.
    pub rounding: RoundingMode,
    /// Account key receiving residual/remainder.
    pub remainder_sink: String,
    /// Static account aliases such as treasury/reward pool.
    #[serde(default)]
    pub accounts: BTreeMap<String, String>,
    /// Dynamic recipient roles such as storage provider/content owner.
    #[serde(default)]
    pub roles: BTreeMap<String, String>,
    /// Global spend limits.
    #[serde(default)]
    pub limits: EconomicsLimits,
    /// Per-action economics.
    #[serde(default)]
    pub actions: BTreeMap<String, ActionEconomics>,
}

/// Global economics limits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomicsLimits {
    /// Default daily cap for an identified passport/account.
    #[serde(default)]
    pub default_daily_spend_cap_minor: u64,
    /// Default single action cap.
    #[serde(default)]
    pub default_single_action_cap_minor: u64,
    /// Default daily cap for anonymous/alt identity mode.
    #[serde(default)]
    pub anonymous_daily_spend_cap_minor: u64,
}

/// Per-action pricing and payout split policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionEconomics {
    /// If false, the action exists but lookup/capture validation fails closed.
    pub enabled: bool,
    /// Pricing strategy.
    pub pricing_kind: PricingKind,
    /// Flat price for `PricingKind::Flat`.
    #[serde(default)]
    pub price_minor: Option<u64>,
    /// Per-byte price for `PricingKind::PerBytePlusMinimum`.
    #[serde(default)]
    pub price_per_byte_minor: Option<u64>,
    /// Per-second price for `PricingKind::PerSecondPlusMinimum`.
    #[serde(default)]
    pub price_per_second_minor: Option<u64>,
    /// Minimum charge for this action.
    pub minimum_charge_minor: u64,
    /// Maximum spend/capture allowed for this action.
    pub max_spend_minor: u64,
    /// Hold multiplier in basis points. `10000` means 100%.
    pub max_hold_multiplier_bps: u32,
    /// Payout splits. Must sum to `10_000` bps.
    #[serde(default)]
    pub splits: Vec<PayoutSplit>,
}

/// Deterministic pricing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingKind {
    /// Fixed price.
    Flat,
    /// Per byte plus configured minimum.
    PerBytePlusMinimum,
    /// Per second plus configured minimum.
    PerSecondPlusMinimum,
}

/// Deterministic rounding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoundingMode {
    /// Floor division.
    Floor,
}

/// A single basis-point split destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayoutSplit {
    /// Destination alias or role.
    pub to: String,
    /// Basis points. Total per action must sum to `10_000`.
    pub bps: u16,
}
