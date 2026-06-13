#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for pure operation-intent projection into frozen ron-proto operation-hash payload DTOs.
//! RO:WHY — ECON/RES: immutable economic identity and explicit policy context must agree before canonical bytes, hashes, or receipts exist.
//! RO:INTERACTS — QuickChain operation intents, operation projection context, and ron-proto operation-hash payload validation.
//! RO:INVARIANTS — all current operation classes; exact family tokens; invalid excluded intent fields reject; no serialization, hashing, roots, clocks, IO, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture policy and chain-parameter IDs are inert content IDs, not authorization or production root claims.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    project_operation_hash_payload, QuickChainHashPayloadProjectionError,
    QuickChainOperationHashProjectionContext,
};
use ron_proto::{
    quickchain::{
        QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC, QUICKCHAIN_OPERATION_HASH_PAYLOAD_SCHEMA,
        QUICKCHAIN_OPERATION_INTENT_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

/// Produce a real BLAKE3 content identifier for an inert test label.
///
/// These IDs test explicit projection plumbing only. They are not production
/// policy, chain-parameter, operation, receipt, tree, or checkpoint hashes.
fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("BLAKE3 test content ID should parse")
}

fn operation_for_class(
    operation_hex_digit: char,
    op_class: QuickChainOperationClassV1,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    let mut operation = QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: format!("idem:operation-projection:{operation_hex_digit}"),
        op_class,
        actor_account_id: "account:alice".to_string(),
        counterparty_account_id: None,
        amount_minor: Some("125".to_string()),
        hold_id: None,
        account_sequence: None,
        produced_at_ms,
    };

    match op_class {
        QuickChainOperationClassV1::Issue => {
            operation.actor_account_id = "account:recipient".to_string();
        }

        QuickChainOperationClassV1::Transfer => {
            operation.counterparty_account_id = Some("account:bob".to_string());
        }

        QuickChainOperationClassV1::Burn => {}

        QuickChainOperationClassV1::HoldOpen => {
            operation.counterparty_account_id = Some("account:merchant".to_string());
            operation.hold_id = Some("hold_11111111111111111111111111111111".to_string());
        }

        QuickChainOperationClassV1::HoldCapture => {
            operation.counterparty_account_id = Some("account:merchant".to_string());
            operation.hold_id = Some("hold_22222222222222222222222222222222".to_string());
        }

        QuickChainOperationClassV1::HoldRelease => {
            operation.hold_id = Some("hold_33333333333333333333333333333333".to_string());
        }

        QuickChainOperationClassV1::HoldExpire => {
            operation.hold_id = Some("hold_44444444444444444444444444444444".to_string());
        }

        _ => panic!("unsupported future operation class in test fixture"),
    }

    operation
}

fn projection_context(
    operation_id: &str,
    purpose: &str,
    session_budget_id: Option<&str>,
) -> QuickChainOperationHashProjectionContext {
    QuickChainOperationHashProjectionContext::new(
        operation_id,
        purpose,
        session_budget_id.map(str::to_owned),
        test_content_id(&format!("{operation_id}/policy")),
        test_content_id(&format!("{operation_id}/chain-params")),
    )
}

#[test]
fn all_current_operation_classes_project_exact_canonical_family_tokens() {
    let cases = [
        (QuickChainOperationClassV1::Issue, '1', "issue"),
        (QuickChainOperationClassV1::Transfer, '2', "transfer"),
        (QuickChainOperationClassV1::Burn, '3', "burn"),
        (QuickChainOperationClassV1::HoldOpen, '4', "hold_open"),
        (QuickChainOperationClassV1::HoldCapture, '5', "hold_capture"),
        (QuickChainOperationClassV1::HoldRelease, '6', "hold_release"),
        (QuickChainOperationClassV1::HoldExpire, '7', "hold_expire"),
    ];

    for (op_class, hex_digit, expected_family) in cases {
        let intent = operation_for_class(hex_digit, op_class, 1_777_000_000_000);

        let session_budget_id = if op_class == QuickChainOperationClassV1::Transfer {
            Some("budget_transfer_001")
        } else {
            None
        };

        let context = projection_context(
            &intent.operation_id,
            &format!("purpose_{expected_family}"),
            session_budget_id,
        );

        let payload = project_operation_hash_payload(&intent, &context)
            .expect("valid intent and explicit context should project");

        payload
            .validate()
            .expect("projected operation payload must satisfy ron-proto");

        assert_eq!(payload.schema, QUICKCHAIN_OPERATION_HASH_PAYLOAD_SCHEMA);
        assert_eq!(payload.version, QUICKCHAIN_DTO_VERSION);
        assert_eq!(payload.chain_id, CHAIN_ID);
        assert_eq!(payload.operation_id, intent.operation_id);
        assert_eq!(payload.op_class, op_class);
        assert_eq!(payload.actor_account_id, intent.actor_account_id);
        assert_eq!(
            payload.counterparty_account_id,
            intent.counterparty_account_id
        );
        assert_eq!(payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
        assert_eq!(payload.amount_minor, "125");
        assert_eq!(payload.purpose, format!("purpose_{expected_family}"));
        assert_eq!(payload.hold_id, intent.hold_id);
        assert_eq!(payload.session_budget_id.as_deref(), session_budget_id);
        assert_eq!(
            payload.idempotency_scope_account_id,
            intent.actor_account_id
        );
        assert_eq!(payload.idempotency_scope_operation_family, expected_family);
        assert_eq!(payload.idempotency_key, intent.idempotency_key);
        assert_eq!(payload.policy_hash, *context.policy_hash());
        assert_eq!(payload.chain_params_hash, *context.chain_params_hash());
    }
}

#[test]
fn projection_context_cannot_be_rebound_to_another_operation() {
    let intent = operation_for_class('1', QuickChainOperationClassV1::Transfer, 1_777_000_000_000);

    let context = projection_context(&operation_id('2'), "paid_transfer", None);

    let error = project_operation_hash_payload(&intent, &context)
        .expect_err("context for another operation must reject");

    assert_eq!(
        error,
        QuickChainHashPayloadProjectionError::OperationContextMismatch {
            context_operation_id: operation_id('2'),
            intent_operation_id: operation_id('1'),
        }
    );
}

#[test]
fn client_assigned_account_sequence_is_rejected_not_silently_dropped() {
    let mut intent = operation_for_class('3', QuickChainOperationClassV1::Issue, 1_777_000_000_000);
    intent.account_sequence = Some(9);

    let context = projection_context(&intent.operation_id, "admin_issue", None);

    let error = project_operation_hash_payload(&intent, &context)
        .expect_err("invalid excluded intent fields must reject before projection");

    match error {
        QuickChainHashPayloadProjectionError::InvalidOperationIntent {
            operation_id,
            reason,
        } => {
            assert_eq!(operation_id, intent.operation_id);
            assert!(!reason.is_empty());
        }

        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn ron_proto_rejects_invalid_explicit_operation_context() {
    let intent = operation_for_class('4', QuickChainOperationClassV1::HoldOpen, 1_777_000_000_000);

    // Purpose is a canonical token, so embedded spaces are invalid.
    let context = projection_context(
        &intent.operation_id,
        "paid storage",
        Some("budget_hold_001"),
    );

    let error = project_operation_hash_payload(&intent, &context)
        .expect_err("invalid purpose context must reject");

    match error {
        QuickChainHashPayloadProjectionError::InvalidOperationHashPayload {
            operation_id,
            reason,
        } => {
            assert_eq!(operation_id, intent.operation_id);
            assert!(!reason.is_empty());
        }

        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn produced_timestamp_is_excluded_and_projection_is_read_only() {
    let first_intent =
        operation_for_class('5', QuickChainOperationClassV1::Transfer, 1_777_000_000_000);

    let mut second_intent = first_intent.clone();
    second_intent.produced_at_ms = 1_888_000_000_000;

    let original_first = first_intent.clone();
    let original_second = second_intent.clone();

    let context = projection_context(
        &first_intent.operation_id,
        "paid_transfer",
        Some("budget_transfer_002"),
    );
    let original_context = context.clone();

    let first = project_operation_hash_payload(&first_intent, &context)
        .expect("first projection should succeed");

    let second = project_operation_hash_payload(&second_intent, &context)
        .expect("second projection should succeed");

    // The frozen operation-hash payload intentionally excludes runtime
    // produced_at_ms. Receipt production time belongs to the receipt payload.
    assert_eq!(first, second);

    // Projection reads but does not mutate either source input.
    assert_eq!(first_intent, original_first);
    assert_eq!(second_intent, original_second);
    assert_eq!(context, original_context);
}
