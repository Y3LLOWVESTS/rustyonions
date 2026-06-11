//! RO:WHAT — Tests strict QuickChain receipt wire, status, and operation-shape contracts.
//! RO:WHY — ECON/RES: receipt fields must be unambiguous before canonical receipt hashes exist.
//! RO:INTERACTS — QuickChainReceiptV1, operation classes, hold IDs, future vector corpus.
//! RO:INVARIANTS — backend-derived data only; exact account/hold matrix; no hashes or ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — validation is structural and never establishes receipt authority.
//! RO:TEST — this file is the receipt DTO and operation-shape matrix gate.

use ron_proto::{
    ContentId, QuickChainOperationClassV1, QuickChainReceiptStatusV1, QuickChainReceiptV1,
    QuickChainValidationError, QUICKCHAIN_DTO_VERSION, QUICKCHAIN_RECEIPT_SCHEMA,
};
use serde_json::json;

const HOLD_ID: &str = "hold_0123456789abcdef0123456789abcdef";

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn valid_receipt(status: QuickChainReceiptStatusV1) -> QuickChainReceiptV1 {
    QuickChainReceiptV1 {
        schema: QUICKCHAIN_RECEIPT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        txid: "tx:roc:000000000001".to_string(),
        operation_id: "op_0123456789abcdef0123456789abcdef".to_string(),
        op: "paid_site_visit".to_string(),
        op_class: QuickChainOperationClassV1::Transfer,
        status,
        from_account_id: Some("account:visitor-b".to_string()),
        to_account_id: Some("account:creator-a".to_string()),
        asset: "roc".to_string(),
        amount_minor: "10".to_string(),
        account_sequence: Some(7),
        hold_id: None,
        session_budget_id: None,
        idempotency_key: "visit-2026-06-10T18:40:00Z-0001".to_string(),
        operation_hash: None,
        receipt_hash: None,
        receipt_root: None,
        checkpoint_hash: None,
        ledger_seq_start: None,
        ledger_seq_end: None,
        previous_ledger_root: None,
        new_ledger_root: None,
        memo: Some("backend-derived receipt reference".to_string()),
        produced_at_ms: 1_800_000_000_000,
    }
}

fn receipt_for_class(op_class: QuickChainOperationClassV1) -> QuickChainReceiptV1 {
    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);
    receipt.op_class = op_class;

    match op_class {
        QuickChainOperationClassV1::Issue => {
            receipt.op = "issue".to_string();
            receipt.from_account_id = None;
            receipt.to_account_id = Some("account:recipient-a".to_string());
            receipt.hold_id = None;
        }

        QuickChainOperationClassV1::Transfer => {
            receipt.op = "transfer".to_string();
            receipt.from_account_id = Some("account:sender-a".to_string());
            receipt.to_account_id = Some("account:recipient-a".to_string());
            receipt.hold_id = None;
        }

        QuickChainOperationClassV1::Burn => {
            receipt.op = "burn".to_string();
            receipt.from_account_id = Some("account:holder-a".to_string());
            receipt.to_account_id = None;
            receipt.hold_id = None;
        }

        QuickChainOperationClassV1::HoldOpen => {
            receipt.op = "hold_open".to_string();
            receipt.from_account_id = Some("account:viewer-a".to_string());
            receipt.to_account_id = Some("account:creator-b".to_string());
            receipt.hold_id = Some(HOLD_ID.to_string());
        }

        QuickChainOperationClassV1::HoldCapture => {
            receipt.op = "hold_capture".to_string();
            receipt.from_account_id = Some("account:viewer-a".to_string());
            receipt.to_account_id = Some("account:creator-b".to_string());
            receipt.hold_id = Some(HOLD_ID.to_string());
        }

        QuickChainOperationClassV1::HoldRelease => {
            receipt.op = "hold_release".to_string();
            receipt.from_account_id = Some("account:viewer-a".to_string());
            receipt.to_account_id = None;
            receipt.hold_id = Some(HOLD_ID.to_string());
        }

        QuickChainOperationClassV1::HoldExpire => {
            receipt.op = "hold_expire".to_string();
            receipt.from_account_id = Some("account:viewer-a".to_string());
            receipt.to_account_id = None;
            receipt.hold_id = Some(HOLD_ID.to_string());
        }

        _ => panic!("unsupported future QuickChain operation class in test fixture"),
    }

    receipt
}

fn assert_invalid_field(receipt: QuickChainReceiptV1, expected_field: &'static str) {
    let error = receipt.validate().unwrap_err();

    match error {
        QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, expected_field);
        }
        other => panic!("expected InvalidField for {expected_field}, got {other:?}"),
    }
}

#[test]
fn accepted_receipt_validates_and_roundtrips_without_epoch_evidence() {
    let receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);

    receipt.validate().unwrap();

    let json = serde_json::to_string(&receipt).unwrap();
    assert!(json.contains("\"schema\":\"quickchain.receipt.v1\""));
    assert!(json.contains("\"status\":\"accepted\""));
    assert!(json.contains("\"asset\":\"roc\""));

    let decoded: QuickChainReceiptV1 = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, receipt);
    decoded.validate().unwrap();
}

#[test]
fn receipt_rejects_unknown_fields_and_bad_wire_values() {
    let mut value =
        serde_json::to_value(valid_receipt(QuickChainReceiptStatusV1::Accepted)).unwrap();

    value
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_string(), json!(true));

    serde_json::from_value::<QuickChainReceiptV1>(value)
        .expect_err("unknown receipt fields must reject");

    serde_json::from_value::<QuickChainReceiptStatusV1>(json!("pending"))
        .expect_err("unknown receipt status must reject");

    let mut value =
        serde_json::to_value(valid_receipt(QuickChainReceiptStatusV1::Accepted)).unwrap();

    value["amount_minor"] = json!(10);

    serde_json::from_value::<QuickChainReceiptV1>(value)
        .expect_err("numeric money must reject before validation");
}

#[test]
fn receipt_status_helpers_are_monotonic_and_wire_names_are_stable() {
    assert!(QuickChainReceiptStatusV1::Accepted.is_backend_accepted());
    assert!(!QuickChainReceiptStatusV1::Accepted.is_epoch_included_or_stronger());
    assert!(QuickChainReceiptStatusV1::EpochIncluded.is_epoch_included_or_stronger());
    assert!(QuickChainReceiptStatusV1::Finalized.is_finalized_or_stronger());
    assert!(QuickChainReceiptStatusV1::Anchored.is_finalized_or_stronger());

    assert_eq!(
        serde_json::to_string(&QuickChainReceiptStatusV1::EpochIncluded).unwrap(),
        "\"epoch_included\""
    );
    assert_eq!(
        serde_json::to_string(&QuickChainReceiptStatusV1::Finalized).unwrap(),
        "\"finalized\""
    );
    assert_eq!(
        serde_json::to_string(&QuickChainReceiptStatusV1::Anchored).unwrap(),
        "\"anchored\""
    );
}

#[test]
fn epoch_included_receipt_requires_honest_epoch_evidence_fields() {
    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::EpochIncluded);

    receipt
        .validate()
        .expect_err("epoch-included receipts require receipt hash/root/checkpoint/ledger range");

    receipt.receipt_hash = Some(cid('a'));
    receipt.receipt_root = Some(cid('b'));
    receipt.checkpoint_hash = Some(cid('c'));
    receipt.ledger_seq_start = Some(100);
    receipt.ledger_seq_end = Some(101);

    receipt.validate().unwrap();

    let mut bad_range = receipt.clone();
    bad_range.ledger_seq_end = Some(99);
    assert_invalid_field(bad_range, "ledger_seq_range");

    let mut one_ledger_root = receipt;
    one_ledger_root.previous_ledger_root = Some(cid('d'));
    assert_invalid_field(one_ledger_root, "ledger_roots");
}

#[test]
fn every_operation_class_has_one_valid_phase0_receipt_shape() {
    for op_class in [
        QuickChainOperationClassV1::Issue,
        QuickChainOperationClassV1::Transfer,
        QuickChainOperationClassV1::Burn,
        QuickChainOperationClassV1::HoldOpen,
        QuickChainOperationClassV1::HoldCapture,
        QuickChainOperationClassV1::HoldRelease,
        QuickChainOperationClassV1::HoldExpire,
    ] {
        receipt_for_class(op_class).validate().unwrap();
    }

    let mut hold_open_without_counterparty =
        receipt_for_class(QuickChainOperationClassV1::HoldOpen);

    hold_open_without_counterparty.to_account_id = None;
    hold_open_without_counterparty.validate().unwrap();
}

#[test]
fn non_hold_receipts_reject_direction_conflicts_and_hold_ids() {
    let mut issue = receipt_for_class(QuickChainOperationClassV1::Issue);
    issue.from_account_id = Some("account:issuer-authority".to_string());
    assert_invalid_field(issue, "from_account_id");

    let mut issue = receipt_for_class(QuickChainOperationClassV1::Issue);
    issue.to_account_id = None;
    assert_invalid_field(issue, "to_account_id");

    let mut issue = receipt_for_class(QuickChainOperationClassV1::Issue);
    issue.hold_id = Some(HOLD_ID.to_string());
    assert_invalid_field(issue, "hold_id");

    let mut transfer = receipt_for_class(QuickChainOperationClassV1::Transfer);
    transfer.from_account_id = None;
    assert_invalid_field(transfer, "from_account_id");

    let mut transfer = receipt_for_class(QuickChainOperationClassV1::Transfer);
    transfer.to_account_id = None;
    assert_invalid_field(transfer, "to_account_id");

    let mut transfer = receipt_for_class(QuickChainOperationClassV1::Transfer);
    transfer.hold_id = Some(HOLD_ID.to_string());
    assert_invalid_field(transfer, "hold_id");

    let mut burn = receipt_for_class(QuickChainOperationClassV1::Burn);
    burn.from_account_id = None;
    assert_invalid_field(burn, "from_account_id");

    let mut burn = receipt_for_class(QuickChainOperationClassV1::Burn);
    burn.to_account_id = Some("account:unexpected-destination".to_string());
    assert_invalid_field(burn, "to_account_id");

    let mut burn = receipt_for_class(QuickChainOperationClassV1::Burn);
    burn.hold_id = Some(HOLD_ID.to_string());
    assert_invalid_field(burn, "hold_id");
}

#[test]
fn hold_receipts_require_lifecycle_accounts_and_terminal_direction() {
    let mut hold_open = receipt_for_class(QuickChainOperationClassV1::HoldOpen);
    hold_open.from_account_id = None;
    assert_invalid_field(hold_open, "from_account_id");

    let mut hold_open = receipt_for_class(QuickChainOperationClassV1::HoldOpen);
    hold_open.hold_id = None;
    assert_invalid_field(hold_open, "hold_id");

    let mut capture = receipt_for_class(QuickChainOperationClassV1::HoldCapture);
    capture.from_account_id = None;
    assert_invalid_field(capture, "from_account_id");

    let mut capture = receipt_for_class(QuickChainOperationClassV1::HoldCapture);
    capture.to_account_id = None;
    assert_invalid_field(capture, "to_account_id");

    let mut capture = receipt_for_class(QuickChainOperationClassV1::HoldCapture);
    capture.hold_id = None;
    assert_invalid_field(capture, "hold_id");

    for op_class in [
        QuickChainOperationClassV1::HoldRelease,
        QuickChainOperationClassV1::HoldExpire,
    ] {
        let mut receipt = receipt_for_class(op_class);
        receipt.from_account_id = None;
        assert_invalid_field(receipt, "from_account_id");

        let mut receipt = receipt_for_class(op_class);
        receipt.to_account_id = Some("account:unexpected-destination".to_string());
        assert_invalid_field(receipt, "to_account_id");

        let mut receipt = receipt_for_class(op_class);
        receipt.hold_id = None;
        assert_invalid_field(receipt, "hold_id");
    }
}

#[test]
fn receipt_rejects_non_roc_asset_bad_money_bad_ids_and_empty_memo() {
    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);
    receipt.asset = "rox".to_string();
    receipt
        .validate()
        .expect_err("non-roc receipt asset rejects");

    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);
    receipt.amount_minor = "01".to_string();
    receipt
        .validate()
        .expect_err("noncanonical amount_minor rejects");

    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);
    receipt.operation_id = "op_NOTLOWERHEX".to_string();
    receipt.validate().expect_err("bad operation_id rejects");

    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);
    receipt.idempotency_key = "bad key with space".to_string();
    receipt
        .validate()
        .expect_err("idempotency keys with spaces reject");

    let mut receipt = valid_receipt(QuickChainReceiptStatusV1::Accepted);
    receipt.memo = Some("   ".to_string());
    receipt.validate().expect_err("empty memo rejects");
}
