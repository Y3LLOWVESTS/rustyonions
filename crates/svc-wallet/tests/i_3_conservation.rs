//! RO:WHAT — Conservation invariant tests for issue, transfer, and burn.
//! RO:WHY  — Pillar 12; Concerns: ECON/GOV. Transfers conserve value; issue and burn are explicit supply changes.
//! RO:INTERACTS — ledger::client and wallet receipt mapping.
//! RO:INVARIANTS — transfer debits equal credits; burn lowers supply; receipts include ledger sequence spans.
//! RO:METRICS — future wallet_conservation_checks_total.
//! RO:CONFIG — default ROC asset.
//! RO:SECURITY — no auth material.
//! RO:TEST — cargo test -p svc-wallet --test i_3_conservation.

mod harness;

use svc_wallet::dto::responses::WalletOp;

#[test]
fn transfer_conserves_sum_of_participant_balances() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(&client, &cfg, "acct_a", 100, "idem_issue_cons");

    let before_total =
        harness::balance_of(&client, &cfg, "acct_a") + harness::balance_of(&client, &cfg, "acct_b");

    let receipt = client
        .transfer(
            &cfg,
            &harness::transfer_req("acct_a", "acct_b", 40, 1),
            "idem_transfer_cons",
        )
        .expect("transfer should commit");

    let after_total =
        harness::balance_of(&client, &cfg, "acct_a") + harness::balance_of(&client, &cfg, "acct_b");

    assert_eq!(before_total, 100);
    assert_eq!(after_total, before_total);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 60);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_b"), 40);
    assert_eq!(receipt.op, WalletOp::Transfer);
    assert_eq!(receipt.ledger_seq_start, Some(2));
    assert_eq!(receipt.ledger_seq_end, Some(3));
}

#[test]
fn burn_explicitly_reduces_supply() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(&client, &cfg, "acct_a", 100, "idem_issue_burn_cons");

    let receipt = client
        .burn(&cfg, &harness::burn_req("acct_a", 25, 1), "idem_burn_cons")
        .expect("burn should commit");

    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 75);
    assert_eq!(receipt.op, WalletOp::Burn);
    assert_eq!(receipt.amount_minor.get(), 25);
}
