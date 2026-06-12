#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for pure active-hold snapshot projection into frozen ron-proto leaf payload DTOs.
//! RO:WHY — ECON/RES: ledger state, policy context, and canonical epoch bindings must agree before canonical bytes or roots are allowed.
//! RO:INTERACTS — QuickChainAtomicState, state snapshots, leaf projection context, and ron-proto active-hold leaf payloads.
//! RO:INVARIANTS — sorted output; exact context set; explicit epochs/policy/purpose; no hashing, roots, clocks, IO, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — test policy hashes and purposes are inert reviewed inputs, not wallet or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainActiveHoldLeafProjectionContext, QuickChainAtomicState, QuickChainEpochBinding,
    QuickChainHoldEpochInput, QuickChainLeafProjectionError, QuickChainSupplyDecision,
};
use ron_proto::{
    quickchain::{
        QuickChainActiveHoldStatusV1, QuickChainOperationClassV1, QuickChainOperationIntentV1,
        QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_OPERATION_INTENT_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";

const POLICY_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const POLICY_B: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

fn content_id(value: &str) -> ContentId {
    value.parse().expect("test ContentId should be valid")
}

#[allow(clippy::too_many_arguments)]
fn intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: Option<&str>,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_string(),
        op_class,
        actor_account_id: actor.to_string(),
        counterparty_account_id: counterparty.map(str::to_string),
        amount_minor: Some(amount_minor.to_string()),
        hold_id: hold_id.map(str::to_string),
        account_sequence: None,
        produced_at_ms,
    }
}

fn commit_issue(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    account_id: &str,
    amount_minor: &str,
) {
    state
        .execute_balance_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:leaf-projection:issue:{operation_hex_digit}"),
                QuickChainOperationClassV1::Issue,
                account_id,
                None,
                amount_minor,
                None,
                1_000,
            ),
            QuickChainSupplyDecision::IssueApproved,
            format!("tx:roc:leaf-projection:issue:{operation_hex_digit}"),
        )
        .expect("test issue should commit");
}

#[allow(clippy::too_many_arguments)]
fn commit_open_hold(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    idempotency_key: &str,
    account_id: &str,
    counterparty_account_id: Option<&str>,
    amount_minor: &str,
    hold_id: &str,
    created_at_epoch: u64,
    expires_at_epoch: u64,
) {
    state
        .execute_hold_operation(
            &intent(
                operation_hex_digit,
                idempotency_key,
                QuickChainOperationClassV1::HoldOpen,
                account_id,
                counterparty_account_id,
                amount_minor,
                Some(hold_id),
                2_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch,
                expires_at_epoch,
            },
            format!("tx:roc:leaf-projection:hold:{operation_hex_digit}"),
        )
        .expect("test hold should open");
}

#[allow(clippy::too_many_arguments)]
fn projection_context(
    hold_id: &str,
    purpose: &str,
    created_epoch_number: u64,
    created_epoch_id: &str,
    expires_epoch_number: u64,
    expires_epoch_id: &str,
    policy_hash: &str,
) -> QuickChainActiveHoldLeafProjectionContext {
    QuickChainActiveHoldLeafProjectionContext::new(
        hold_id,
        purpose,
        QuickChainEpochBinding::new(created_epoch_number, created_epoch_id),
        QuickChainEpochBinding::new(expires_epoch_number, expires_epoch_id),
        content_id(policy_hash),
    )
}

fn snapshot_with_one_hold() -> (ron_ledger::quickchain::QuickChainStateSnapshot, String) {
    let hold = hold_id('a');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    commit_open_hold(
        &mut state,
        '2',
        "idem:leaf-projection:one-hold",
        "account:alice",
        Some("account:merchant"),
        "40",
        &hold,
        1,
        10,
    );

    (
        state.state_snapshot().expect("valid state should snapshot"),
        hold,
    )
}

#[test]
fn active_hold_payloads_are_valid_and_follow_snapshot_hold_order() {
    let hold_f = hold_id('f');
    let hold_1 = hold_id('1');
    let mut state = QuickChainAtomicState::new();

    // Deliberately create accounts and holds outside lexical hold-ID order.
    commit_issue(&mut state, 'a', "account:bob", "300");
    commit_issue(&mut state, 'b', "account:alice", "500");

    commit_open_hold(
        &mut state,
        'c',
        "idem:leaf-projection:hold:f",
        "account:alice",
        Some("account:storage-provider"),
        "100",
        &hold_f,
        3,
        30,
    );

    commit_open_hold(
        &mut state,
        'd',
        "idem:leaf-projection:hold:1",
        "account:bob",
        None,
        "70",
        &hold_1,
        2,
        20,
    );

    let snapshot = state
        .state_snapshot()
        .expect("valid populated state should snapshot");

    // Context order is intentionally reversed. Payload order must come from
    // the deterministic snapshot, not from caller input order.
    let contexts = vec![
        projection_context(
            &hold_f,
            "paid_storage",
            3,
            "epoch_0003",
            30,
            "epoch_0030",
            POLICY_A,
        ),
        projection_context(
            &hold_1,
            "paid_site_visit",
            2,
            "epoch_0002",
            20,
            "epoch_0020",
            POLICY_B,
        ),
    ];

    let payloads = snapshot
        .project_active_hold_leaf_payloads(&contexts)
        .expect("explicit valid context should project");

    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0].hold_id, hold_1);
    assert_eq!(payloads[1].hold_id, hold_f);

    for payload in &payloads {
        payload
            .validate()
            .expect("projected payload must satisfy ron-proto");
        assert_eq!(payload.schema, QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA);
        assert_eq!(payload.version, QUICKCHAIN_DTO_VERSION);
        assert_eq!(payload.chain_id, CHAIN_ID);
        assert_eq!(payload.status, QuickChainActiveHoldStatusV1::Open);
    }

    let bob_hold = &payloads[0];
    assert_eq!(bob_hold.account_id, "account:bob");
    assert_eq!(bob_hold.counterparty_account_id, None);
    assert_eq!(bob_hold.amount_minor, "70");
    assert_eq!(bob_hold.purpose, "paid_site_visit");
    assert_eq!(bob_hold.created_at_epoch, "epoch_0002");
    assert_eq!(bob_hold.expires_at_epoch, "epoch_0020");
    assert_eq!(bob_hold.operation_id, operation_id('d'));
    assert_eq!(bob_hold.idempotency_key, "idem:leaf-projection:hold:1");
    assert_eq!(bob_hold.policy_hash.as_str(), POLICY_B);

    let alice_hold = &payloads[1];
    assert_eq!(alice_hold.account_id, "account:alice");
    assert_eq!(
        alice_hold.counterparty_account_id.as_deref(),
        Some("account:storage-provider")
    );
    assert_eq!(alice_hold.amount_minor, "100");
    assert_eq!(alice_hold.purpose, "paid_storage");
    assert_eq!(alice_hold.created_at_epoch, "epoch_0003");
    assert_eq!(alice_hold.expires_at_epoch, "epoch_0030");
    assert_eq!(alice_hold.operation_id, operation_id('c'));
    assert_eq!(alice_hold.idempotency_key, "idem:leaf-projection:hold:f");
    assert_eq!(alice_hold.policy_hash.as_str(), POLICY_A);
}

#[test]
fn numeric_epoch_binding_must_match_ledger_owned_hold_state() {
    let (snapshot, hold) = snapshot_with_one_hold();

    let context = projection_context(
        &hold,
        "paid_storage",
        2,
        "epoch_0002",
        10,
        "epoch_0010",
        POLICY_A,
    );

    let error = snapshot
        .project_active_hold_leaf_payloads(&[context])
        .expect_err("wrong numeric creation epoch must reject");

    assert_eq!(
        error,
        QuickChainLeafProjectionError::CreatedEpochNumberMismatch {
            hold_id: hold,
            expected_epoch_number: 1,
            actual_epoch_number: 2,
        }
    );
}

#[test]
fn projection_context_set_must_match_active_hold_set_exactly() {
    let (snapshot, hold) = snapshot_with_one_hold();

    let missing = snapshot
        .project_active_hold_leaf_payloads(&[])
        .expect_err("active hold without context must reject");

    assert_eq!(
        missing,
        QuickChainLeafProjectionError::MissingActiveHoldContext {
            hold_id: hold.clone(),
        }
    );

    let valid = projection_context(
        &hold,
        "paid_storage",
        1,
        "epoch_0001",
        10,
        "epoch_0010",
        POLICY_A,
    );

    let duplicate = snapshot
        .project_active_hold_leaf_payloads(&[valid.clone(), valid.clone()])
        .expect_err("duplicate hold context must reject");

    assert_eq!(
        duplicate,
        QuickChainLeafProjectionError::DuplicateActiveHoldContext {
            hold_id: hold.clone(),
        }
    );

    let unknown_hold = hold_id('f');
    let unknown = projection_context(
        &unknown_hold,
        "paid_storage",
        1,
        "epoch_0001",
        10,
        "epoch_0010",
        POLICY_B,
    );

    let extra = snapshot
        .project_active_hold_leaf_payloads(&[valid, unknown])
        .expect_err("context for non-active hold must reject");

    assert_eq!(
        extra,
        QuickChainLeafProjectionError::UnknownActiveHoldContext {
            hold_id: unknown_hold,
        }
    );
}

#[test]
fn ron_proto_rejects_invalid_explicit_projection_context() {
    let (snapshot, hold) = snapshot_with_one_hold();

    // Spaces are not valid in the frozen purpose-token contract.
    let context = projection_context(
        &hold,
        "paid storage",
        1,
        "epoch_0001",
        10,
        "epoch_0010",
        POLICY_A,
    );

    let error = snapshot
        .project_active_hold_leaf_payloads(&[context])
        .expect_err("invalid purpose must be rejected by ron-proto");

    match error {
        QuickChainLeafProjectionError::InvalidActiveHoldPayload { hold_id, reason } => {
            assert_eq!(hold_id, hold);
            assert!(!reason.is_empty());
        }

        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn one_numeric_epoch_cannot_map_to_multiple_canonical_epoch_ids() {
    let hold_1 = hold_id('1');
    let hold_2 = hold_id('2');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "200");

    commit_open_hold(
        &mut state,
        '2',
        "idem:leaf-projection:binding:1",
        "account:alice",
        None,
        "50",
        &hold_1,
        1,
        10,
    );

    commit_open_hold(
        &mut state,
        '3',
        "idem:leaf-projection:binding:2",
        "account:alice",
        None,
        "50",
        &hold_2,
        1,
        11,
    );

    let snapshot = state.state_snapshot().expect("valid state should snapshot");

    let contexts = vec![
        projection_context(
            &hold_2,
            "paid_storage",
            1,
            "epoch_9001",
            11,
            "epoch_0011",
            POLICY_B,
        ),
        projection_context(
            &hold_1,
            "paid_storage",
            1,
            "epoch_0001",
            10,
            "epoch_0010",
            POLICY_A,
        ),
    ];

    let error = snapshot
        .project_active_hold_leaf_payloads(&contexts)
        .expect_err("one epoch number cannot bind to two canonical IDs");

    assert_eq!(
        error,
        QuickChainLeafProjectionError::EpochNumberBindingConflict {
            hold_id: hold_2,
            epoch_number: 1,
            expected_epoch_id: "epoch_0001".to_string(),
            actual_epoch_id: "epoch_9001".to_string(),
        }
    );
}
