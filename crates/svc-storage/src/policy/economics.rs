//! RO:WHAT — Optional ROC economics policy adapter for paid storage pricing.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/DX. Storage can price paid writes from validated ROC config.
//! RO:INTERACTS — ron_policy::economics, policy::settlement, /paid/o and /paid/o/estimate routes.
//! RO:INVARIANTS — integer minor units only; no floating point; fail closed when economics config is explicitly set.
//! RO:METRICS — route layer maps economics failures into paid-write config_error status.
//! RO:CONFIG — RON_STORAGE_ROC_ECONOMICS_PATH, RON_STORAGE_ROC_ECONOMICS_ACTION.
//! RO:SECURITY — reads policy only; no wallet mutation, ledger mutation, ownership, names, or payout routing.
//! RO:TEST — tests/paid_write_economics.rs, tests/paid_write_estimate.rs.

use std::{env, fs};

/// Optional path to the ROC economics TOML config.
pub const ENV_ROC_ECONOMICS_PATH: &str = "RON_STORAGE_ROC_ECONOMICS_PATH";

/// Optional action ID to price for the paid object route.
pub const ENV_ROC_ECONOMICS_ACTION: &str = "RON_STORAGE_ROC_ECONOMICS_ACTION";

/// Default paid storage action ID.
pub const DEFAULT_PAID_STORAGE_ACTION: &str = "paid_storage_put";

/// Default asset charged by the paid storage beta path.
pub const DEFAULT_PAID_STORAGE_ASSET: &str = "roc";

/// Legacy beta pricing label used when no ROC economics config is explicitly set.
pub const PRICING_MODE_LEGACY: &str = "legacy";

/// ROC economics pricing label used when `RON_STORAGE_ROC_ECONOMICS_PATH` is set.
pub const PRICING_MODE_ROC_ECONOMICS: &str = "roc-economics";

/// Complete paid-storage price estimate.
///
/// This is intentionally side-effect free. It does not call wallet, ledger,
/// accounting, naming, index, or manifest services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaidStoragePriceEstimate {
    /// Bytes the caller plans to store.
    pub bytes_stored: u64,
    /// Economics action ID used for pricing.
    pub action_id: String,
    /// Internal unit/asset used by this beta paid-storage path.
    pub asset: String,
    /// Minimum ROC minor units that must be available for capture.
    pub amount_minor: u128,
    /// Pricing source/mode.
    pub pricing_mode: &'static str,
    /// Explicit economics policy path, when configured.
    pub economics_policy_path: Option<String>,
}

/// Current legacy beta pricing: `max(bytes_stored, 1)`.
#[must_use]
pub fn legacy_paid_storage_capture_amount(bytes_stored: u64) -> u128 {
    u128::from(bytes_stored).max(1)
}

/// Return the capture amount for paid storage.
///
/// If `RON_STORAGE_ROC_ECONOMICS_PATH` is unset, this deliberately preserves the current
/// beta behavior, `max(bytes_stored, 1)`. If the env var is set, the policy must load and
/// validate through `ron-policy`, otherwise the paid write fails closed.
///
/// # Errors
///
/// Returns a human-safe config error string when the explicit economics policy cannot be
/// read, parsed, validated, or used to price the configured action.
pub fn paid_storage_capture_amount_from_env(bytes_stored: u64) -> Result<u128, String> {
    Ok(paid_storage_price_estimate_from_env(bytes_stored)?.amount_minor)
}

/// Return a complete side-effect-free paid storage estimate from the current env config.
///
/// This is shared by `/paid/o` settlement planning and `/paid/o/estimate` preflight
/// so product UX cannot drift from live capture pricing.
///
/// # Errors
///
/// Returns a human-safe config error string when the explicit economics policy cannot be
/// read, parsed, validated, or used to price the configured action.
pub fn paid_storage_price_estimate_from_env(
    bytes_stored: u64,
) -> Result<PaidStoragePriceEstimate, String> {
    let action_id = economics_action_from_env();
    let policy_path = economics_policy_path_from_env()?;

    let (amount_minor, pricing_mode, economics_policy_path) = match policy_path {
        Some(policy_path) => {
            let bytes = fs::read(&policy_path).map_err(|err| {
                format!("failed to read {ENV_ROC_ECONOMICS_PATH}={policy_path}: {err}")
            })?;

            let policy = ron_policy::load_economics_toml(&bytes).map_err(|err| {
                format!("failed to load ROC economics policy {policy_path}: {err}")
            })?;

            let amount_minor = policy
                .price_for(&action_id, u128::from(bytes_stored))
                .map_err(|err| format!("failed to price economics action {action_id}: {err}"))?;

            (amount_minor, PRICING_MODE_ROC_ECONOMICS, Some(policy_path))
        }
        None => (
            legacy_paid_storage_capture_amount(bytes_stored),
            PRICING_MODE_LEGACY,
            None,
        ),
    };

    Ok(PaidStoragePriceEstimate {
        bytes_stored,
        action_id,
        asset: DEFAULT_PAID_STORAGE_ASSET.to_string(),
        amount_minor,
        pricing_mode,
        economics_policy_path,
    })
}

fn economics_policy_path_from_env() -> Result<Option<String>, String> {
    match env::var(ENV_ROC_ECONOMICS_PATH) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(format!("{ENV_ROC_ECONOMICS_PATH} cannot be empty"));
            }
            Ok(Some(trimmed.to_string()))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(format!("failed to read {ENV_ROC_ECONOMICS_PATH}: {err}")),
    }
}

fn economics_action_from_env() -> String {
    env::var(ENV_ROC_ECONOMICS_ACTION)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PAID_STORAGE_ACTION.to_string())
}
