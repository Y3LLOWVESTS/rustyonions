use ron_proto::{
    validate_hold_id_v1, validate_idempotency_key_v1, validate_operation_id_v1,
    validate_quickchain_minor_units, QuickChainAccountStateV1, QuickChainValidationError,
    QUICKCHAIN_ACCOUNT_STATE_SCHEMA, QUICKCHAIN_DTO_VERSION,
};
use serde_json::json;

#[test]
fn minor_unit_strings_accept_only_canonical_unsigned_integers() {
    for value in [
        "0",
        "1",
        "1000000",
        "340282366920938463463374607431768211455",
    ] {
        validate_quickchain_minor_units("amount_minor", value).unwrap();
    }

    for value in ["", "01", "+1", "-1", "1.0", "1_000", "1 ROC"] {
        let err = validate_quickchain_minor_units("amount_minor", value).unwrap_err();
        assert!(matches!(
            err,
            QuickChainValidationError::InvalidMoney { .. }
        ));
    }
}

#[test]
fn json_numeric_money_rejects_before_validation() {
    let value = json!({
        "schema": QUICKCHAIN_ACCOUNT_STATE_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "chain_id": "roc-dev",
        "account_id": "account:creator-a",
        "available_minor_units": 1,
        "held_minor_units": "0",
        "nonce": 1,
        "last_ledger_seq": 1
    });

    let err = serde_json::from_value::<QuickChainAccountStateV1>(value)
        .expect_err("numeric money must reject because DTO field is string");
    assert!(err.to_string().contains("invalid type"));
}

#[test]
fn operation_and_hold_ids_have_strict_shapes() {
    validate_operation_id_v1("operation_id", "op_0123456789abcdef0123456789abcdef").unwrap();
    validate_hold_id_v1("hold_id", "hold_0123456789abcdef0123456789abcdef").unwrap();

    for value in [
        "0123456789abcdef0123456789abcdef",
        "OP_0123456789abcdef0123456789abcdef",
        "op_0123456789ABCDEF0123456789abcdef",
        "op_0123456789abcdef0123456789abcde",
        "op_0123456789abcdef0123456789abcdef00",
        "op_0123456789abcdef0123456789abcdeg",
    ] {
        validate_operation_id_v1("operation_id", value)
            .expect_err("invalid operation id must reject");
    }

    for value in [
        "0123456789abcdef0123456789abcdef",
        "HOLD_0123456789abcdef0123456789abcdef",
        "hold_0123456789ABCDEF0123456789abcdef",
        "hold_0123456789abcdef0123456789abcde",
        "hold_0123456789abcdef0123456789abcdef00",
        "hold_0123456789abcdef0123456789abcdeg",
    ] {
        validate_hold_id_v1("hold_id", value).expect_err("invalid hold id must reject");
    }
}

#[test]
fn idempotency_key_is_bounded_visible_ascii_not_authority() {
    validate_idempotency_key_v1("idempotency_key", "publish-image:retry-0001").unwrap();
    validate_idempotency_key_v1("idempotency_key", &"x".repeat(128)).unwrap();

    for value in ["", "contains space", "contains\nnewline", "contains\ttab"] {
        validate_idempotency_key_v1("idempotency_key", value)
            .expect_err("invalid idempotency key must reject");
    }

    validate_idempotency_key_v1("idempotency_key", &"x".repeat(129))
        .expect_err("overlong idempotency key must reject");
}
