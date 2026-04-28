//! RO:WHAT — Minimal capability scope model and verifier seam for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: SEC/ECON. Keeps authorization explicit before policy and ledger work.
//! RO:INTERACTS — routes/v1, policy::enforce, future ron-auth verifier adapter.
//! RO:INVARIANTS — missing scope is forbidden; no ambient admin bypass; token value is not logged or stored.
//! RO:METRICS — caller increments wallet_rejects_total{reason="FORBIDDEN"}.
//! RO:CONFIG — future verifier config plugs in here.
//! RO:SECURITY — bearer tokens are consumed but never formatted in errors.
//! RO:TEST — require_scope_accepts_and_denies.

use crate::errors::{WalletError, WalletResult};

/// Wallet capability scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WalletScope {
    /// Read balances and receipts.
    Read,
    /// Issue/mint ROC.
    Issue,
    /// Transfer ROC.
    Transfer,
    /// Burn ROC.
    Burn,
}

impl WalletScope {
    /// Stable lowercase label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Issue => "issue",
            Self::Transfer => "transfer",
            Self::Burn => "burn",
        }
    }
}

/// Claims extracted from a verified capability token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityClaims {
    /// Subject identifier.
    pub subject: String,
    /// Allowed scopes.
    pub scopes: Vec<WalletScope>,
    /// Optional account caveats.
    pub accounts: Vec<String>,
    /// Optional asset caveats.
    pub assets: Vec<String>,
}

impl CapabilityClaims {
    /// Require a scope.
    pub fn require_scope(&self, scope: WalletScope) -> WalletResult<()> {
        if self.scopes.contains(&scope) {
            Ok(())
        } else {
            Err(WalletError::forbidden(format!(
                "capability missing required scope {}",
                scope.as_str()
            )))
        }
    }
}

/// Verification seam for future ron-auth integration.
pub trait CapabilityVerifier: Send + Sync + 'static {
    /// Verify a bearer token and return claims.
    fn verify(&self, bearer_token: &str) -> WalletResult<CapabilityClaims>;
}

/// Test/dev verifier that grants explicit configured scopes.
#[derive(Debug, Clone)]
pub struct StaticCapabilityVerifier {
    claims: CapabilityClaims,
}

impl StaticCapabilityVerifier {
    /// Build a static verifier.
    pub fn new(claims: CapabilityClaims) -> Self {
        Self { claims }
    }
}

impl CapabilityVerifier for StaticCapabilityVerifier {
    fn verify(&self, bearer_token: &str) -> WalletResult<CapabilityClaims> {
        if bearer_token.trim().is_empty() {
            return Err(WalletError::new(
                crate::errors::WalletErrorCode::Unauthorized,
                "missing bearer token",
            ));
        }
        Ok(self.claims.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_scope_accepts_and_denies() {
        let claims = CapabilityClaims {
            subject: "sub".into(),
            scopes: vec![WalletScope::Read],
            accounts: Vec::new(),
            assets: Vec::new(),
        };
        claims.require_scope(WalletScope::Read).unwrap();
        assert!(claims.require_scope(WalletScope::Transfer).is_err());
    }
}
