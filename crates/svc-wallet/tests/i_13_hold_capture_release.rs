//! RO:WHAT — Escrow invariant tests for wallet hold/capture/release primitives.
//! RO:WHY  — Pillar 12; Concerns: ECON/GOV/RES. Paid storage must reserve, capture, and release through ledger truth.
//! RO:INTERACTS — wallet ledger adapter, TransferRequest-compatible escrow moves, ron-ledger balances.
//! RO:INVARIANTS — hold debits payer; capture pays service; release refunds remainder; total value conserved.
//! RO:METRICS — none; route-level metrics arrive in the next HTTP wrapper batch.
//! RO:CONFIG — uses WalletConfig::default with asset=roc and amnesia=true.
//! RO:SECURITY — no auth material; route/capability checks arrive in HTTP batch.
//! RO:TEST — cargo test -p svc-wallet --test i_13_hold_capture_release.

mod harness;

use svc_wallet::{dto::responses::WalletOp, errors::WalletErrorCode};

#[test]
fn hold_capture_release_preserves_total_value() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(&client, &cfg, "acct_user", 100, "idem_issue_paid_storage");

    let hold = client
        .hold(
            &cfg,
            &harness::transfer_req("acct_user", "escrow_hold_1", 70, 1),
            "idem_hold_paid_storage",
        )
        .expect("hold should reserve funds");

    assert_eq!(hold.op, WalletOp::Hold);
    assert_eq!(hold.from.as_deref(), Some("acct_user"));
    assert_eq!(hold.to.as_deref(), Some("escrow_hold_1"));
    assert_eq!(hold.amount_minor.get(), 70);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_user"), 30);
    assert_eq!(harness::balance_of(&client, &cfg, "escrow_hold_1"), 70);

    let capture = client
        .capture(
            &cfg,
            &harness::transfer_req("escrow_hold_1", "svc_storage", 40, 1),
            "idem_capture_paid_storage",
        )
        .expect("capture should pay storage service");

    assert_eq!(capture.op, WalletOp::Capture);
    assert_eq!(capture.from.as_deref(), Some("escrow_hold_1"));
    assert_eq!(capture.to.as_deref(), Some("svc_storage"));
    assert_eq!(capture.amount_minor.get(), 40);
    assert_eq!(harness::balance_of(&client, &cfg, "escrow_hold_1"), 30);
    assert_eq!(harness::balance_of(&client, &cfg, "svc_storage"), 40);

    let release = client
        .release(
            &cfg,
            &harness::transfer_req("escrow_hold_1", "acct_user", 30, 2),
            "idem_release_paid_storage",
        )
        .expect("release should return remaining escrow");

    assert_eq!(release.op, WalletOp::Release);
    assert_eq!(release.from.as_deref(), Some("escrow_hold_1"));
    assert_eq!(release.to.as_deref(), Some("acct_user"));
    assert_eq!(release.amount_minor.get(), 30);
    assert_eq!(harness::balance_of(&client, &cfg, "escrow_hold_1"), 0);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_user"), 60);

    let final_total = harness::balance_of(&client, &cfg, "acct_user")
        + harness::balance_of(&client, &cfg, "escrow_hold_1")
        + harness::balance_of(&client, &cfg, "svc_storage");

    assert_eq!(final_total, 100);
}

#[test]
fn capture_more_than_held_rejects_without_crediting_payee() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(&client, &cfg, "acct_user", 50, "idem_issue_capture_reject");

    client
        .hold(
            &cfg,
            &harness::transfer_req("acct_user", "escrow_hold_2", 20, 1),
            "idem_hold_capture_reject",
        )
        .expect("hold should reserve funds");

    let err = client
        .capture(
            &cfg,
            &harness::transfer_req("escrow_hold_2", "svc_storage", 21, 1),
            "idem_capture_too_much",
        )
        .expect_err("capture above held amount must reject");

    assert_eq!(err.code, WalletErrorCode::InsufficientFunds);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_user"), 30);
    assert_eq!(harness::balance_of(&client, &cfg, "escrow_hold_2"), 20);
    assert_eq!(harness::balance_of(&client, &cfg, "svc_storage"), 0);
}
