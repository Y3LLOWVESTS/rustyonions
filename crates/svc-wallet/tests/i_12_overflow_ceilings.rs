//! RO:WHAT — Overflow and amount-ceiling tests for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC. Amount math must stay integer-only and bounded.
//! RO:INTERACTS — dto::requests::AmountMinor and WalletConfig.
//! RO:INVARIANTS — no zero amount; max_amount_per_op enforced; adapter rejects values above current ledger u64 ceiling.
//! RO:METRICS — future wallet_rejects_total{reason="LIMITS_EXCEEDED"}.
//! RO:CONFIG — max_amount_per_op and max_account_total.
//! RO:SECURITY — prevents arithmetic abuse before ledger IO.
//! RO:TEST — cargo test -p svc-wallet --test i_12_overflow_ceilings.

use svc_wallet::{
    config::WalletConfig,
    dto::requests::{validate_amount, AmountMinor},
    errors::WalletErrorCode,
};

#[test]
fn amount_zero_is_rejected_at_constructor() {
    let err = AmountMinor::new(0).expect_err("zero amount must reject");

    assert_eq!(err.code, WalletErrorCode::BadRequest);
}

#[test]
fn amount_over_config_ceiling_is_rejected() {
    let cfg = WalletConfig {
        max_amount_per_op: 100,
        ..WalletConfig::default()
    };

    let err = validate_amount(AmountMinor(101), &cfg)
        .expect_err("amount above max_amount_per_op must reject");

    assert_eq!(err.code, WalletErrorCode::LimitsExceeded);
}

#[test]
fn ledger_adapter_ceiling_rejects_amounts_above_u64() {
    let amount = AmountMinor(u64::MAX as u128 + 1);

    let err = amount
        .try_as_u64_for_ledger()
        .expect_err("current ron-ledger adapter is u64-backed");

    assert_eq!(err.code, WalletErrorCode::LimitsExceeded);
}
