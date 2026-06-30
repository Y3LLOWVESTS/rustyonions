//! RO:WHAT — Internal ROC Beta paid content receipt-path tests for svc-wallet.
//! RO:WHY — Internal ROC Beta Phase 1; Concerns: ECON/SEC/RES/GOV. Paid post/comment/article/content_view must use existing wallet/ledger receipt path.
//! RO:INTERACTS — LocalLedgerClient, wallet receipt DTOs, transfer/hold/capture request DTOs, ron-ledger memory storage.
//! RO:INVARIANTS — svc-wallet remains mutation front-door; no fake receipt/finality; no bridge/staking/liquidity/external settlement fields.
//! RO:METRICS — none; route metrics stay covered by existing HTTP tests.
//! RO:CONFIG — WalletConfig::default, asset=roc, amnesia=true.
//! RO:SECURITY — no bearer tokens or private keys; this is local economic path proof only.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_beta_paid_content_receipt_path.

mod harness;

use serde_json::json;
use svc_wallet::{
    dto::{
        requests::TransferRequest,
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    errors::WalletErrorCode,
};

const VIEWER: &str = "acct_internal_roc_paid_viewer";
const CREATOR: &str = "acct_internal_roc_paid_creator";
const UNFUNDED_VIEWER: &str = "acct_internal_roc_unfunded_viewer";

#[derive(Debug, Clone, Copy)]
struct PaidContentCase {
    label: &'static str,
    amount_minor: u128,
    viewer_nonce: u64,
}

const PAID_CONTENT_CASES: &[PaidContentCase] = &[
    PaidContentCase {
        label: "paid_post",
        amount_minor: 10,
        viewer_nonce: 1,
    },
    PaidContentCase {
        label: "paid_comment",
        amount_minor: 15,
        viewer_nonce: 2,
    },
    PaidContentCase {
        label: "paid_article",
        amount_minor: 20,
        viewer_nonce: 3,
    },
    PaidContentCase {
        label: "content_view",
        amount_minor: 25,
        viewer_nonce: 4,
    },
];

#[test]
fn paid_content_hold_capture_receipts_cover_beta_action_labels() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        VIEWER,
        1_000,
        "internal_roc_paid_content_seed_viewer",
    );

    let mut total_paid = 0_u128;

    for case in PAID_CONTENT_CASES {
        let escrow = escrow_account(case.label);
        let hold_idem = format!("internal_roc_{}_hold", case.label);
        let capture_idem = format!("internal_roc_{}_capture", case.label);

        let hold = client
            .hold(
                &cfg,
                &harness::transfer_req(VIEWER, &escrow, case.amount_minor, case.viewer_nonce),
                &hold_idem,
            )
            .expect("paid content hold should commit through wallet/ledger path");

        assert_wallet_receipt(
            &hold,
            WalletOp::Hold,
            Some(VIEWER),
            Some(escrow.as_str()),
            case.amount_minor,
            &hold_idem,
        );

        assert_eq!(
            harness::balance_of(&client, &cfg, &escrow),
            case.amount_minor,
            "hold should reserve value in escrow account for {}",
            case.label
        );

        let capture = client
            .capture(
                &cfg,
                &harness::transfer_req(&escrow, CREATOR, case.amount_minor, 1),
                &capture_idem,
            )
            .expect("paid content capture should commit through wallet/ledger path");

        assert_wallet_receipt(
            &capture,
            WalletOp::Capture,
            Some(escrow.as_str()),
            Some(CREATOR),
            case.amount_minor,
            &capture_idem,
        );

        assert_eq!(
            harness::balance_of(&client, &cfg, &escrow),
            0,
            "capture should drain escrow account for {}",
            case.label
        );

        assert_receipt_does_not_claim_future_finality_or_external_settlement(&hold);
        assert_receipt_does_not_claim_future_finality_or_external_settlement(&capture);

        total_paid += case.amount_minor;
    }

    assert_eq!(total_paid, 70);
    assert_eq!(harness::balance_of(&client, &cfg, VIEWER), 930);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR), 70);
}

#[test]
fn unfunded_paid_content_hold_rejects_without_creator_credit_or_fake_receipt() {
    let cfg = harness::cfg();
    let client = harness::client();

    let creator_before = harness::balance_of(&client, &cfg, CREATOR);
    let escrow = escrow_account("paid_post");
    let escrow_before = harness::balance_of(&client, &cfg, &escrow);

    let err = client
        .hold(
            &cfg,
            &harness::transfer_req(UNFUNDED_VIEWER, &escrow, 10, 1),
            "internal_roc_paid_post_unfunded_hold",
        )
        .expect_err("unfunded paid content hold must reject");

    assert_eq!(err.code, WalletErrorCode::InsufficientFunds);
    assert_eq!(
        harness::balance_of(&client, &cfg, CREATOR),
        creator_before,
        "creator must not be credited after rejected paid content hold"
    );
    assert_eq!(
        harness::balance_of(&client, &cfg, &escrow),
        escrow_before,
        "escrow must not be credited after rejected paid content hold"
    );
}

#[test]
fn paid_content_wallet_request_dtos_reject_authority_poison_fields() {
    for forbidden_field in [
        "gateway_receipt_truth",
        "omnigate_receipt_truth",
        "accounting_balance_truth",
        "rewarder_payout_truth",
        "client_finality_claim",
        "cache_unlock_authority",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
    ] {
        let mut value = json!({
            "from": VIEWER,
            "to": CREATOR,
            "asset": "roc",
            "amount_minor": "10",
            "nonce": 1
        });

        value
            .as_object_mut()
            .expect("test request object")
            .insert(forbidden_field.to_string(), json!(true));

        assert!(
            serde_json::from_value::<TransferRequest>(value).is_err(),
            "TransferRequest must reject forbidden paid-content authority field `{forbidden_field}`"
        );
    }
}

fn escrow_account(label: &str) -> String {
    format!("escrow_internal_roc_{label}")
}

fn assert_wallet_receipt(
    receipt: &Receipt,
    expected_op: WalletOp,
    expected_from: Option<&str>,
    expected_to: Option<&str>,
    expected_amount_minor: u128,
    expected_idem: &str,
) {
    assert_eq!(receipt.op, expected_op);
    assert_eq!(receipt.from.as_deref(), expected_from);
    assert_eq!(receipt.to.as_deref(), expected_to);
    assert_eq!(receipt.asset, "roc");
    assert_eq!(receipt.amount_minor.get(), expected_amount_minor);
    assert_eq!(receipt.idem, expected_idem);
    assert_eq!(
        receipt.settlement_status,
        ReceiptSettlementStatus::Accepted,
        "svc-wallet may claim accepted wallet/ledger receipt status, not future finality"
    );

    assert!(
        receipt.txid.starts_with("tx_"),
        "wallet txid should be stable structured backend receipt proof, got {}",
        receipt.txid
    );

    assert_b3_hash(&receipt.receipt_hash);
    assert_ledger_root(&receipt.ledger_root);

    let start = receipt
        .ledger_seq_start
        .expect("wallet receipt should include ledger start sequence");
    let end = receipt
        .ledger_seq_end
        .expect("wallet receipt should include ledger end sequence");

    assert!(
        start <= end,
        "ledger sequence bounds must be monotonic: start={start}, end={end}"
    );

    let encoded = serde_json::to_string(receipt).expect("receipt serializes");
    assert!(
        encoded.contains(&format!(r#""amount_minor":"{}""#, expected_amount_minor)),
        "amount_minor must serialize as integer minor-unit string"
    );
    assert!(
        !encoded.contains(&format!(r#""amount_minor":{}"#, expected_amount_minor)),
        "amount_minor must not serialize as numeric JSON"
    );
}

fn assert_receipt_does_not_claim_future_finality_or_external_settlement(receipt: &Receipt) {
    let value = serde_json::to_value(receipt).expect("receipt JSON");
    let object = value.as_object().expect("receipt object");

    assert_eq!(
        object
            .get("settlement_status")
            .and_then(|value| value.as_str()),
        Some("accepted"),
        "wallet receipt status must stay accepted-only"
    );

    for forbidden in [
        "prepared",
        "quoted",
        "epoch_included",
        "finalized",
        "anchored",
        "committee_ready",
        "quorum_ready",
        "gateway_receipt_truth",
        "omnigate_receipt_truth",
        "accounting_balance_truth",
        "rewarder_payout_truth",
        "client_finality_claim",
        "cache_unlock_authority",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "svc-wallet receipt must not expose forbidden authority/finality field `{forbidden}`"
        );
    }
}

fn assert_b3_hash(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected b3:<64 lowercase hex>, got {value}"
    );

    let hex = &value[3..];
    assert_eq!(hex.len(), 64, "expected 64 hex chars, got {value}");
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase hex receipt hash, got {value}"
    );
}

fn assert_ledger_root(value: &str) {
    assert_eq!(
        value.len(),
        64,
        "ledger_root should be 64 lowercase hex chars"
    );
    assert!(
        value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "ledger_root should be lowercase hex, got {value}"
    );
}
