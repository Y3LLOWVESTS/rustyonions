//! RO:WHAT — Capability and local policy invariant tests.
//! RO:WHY  — Pillar 12; Concerns: SEC/ECON/GOV. Money operations must be capability-gated.
//! RO:INTERACTS — auth::caps and policy::enforce.
//! RO:INVARIANTS — missing scope denies; asset/account caveats deny before ledger IO.
//! RO:METRICS — future wallet_rejects_total{reason="FORBIDDEN"}.
//! RO:CONFIG — WalletConfig asset and amount ceilings.
//! RO:SECURITY — bearer token strings are not stored or logged.
//! RO:TEST — cargo test -p svc-wallet --test i_4_caps_policy.

use svc_wallet::{
    auth::caps::{CapabilityClaims, CapabilityVerifier, StaticCapabilityVerifier, WalletScope},
    config::WalletConfig,
    dto::requests::AmountMinor,
    errors::WalletErrorCode,
    policy::enforce::{enforce_local_policy, PolicyAction, PolicyContext},
};

fn claims(scopes: Vec<WalletScope>) -> CapabilityClaims {
    CapabilityClaims {
        subject: "tester".to_string(),
        scopes,
        accounts: Vec::new(),
        assets: Vec::new(),
    }
}

#[test]
fn missing_required_scope_is_forbidden() {
    let claims = claims(vec![WalletScope::Read]);

    let err = claims
        .require_scope(WalletScope::Transfer)
        .expect_err("missing transfer scope must reject");

    assert_eq!(err.code, WalletErrorCode::Forbidden);
}

#[test]
fn verifier_rejects_empty_bearer_token() {
    let verifier = StaticCapabilityVerifier::new(claims(vec![WalletScope::Transfer]));

    let err = verifier
        .verify("")
        .expect_err("empty bearer token must reject");

    assert_eq!(err.code, WalletErrorCode::Unauthorized);
}

#[test]
fn account_caveat_denies_unlisted_counterparty() {
    let cfg = WalletConfig::default();
    let claims = CapabilityClaims {
        subject: "tester".to_string(),
        scopes: vec![WalletScope::Transfer],
        accounts: vec!["acct_a".to_string()],
        assets: vec!["roc".to_string()],
    };
    let ctx = PolicyContext {
        action: PolicyAction::Transfer,
        asset: "roc",
        from: Some("acct_a"),
        to: Some("acct_b"),
        amount: Some(AmountMinor(1)),
    };

    let err =
        enforce_local_policy(&cfg, &claims, &ctx).expect_err("account caveat must deny acct_b");

    assert_eq!(err.code, WalletErrorCode::Forbidden);
}

#[test]
fn amount_over_policy_ceiling_is_limits_exceeded() {
    let cfg = WalletConfig {
        max_amount_per_op: 10,
        ..WalletConfig::default()
    };
    let claims = claims(vec![WalletScope::Issue]);
    let ctx = PolicyContext {
        action: PolicyAction::Issue,
        asset: "roc",
        from: None,
        to: Some("acct_a"),
        amount: Some(AmountMinor(11)),
    };

    let err =
        enforce_local_policy(&cfg, &claims, &ctx).expect_err("amount above ceiling must reject");

    assert_eq!(err.code, WalletErrorCode::LimitsExceeded);
}
