//! RO:WHAT — Validation rules for ROC economics policy.
//!
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Invalid economics must fail closed.
//!
//! RO:INTERACTS — `economics::types` and consumer paid-action enforcement.
//!
//! RO:INVARIANTS — integer minor units only; bps totals exactly `10_000`; unknown actions reject.
//!
//! RO:METRICS — consumers should count validation failures by reason.
//!
//! RO:CONFIG — validates caller-provided economics policy.
//!
//! RO:SECURITY — no ambient authority; validates recipient aliases before paid actions use them.
//!
//! RO:TEST — `economics_policy.rs`.

use std::collections::BTreeSet;

use crate::economics::types::{ActionEconomics, EconomicsPolicy, PricingKind, RoundingMode};
use crate::economics::BETA_ACTIONS;
use crate::errors::Error;

/// Validate a ROC economics policy.
///
/// # Errors
///
/// Returns `Error::Validation` if any invariant is broken.
pub fn validate(policy: &EconomicsPolicy) -> Result<(), Error> {
    validate_root(policy)?;
    validate_required_actions(policy)?;

    for (action_id, action) in &policy.actions {
        validate_action_id(action_id)?;
        validate_action(action, action_id, policy)?;
    }

    Ok(())
}

fn validate_root(policy: &EconomicsPolicy) -> Result<(), Error> {
    if policy.version == 0 {
        return Err(Error::Validation("economics.version must be >= 1".into()));
    }
    if policy.unit != "roc_minor" {
        return Err(Error::Validation("economics.unit must be roc_minor".into()));
    }
    if policy.default_asset != "roc" {
        return Err(Error::Validation(
            "economics.default_asset must be roc".into(),
        ));
    }
    if policy.rounding != RoundingMode::Floor {
        return Err(Error::Validation(
            "economics.rounding currently supports only floor".into(),
        ));
    }
    if policy.remainder_sink.trim().is_empty() {
        return Err(Error::Validation(
            "economics.remainder_sink must be non-empty".into(),
        ));
    }
    if !policy.accounts.contains_key(&policy.remainder_sink) {
        return Err(Error::Validation(format!(
            "economics.remainder_sink {} is not a declared account",
            policy.remainder_sink
        )));
    }
    if policy.limits.default_single_action_cap_minor == 0 {
        return Err(Error::Validation(
            "limits.default_single_action_cap_minor must be > 0".into(),
        ));
    }
    if policy.limits.default_daily_spend_cap_minor < policy.limits.default_single_action_cap_minor {
        return Err(Error::Validation(
            "limits.default_daily_spend_cap_minor must be >= default_single_action_cap_minor"
                .into(),
        ));
    }
    if policy.limits.anonymous_daily_spend_cap_minor > policy.limits.default_daily_spend_cap_minor {
        return Err(Error::Validation(
            "limits.anonymous_daily_spend_cap_minor must be <= default_daily_spend_cap_minor"
                .into(),
        ));
    }

    Ok(())
}

fn validate_required_actions(policy: &EconomicsPolicy) -> Result<(), Error> {
    for required in BETA_ACTIONS {
        if !policy.actions.contains_key(required) {
            return Err(Error::Validation(format!(
                "missing required economics action: {required}"
            )));
        }
    }
    Ok(())
}

fn validate_action_id(action_id: &str) -> Result<(), Error> {
    if !BETA_ACTIONS.contains(&action_id) {
        return Err(Error::Validation(format!(
            "unknown economics action: {action_id}"
        )));
    }
    Ok(())
}

fn validate_action(
    action: &ActionEconomics,
    action_id: &str,
    policy: &EconomicsPolicy,
) -> Result<(), Error> {
    if action.max_hold_multiplier_bps < 10_000 {
        return Err(Error::Validation(format!(
            "action {action_id} max_hold_multiplier_bps must be >= 10000"
        )));
    }
    if action.max_spend_minor == 0 {
        return Err(Error::Validation(format!(
            "action {action_id} max_spend_minor must be > 0"
        )));
    }
    if action.minimum_charge_minor == 0 {
        return Err(Error::Validation(format!(
            "action {action_id} minimum_charge_minor must be > 0"
        )));
    }
    if action.max_spend_minor < action.minimum_charge_minor {
        return Err(Error::Validation(format!(
            "action {action_id} max_spend_minor must be >= minimum_charge_minor"
        )));
    }
    if action.max_spend_minor > policy.limits.default_daily_spend_cap_minor {
        return Err(Error::Validation(format!(
            "action {action_id} max_spend_minor exceeds default_daily_spend_cap_minor"
        )));
    }

    validate_pricing(action, action_id)?;
    validate_splits(action, action_id, policy)?;
    Ok(())
}

fn validate_pricing(action: &ActionEconomics, action_id: &str) -> Result<(), Error> {
    match action.pricing_kind {
        PricingKind::Flat => {
            let price = action.price_minor.ok_or_else(|| {
                Error::Validation(format!(
                    "action {action_id} flat pricing missing price_minor"
                ))
            })?;
            if price == 0 {
                return Err(Error::Validation(format!(
                    "action {action_id} price_minor must be > 0"
                )));
            }
            if price > action.max_spend_minor {
                return Err(Error::Validation(format!(
                    "action {action_id} price_minor exceeds max_spend_minor"
                )));
            }
        }
        PricingKind::PerBytePlusMinimum => {
            let price = action.price_per_byte_minor.ok_or_else(|| {
                Error::Validation(format!(
                    "action {action_id} per-byte pricing missing price_per_byte_minor"
                ))
            })?;
            if price == 0 {
                return Err(Error::Validation(format!(
                    "action {action_id} price_per_byte_minor must be > 0"
                )));
            }
        }
        PricingKind::PerSecondPlusMinimum => {
            let price = action.price_per_second_minor.ok_or_else(|| {
                Error::Validation(format!(
                    "action {action_id} per-second pricing missing price_per_second_minor"
                ))
            })?;
            if price == 0 {
                return Err(Error::Validation(format!(
                    "action {action_id} price_per_second_minor must be > 0"
                )));
            }
        }
    }

    Ok(())
}

fn validate_splits(
    action: &ActionEconomics,
    action_id: &str,
    policy: &EconomicsPolicy,
) -> Result<(), Error> {
    if action.splits.is_empty() {
        return Err(Error::Validation(format!(
            "action {action_id} must have at least one split"
        )));
    }

    let mut total = 0_u32;
    let mut seen = BTreeSet::<&str>::new();

    for split in &action.splits {
        if split.to.trim().is_empty() {
            return Err(Error::Validation(format!(
                "action {action_id} split destination must be non-empty"
            )));
        }
        if split.bps == 0 {
            return Err(Error::Validation(format!(
                "action {action_id} split {} bps must be > 0",
                split.to
            )));
        }
        if !seen.insert(split.to.as_str()) {
            return Err(Error::Validation(format!(
                "action {action_id} duplicate split destination {}",
                split.to
            )));
        }
        if !policy.accounts.contains_key(&split.to) && !policy.roles.contains_key(&split.to) {
            return Err(Error::Validation(format!(
                "action {action_id} unknown split destination {}",
                split.to
            )));
        }

        total = total
            .checked_add(u32::from(split.bps))
            .ok_or_else(|| Error::Validation(format!("action {action_id} split bps overflow")))?;
    }

    if total != 10_000 {
        return Err(Error::Validation(format!(
            "action {action_id} split bps must sum to 10000, got {total}"
        )));
    }

    Ok(())
}
