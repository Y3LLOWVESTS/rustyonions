//! RO:WHAT — Local policy admission control for wallet operations.
//! RO:WHY  — Pillar 12; Concerns: SEC/ECON/GOV. Provides a safe deny/allow seam before ron-policy integration.
//! RO:INTERACTS — auth::caps::CapabilityClaims, dto::requests, config.
//! RO:INVARIANTS — amount ceilings checked; asset/account caveats honored; no ledger IO occurs here.
//! RO:METRICS — caller increments wallet_rejects_total{reason="FORBIDDEN"|"LIMITS_EXCEEDED"}.
//! RO:CONFIG — WalletConfig amount ceilings and asset.
//! RO:SECURITY — enforces capability caveats; no token strings are inspected.
//! RO:TEST — caveat_denies_wrong_asset.

use crate::{
    auth::caps::CapabilityClaims,
    config::WalletConfig,
    dto::requests::AmountMinor,
    errors::{WalletError, WalletResult},
};

/// Policy action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    /// Read path.
    Read,
    /// Issue/mint.
    Issue,
    /// Transfer.
    Transfer,
    /// Burn.
    Burn,
}

/// Request context passed to policy enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyContext<'a> {
    /// Action being attempted.
    pub action: PolicyAction,
    /// Asset id.
    pub asset: &'a str,
    /// Debit account where applicable.
    pub from: Option<&'a str>,
    /// Credit account where applicable.
    pub to: Option<&'a str>,
    /// Amount in minor units where applicable.
    pub amount: Option<AmountMinor>,
}

/// Enforce local policy and capability caveats.
pub fn enforce_local_policy(
    cfg: &WalletConfig,
    claims: &CapabilityClaims,
    ctx: &PolicyContext<'_>,
) -> WalletResult<()> {
    if ctx.asset != cfg.asset {
        return Err(WalletError::forbidden(
            "asset is not allowed by wallet config",
        ));
    }

    if let Some(amount) = ctx.amount {
        if amount.get() > cfg.max_amount_per_op {
            return Err(WalletError::limits_exceeded(
                "amount exceeds max_amount_per_op",
            ));
        }
    }

    if !claims.assets.is_empty() && !claims.assets.iter().any(|asset| asset == ctx.asset) {
        return Err(WalletError::forbidden("asset caveat denied operation"));
    }

    if !claims.accounts.is_empty() {
        for account in [ctx.from, ctx.to].into_iter().flatten() {
            if !claims.accounts.iter().any(|allowed| allowed == account) {
                return Err(WalletError::forbidden("account caveat denied operation"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::caps::WalletScope;

    #[test]
    fn caveat_denies_wrong_asset() {
        let cfg = WalletConfig::default();
        let claims = CapabilityClaims {
            subject: "sub".into(),
            scopes: vec![WalletScope::Transfer],
            accounts: Vec::new(),
            assets: vec!["other".into()],
        };
        let ctx = PolicyContext {
            action: PolicyAction::Transfer,
            asset: "roc",
            from: Some("a"),
            to: Some("b"),
            amount: Some(AmountMinor(1)),
        };
        assert!(enforce_local_policy(&cfg, &claims, &ctx).is_err());
    }
}
