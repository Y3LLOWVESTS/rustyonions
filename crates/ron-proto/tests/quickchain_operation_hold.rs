use ron_proto::{
    QuickChainHoldStateV1, QuickChainHoldStatusV1, QuickChainOperationClassV1,
    QuickChainOperationIntentV1, QuickChainValidationError, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_HOLD_STATE_SCHEMA, QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};
use serde_json::json;

const OP_A: &str = "op_0123456789abcdef0123456789abcdef";
const OP_B: &str = "op_abcdef0123456789abcdef0123456789";
const HOLD_A: &str = "hold_0123456789abcdef0123456789abcdef";

fn operation_for_class(op_class: QuickChainOperationClassV1) -> QuickChainOperationIntentV1 {
    let mut operation = QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        operation_id: OP_A.to_string(),
        idempotency_key: "idem:0001".to_string(),
        op_class,
        actor_account_id: "account:viewer-a".to_string(),
        counterparty_account_id: None,
        amount_minor: Some("100".to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms: 1_777_000_000_000,
    };

    match op_class {
        QuickChainOperationClassV1::Issue => {
            operation.actor_account_id = "account:recipient-a".to_string();
        }

        QuickChainOperationClassV1::Transfer => {
            operation.counterparty_account_id = Some("account:creator-b".to_string());
        }

        QuickChainOperationClassV1::Burn => {
            operation.actor_account_id = "account:holder-a".to_string();
        }

        QuickChainOperationClassV1::HoldOpen => {
            operation.counterparty_account_id = Some("account:creator-b".to_string());
            operation.hold_id = Some(HOLD_A.to_string());
        }

        QuickChainOperationClassV1::HoldCapture => {
            operation.counterparty_account_id = Some("account:creator-b".to_string());
            operation.hold_id = Some(HOLD_A.to_string());
        }

        QuickChainOperationClassV1::HoldRelease | QuickChainOperationClassV1::HoldExpire => {
            operation.hold_id = Some(HOLD_A.to_string());
        }

        _ => panic!("unsupported future QuickChain operation class in test fixture"),
    }

    operation
}

fn open_hold() -> QuickChainHoldStateV1 {
    QuickChainHoldStateV1 {
        schema: QUICKCHAIN_HOLD_STATE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        hold_id: HOLD_A.to_string(),
        account_id: "account:viewer-a".to_string(),
        counterparty_account_id: Some("account:creator-b".to_string()),
        amount_minor: "100".to_string(),
        status: QuickChainHoldStatusV1::Open,
        opened_operation_id: OP_A.to_string(),
        terminal_operation_id: None,
        opened_at_ms: 1_777_000_000_000,
        expires_at_ms: 1_777_003_600_000,
        terminal_at_ms: None,
        account_sequence_opened: 7,
        account_sequence_terminal: None,
    }
}

fn assert_invalid_field(operation: QuickChainOperationIntentV1, expected_field: &'static str) {
    let error = operation.validate().unwrap_err();

    match error {
        QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, expected_field);
        }
        other => panic!("expected InvalidField for {expected_field}, got {other:?}"),
    }
}

#[test]
fn every_operation_class_has_one_valid_precommit_shape() {
    for op_class in [
        QuickChainOperationClassV1::Issue,
        QuickChainOperationClassV1::Transfer,
        QuickChainOperationClassV1::Burn,
        QuickChainOperationClassV1::HoldOpen,
        QuickChainOperationClassV1::HoldCapture,
        QuickChainOperationClassV1::HoldRelease,
        QuickChainOperationClassV1::HoldExpire,
    ] {
        operation_for_class(op_class).validate().unwrap();
    }
}

#[test]
fn operation_class_wire_names_are_exact_and_unknown_values_reject() {
    let cases = [
        (QuickChainOperationClassV1::Issue, "issue"),
        (QuickChainOperationClassV1::Transfer, "transfer"),
        (QuickChainOperationClassV1::Burn, "burn"),
        (QuickChainOperationClassV1::HoldOpen, "hold_open"),
        (QuickChainOperationClassV1::HoldCapture, "hold_capture"),
        (QuickChainOperationClassV1::HoldRelease, "hold_release"),
        (QuickChainOperationClassV1::HoldExpire, "hold_expire"),
    ];

    for (op_class, wire) in cases {
        assert_eq!(
            serde_json::to_string(&op_class).unwrap(),
            format!("\"{wire}\"")
        );
    }

    serde_json::from_value::<QuickChainOperationClassV1>(json!("mint"))
        .expect_err("unknown operation classes must reject");
}

#[test]
fn operation_intent_rejects_unknown_fields() {
    let mut value =
        serde_json::to_value(operation_for_class(QuickChainOperationClassV1::HoldOpen)).unwrap();

    value
        .as_object_mut()
        .unwrap()
        .insert("ledger_authority".to_string(), json!(true));

    serde_json::from_value::<QuickChainOperationIntentV1>(value)
        .expect_err("unknown operation fields must reject");
}

#[test]
fn operation_intent_rejects_ledger_assigned_account_sequence() {
    for op_class in [
        QuickChainOperationClassV1::Issue,
        QuickChainOperationClassV1::Transfer,
        QuickChainOperationClassV1::Burn,
        QuickChainOperationClassV1::HoldOpen,
        QuickChainOperationClassV1::HoldCapture,
        QuickChainOperationClassV1::HoldRelease,
        QuickChainOperationClassV1::HoldExpire,
    ] {
        let mut operation = operation_for_class(op_class);
        operation.account_sequence = Some(7);

        assert_invalid_field(operation, "account_sequence");
    }
}

#[test]
fn every_operation_class_requires_explicit_amount() {
    for op_class in [
        QuickChainOperationClassV1::Issue,
        QuickChainOperationClassV1::Transfer,
        QuickChainOperationClassV1::Burn,
        QuickChainOperationClassV1::HoldOpen,
        QuickChainOperationClassV1::HoldCapture,
        QuickChainOperationClassV1::HoldRelease,
        QuickChainOperationClassV1::HoldExpire,
    ] {
        let mut operation = operation_for_class(op_class);
        operation.amount_minor = None;

        assert_invalid_field(operation, "amount_minor");
    }
}

#[test]
fn non_hold_operation_intents_reject_conflicting_fields() {
    let mut issue = operation_for_class(QuickChainOperationClassV1::Issue);
    issue.counterparty_account_id = Some("account:issuer".to_string());
    assert_invalid_field(issue, "counterparty_account_id");

    let mut issue = operation_for_class(QuickChainOperationClassV1::Issue);
    issue.hold_id = Some(HOLD_A.to_string());
    assert_invalid_field(issue, "hold_id");

    let mut transfer = operation_for_class(QuickChainOperationClassV1::Transfer);
    transfer.hold_id = Some(HOLD_A.to_string());
    assert_invalid_field(transfer, "hold_id");

    let mut burn = operation_for_class(QuickChainOperationClassV1::Burn);
    burn.counterparty_account_id = Some("account:sink".to_string());
    assert_invalid_field(burn, "counterparty_account_id");

    let mut burn = operation_for_class(QuickChainOperationClassV1::Burn);
    burn.hold_id = Some(HOLD_A.to_string());
    assert_invalid_field(burn, "hold_id");
}

#[test]
fn hold_operation_intents_enforce_lifecycle_matrix() {
    let mut open_without_counterparty = operation_for_class(QuickChainOperationClassV1::HoldOpen);
    open_without_counterparty.counterparty_account_id = None;
    open_without_counterparty.validate().unwrap();

    let mut open_without_hold = operation_for_class(QuickChainOperationClassV1::HoldOpen);
    open_without_hold.hold_id = None;
    assert_invalid_field(open_without_hold, "hold_id");

    let mut capture_without_counterparty =
        operation_for_class(QuickChainOperationClassV1::HoldCapture);
    capture_without_counterparty.counterparty_account_id = None;
    assert_invalid_field(capture_without_counterparty, "counterparty_account_id");

    let mut capture_without_hold = operation_for_class(QuickChainOperationClassV1::HoldCapture);
    capture_without_hold.hold_id = None;
    assert_invalid_field(capture_without_hold, "hold_id");

    for op_class in [
        QuickChainOperationClassV1::HoldRelease,
        QuickChainOperationClassV1::HoldExpire,
    ] {
        let mut with_counterparty = operation_for_class(op_class);
        with_counterparty.counterparty_account_id = Some("account:creator-b".to_string());
        assert_invalid_field(with_counterparty, "counterparty_account_id");

        let mut without_hold = operation_for_class(op_class);
        without_hold.hold_id = None;
        assert_invalid_field(without_hold, "hold_id");
    }
}

#[test]
fn operation_intent_rejects_bad_ids_and_noncanonical_money() {
    let mut operation = operation_for_class(QuickChainOperationClassV1::Transfer);
    operation.operation_id = "op_ABCDEF0123456789abcdef0123456789".to_string();

    operation
        .validate()
        .expect_err("uppercase operation id must reject");

    let mut operation = operation_for_class(QuickChainOperationClassV1::Transfer);
    operation.counterparty_account_id = None;

    assert_invalid_field(operation, "counterparty_account_id");

    let mut operation = operation_for_class(QuickChainOperationClassV1::Issue);
    operation.amount_minor = Some("1.0".to_string());

    operation
        .validate()
        .expect_err("float-like money must reject");
}

#[test]
fn hold_state_validates_open_and_terminal_lifecycle_shapes() {
    open_hold().validate().unwrap();

    let mut captured = open_hold();
    captured.status = QuickChainHoldStatusV1::Captured;
    captured.terminal_operation_id = Some(OP_B.to_string());
    captured.terminal_at_ms = Some(1_777_000_100_000);
    captured.account_sequence_terminal = Some(8);

    captured.validate().unwrap();
}

#[test]
fn hold_state_rejects_ambiguous_lifecycle_fields() {
    let mut open = open_hold();
    open.terminal_operation_id = Some(OP_B.to_string());

    open.validate()
        .expect_err("open hold must not include terminal operation");

    let mut terminal = open_hold();
    terminal.status = QuickChainHoldStatusV1::Released;

    terminal
        .validate()
        .expect_err("terminal hold must include terminal lifecycle fields");

    let mut bad_time = open_hold();
    bad_time.expires_at_ms = bad_time.opened_at_ms - 1;

    bad_time
        .validate()
        .expect_err("bad hold timestamp order must reject");
}
