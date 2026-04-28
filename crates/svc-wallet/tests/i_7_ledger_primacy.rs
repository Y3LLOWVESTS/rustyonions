//! RO:WHAT — Ledger primacy tests for wallet balance truth.
//! RO:WHY  — Pillar 12; Concerns: ECON/GOV/RES. Wallet must not create an alternate balance truth.
//! RO:INTERACTS — ledger::client and ron-ledger through the adapter.
//! RO:INVARIANTS — balance reads reflect committed ledger entries; rejected commits do not mutate visible balances.
//! RO:METRICS — future upstream_calls_total{svc="ledger"}.
//! RO:CONFIG — default in-memory/amnesia ledger.
//! RO:SECURITY — KID/cap refs are adapter identifiers only.
//! RO:TEST — cargo test -p svc-wallet --test i_7_ledger_primacy.

mod harness;

use svc_wallet::errors::WalletErrorCode;

#[test]
fn balance_reads_follow_only_successful_ledger_commits() {
    let cfg = harness::cfg();
    let client = harness::client();

    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 0);

    harness::issue_to(&client, &cfg, "acct_a", 100, "idem_issue_primacy");
    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 100);

    client
        .transfer(
            &cfg,
            &harness::transfer_req("acct_a", "acct_b", 40, 1),
            "idem_transfer_primacy",
        )
        .expect("valid transfer should commit");

    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 60);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_b"), 40);
}

#[test]
fn rejected_ledger_commit_does_not_apply_partial_credit() {
    let cfg = harness::cfg();
    let client = harness::client();

    let err = client
        .transfer(
            &cfg,
            &harness::transfer_req("acct_a", "acct_b", 999, 1),
            "idem_rejected_primacy",
        )
        .expect_err("unfunded transfer must reject");

    assert_eq!(err.code, WalletErrorCode::InsufficientFunds);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 0);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_b"), 0);
}
