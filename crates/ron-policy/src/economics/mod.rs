//! RO:WHAT — ROC economics policy parser, validator, and lookup helpers.
//!
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/DX. Paid actions need deterministic pricing and payout splits.
//!
//! RO:INTERACTS — `economics::{types,load,validate}` and future paid-action consumers.
//!
//! RO:INVARIANTS — integer minor units only; basis points only; splits sum to `10_000`; fail closed.
//!
//! RO:METRICS — consumers should expose `roc_economics_*` metrics after calling these helpers.
//!
//! RO:CONFIG — caller-provided TOML bytes, normally from `configs/roc-economics.toml`.
//!
//! RO:SECURITY — no wallet mutation; no ledger mutation; no network or file I/O inside this library.
//!
//! RO:TEST — `crates/ron-policy/tests/economics_policy.rs`.

#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;

use crate::errors::Error;

pub mod load;
pub mod types;
pub mod validate;

pub use load::{from_slice as load_economics_toml, from_str as load_economics_toml_str};
pub use types::{
    ActionEconomics, EconomicsLimits, EconomicsPolicy, PayoutSplit, PricingKind, RoundingMode,
};
pub use validate::validate as validate_economics_policy;

/// Known beta action identifiers accepted by the economics policy validator.
pub const BETA_ACTIONS: [&str; 5] = [
    "paid_storage_put",
    "paid_storage_pin",
    "paid_content_view",
    "paid_song_play",
    "site_visit",
];

impl EconomicsPolicy {
    /// Return deterministic action IDs in canonical sorted order.
    #[must_use]
    pub fn action_ids(&self) -> Vec<String> {
        self.actions.keys().cloned().collect()
    }

    /// Return the enabled action policy for `action_id`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the action is unknown or disabled.
    pub fn enabled_action(&self, action_id: &str) -> Result<&ActionEconomics, Error> {
        let action = self
            .actions
            .get(action_id)
            .ok_or_else(|| Error::Validation(format!("unknown economics action: {action_id}")))?;

        if !action.enabled {
            return Err(Error::Validation(format!(
                "economics action disabled: {action_id}"
            )));
        }

        Ok(action)
    }

    /// Estimate the hold amount for an action and quantity.
    ///
    /// For flat pricing, `quantity` is ignored. For per-byte and per-second actions, `quantity`
    /// is the measured byte/second count. The returned amount includes the configured hold
    /// multiplier and is capped by the action's `max_spend_minor`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the action is unknown, disabled, malformed, or exceeds caps.
    pub fn price_for(&self, action_id: &str, quantity: u128) -> Result<u128, Error> {
        let action = self.enabled_action(action_id)?;
        let base = action.base_charge_minor(quantity)?;

        let multiplied = base
            .checked_mul(u128::from(action.max_hold_multiplier_bps))
            .ok_or_else(|| {
                Error::Validation(format!("action {action_id} hold multiplier overflowed"))
            })?;

        let hold = multiplied.checked_add(9_999).ok_or_else(|| {
            Error::Validation(format!("action {action_id} hold rounded overflowed"))
        })? / 10_000;

        if hold > u128::from(action.max_spend_minor) {
            return Err(Error::Validation(format!(
                "action {action_id} hold amount exceeds max_spend_minor"
            )));
        }

        Ok(hold)
    }

    /// Validate that a capture plan has recipients for all dynamic split destinations.
    ///
    /// The `recipients` map is keyed by split destination/role name, for example
    /// `storage_provider -> t:default/provider/w`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` if the action is unknown/disabled, amount is invalid, or a
    /// dynamic recipient is missing.
    pub fn validate_capture_plan(
        &self,
        action_id: &str,
        recipients: &BTreeMap<String, String>,
        amount_minor: u128,
    ) -> Result<(), Error> {
        let action = self.enabled_action(action_id)?;

        if amount_minor == 0 {
            return Err(Error::Validation(format!(
                "action {action_id} capture amount must be > 0"
            )));
        }
        if amount_minor > u128::from(action.max_spend_minor) {
            return Err(Error::Validation(format!(
                "action {action_id} capture amount exceeds max_spend_minor"
            )));
        }

        for split in &action.splits {
            if self.roles.contains_key(&split.to) && !recipients.contains_key(&split.to) {
                return Err(Error::Validation(format!(
                    "action {action_id} missing recipient for split destination {}",
                    split.to
                )));
            }
        }

        Ok(())
    }
}

impl ActionEconomics {
    fn base_charge_minor(&self, quantity: u128) -> Result<u128, Error> {
        let raw = match self.pricing_kind {
            PricingKind::Flat => u128::from(
                self.price_minor
                    .ok_or_else(|| Error::Validation("flat action missing price_minor".into()))?,
            ),
            PricingKind::PerBytePlusMinimum => quantity
                .checked_mul(u128::from(self.price_per_byte_minor.ok_or_else(|| {
                    Error::Validation("per-byte action missing price_per_byte_minor".into())
                })?))
                .ok_or_else(|| Error::Validation("per-byte price overflowed".into()))?,
            PricingKind::PerSecondPlusMinimum => quantity
                .checked_mul(u128::from(self.price_per_second_minor.ok_or_else(
                    || Error::Validation("per-second action missing price_per_second_minor".into()),
                )?))
                .ok_or_else(|| Error::Validation("per-second price overflowed".into()))?,
        };

        Ok(raw.max(u128::from(self.minimum_charge_minor)))
    }
}
