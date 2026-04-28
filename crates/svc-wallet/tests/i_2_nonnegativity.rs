//! RO:WHAT — Non-negativity invariant tests for debit-side wallet operations.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC. Wallet must never allow account balances below zero.
//! RO:INTERACTS — ledger::client, errors.
//! RO:INVARIANTS — overdraft transfer/burn rejects with INSUFFICIENT_FUNDS and leaves balances unchanged.
//! RO:METRICS — future wallet_rejects_total{reason="INSUFFICIENT_FUNDS"}.
//! RO:CONFIG — default asset and amount bounds.
//! RO:SECURITY — no auth material.
//! RO:TEST — cargo test -p svc-wallet --test i_2_nonnegativity.

mod harness;

use svc_wallet::errors::WalletErrorCode;

#[test]
fn transfer_from_empty_account_rejects_without_crediting_receiver() {
    let cfg = harness::cfg();
    let client = harness::client();

    let err = client
        .transfer(
            &cfg,
            &harness::transfer_req("acct_empty", "acct_b", 1, 1),
            "idem_overdraft_transfer",
        )
        .expect_err("overdraft transfer must reject");

    assert_eq!(err.code, WalletErrorCode::InsufficientFunds);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_empty"), 0);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_b"), 0);
}

#[test]
fn burn_more_than_balance_rejects_without_state_change() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(&client, &cfg, "acct_a", 10, "idem_issue_nonneg");

    let err = client
        .burn(
            &cfg,
            &harness::burn_req("acct_a", 11, 1),
            "idem_burn_too_much",
        )
        .expect_err("burn over balance must reject");

    assert_eq!(err.code, WalletErrorCode::InsufficientFunds);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 10);
}
