#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for pure account snapshot projection into frozen ron-proto account-leaf DTOs.
//! RO:WHY — ECON/RES: ledger balances/sequences and externally reviewed commitments must agree before canonical bytes or roots are allowed.
//! RO:INTERACTS — QuickChainAtomicState, state snapshots, account projection context, and ron-proto account-leaf payloads.
//! RO:INVARIANTS — sorted account output; exact context set; explicit roots/epoch; no root calculation, hashing, clocks, IO, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture content IDs are opaque adapter inputs and are not claimed as production tree roots or golden vectors.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAccountLeafProjectionContext, QuickChainAtomicState, QuickChainHoldEpochInput,
    QuickChainLeafProjectionError, QuickChainSupplyDecision,
};
use ron_proto::{
    quickchain::{
        QuickChainOperationClassV1, QuickChainOperationIntentV1,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC, QUICKCHAIN_OPERATION_INTENT_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

/// Produce a real BLAKE3 content identifier for an inert test label.
///
/// These values test explicit adapter plumbing only. They are not receipt-tree,
/// hold-tree, permissions-tree, root-vector, or production-hash claims.
fn test_content_id(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("BLAKE3 test content ID should parse")
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
                &format!("idem:account-leaf:issue:{operation_hex_digit}"),
                QuickChainOperationClassV1::Issue,
                account_id,
                None,
                amount_minor,
                None,
                1_000,
            ),
            QuickChainSupplyDecision::IssueApproved,
            format!("tx:roc:account-leaf:issue:{operation_hex_digit}"),
        )
        .expect("test issue should commit");
}

fn commit_transfer(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    actor: &str,
    counterparty: &str,
    amount_minor: &str,
) {
    state
        .execute_balance_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:account-leaf:transfer:{operation_hex_digit}"),
                QuickChainOperationClassV1::Transfer,
                actor,
                Some(counterparty),
                amount_minor,
                None,
                2_000,
            ),
            QuickChainSupplyDecision::NoSupplyChange,
            format!("tx:roc:account-leaf:transfer:{operation_hex_digit}"),
        )
        .expect("test transfer should commit");
}

#[allow(clippy::too_many_arguments)]
fn commit_open_hold(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    account_id: &str,
    amount_minor: &str,
    hold_id: &str,
    created_at_epoch: u64,
    expires_at_epoch: u64,
) {
    state
        .execute_hold_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:account-leaf:hold:{operation_hex_digit}"),
                QuickChainOperationClassV1::HoldOpen,
                account_id,
                Some("account:merchant"),
                amount_minor,
                Some(hold_id),
                3_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch,
                expires_at_epoch,
            },
            format!("tx:roc:account-leaf:hold:{operation_hex_digit}"),
        )
        .expect("test hold should open");
}

fn projection_context(
    account_id: &str,
    receipt_root: ContentId,
    holds_root: ContentId,
    permissions_root: Option<ContentId>,
    updated_at_epoch: &str,
) -> QuickChainAccountLeafProjectionContext {
    QuickChainAccountLeafProjectionContext::new(
        account_id,
        receipt_root,
        holds_root,
        permissions_root,
        updated_at_epoch,
    )
}

fn snapshot_with_one_account() -> ron_ledger::quickchain::QuickChainStateSnapshot {
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .state_snapshot()
        .expect("valid one-account state should snapshot")
}

#[test]
fn account_payloads_are_valid_and_follow_frozen_account_sort_order() {
    let bob_hold = hold_id('d');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, 'a', "account:alice", "500");
    commit_transfer(&mut state, 'b', "account:alice", "account:bob", "150");
    commit_open_hold(&mut state, 'c', "account:bob", "70", &bob_hold, 3, 30);

    let snapshot = state
        .state_snapshot()
        .expect("valid populated state should snapshot");

    let alice_receipt_root = test_content_id("account-leaf/alice/receipt-root");
    let alice_holds_root = test_content_id("account-leaf/alice/holds-root");
    let alice_permissions_root = test_content_id("account-leaf/alice/permissions-root");

    let bob_receipt_root = test_content_id("account-leaf/bob/receipt-root");
    let bob_holds_root = test_content_id("account-leaf/bob/holds-root");

    // Context order is intentionally reversed. Payload ordering must come from
    // the deterministic account snapshot and frozen account sort-key rule.
    let contexts = vec![
        projection_context(
            "account:bob",
            bob_receipt_root.clone(),
            bob_holds_root.clone(),
            None,
            "epoch_0003",
        ),
        projection_context(
            "account:alice",
            alice_receipt_root.clone(),
            alice_holds_root.clone(),
            Some(alice_permissions_root.clone()),
            "epoch_0002",
        ),
    ];

    let payloads = snapshot
        .project_account_leaf_payloads(&contexts)
        .expect("explicit valid account context should project");

    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0].account_id, "account:alice");
    assert_eq!(payloads[1].account_id, "account:bob");

    for payload in &payloads {
        payload
            .validate()
            .expect("projected account payload must satisfy ron-proto");

        assert_eq!(payload.schema, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA);
        assert_eq!(payload.version, QUICKCHAIN_DTO_VERSION);
        assert_eq!(payload.chain_id, CHAIN_ID);
        assert_eq!(payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
    }

    let alice = &payloads[0];
    assert_eq!(alice.balance_minor, "350");
    assert_eq!(alice.held_minor, "0");
    assert_eq!(alice.available_minor, "350");
    assert_eq!(alice.account_sequence, 2);
    assert_eq!(alice.receipt_root, alice_receipt_root);
    assert_eq!(alice.holds_root, alice_holds_root);
    assert_eq!(
        alice.permissions_root.as_ref(),
        Some(&alice_permissions_root)
    );
    assert_eq!(alice.updated_at_epoch, "epoch_0002");

    let bob = &payloads[1];
    assert_eq!(bob.balance_minor, "150");
    assert_eq!(bob.held_minor, "70");
    assert_eq!(bob.available_minor, "80");

    // Transfer credit advanced Bob to sequence 1; opening the hold advanced
    // Bob's leaf-relevant account state to sequence 2.
    assert_eq!(bob.account_sequence, 2);

    assert_eq!(bob.receipt_root, bob_receipt_root);
    assert_eq!(bob.holds_root, bob_holds_root);
    assert_eq!(bob.permissions_root, None);
    assert_eq!(bob.updated_at_epoch, "epoch_0003");
}

#[test]
fn projection_context_set_must_match_account_set_exactly() {
    let snapshot = snapshot_with_one_account();

    let missing = snapshot
        .project_account_leaf_payloads(&[])
        .expect_err("account without context must reject");

    assert_eq!(
        missing,
        QuickChainLeafProjectionError::MissingAccountContext {
            account_id: "account:alice".to_string(),
        }
    );

    let valid = projection_context(
        "account:alice",
        test_content_id("account-set/alice/receipts"),
        test_content_id("account-set/alice/holds"),
        None,
        "epoch_0001",
    );

    let duplicate = snapshot
        .project_account_leaf_payloads(&[valid.clone(), valid.clone()])
        .expect_err("duplicate account context must reject");

    assert_eq!(
        duplicate,
        QuickChainLeafProjectionError::DuplicateAccountContext {
            account_id: "account:alice".to_string(),
        }
    );

    let unknown = projection_context(
        "account:bob",
        test_content_id("account-set/bob/receipts"),
        test_content_id("account-set/bob/holds"),
        None,
        "epoch_0001",
    );

    let extra = snapshot
        .project_account_leaf_payloads(&[valid, unknown])
        .expect_err("context for an absent account must reject");

    assert_eq!(
        extra,
        QuickChainLeafProjectionError::UnknownAccountContext {
            account_id: "account:bob".to_string(),
        }
    );
}

#[test]
fn ron_proto_rejects_invalid_updated_epoch_context() {
    let snapshot = snapshot_with_one_account();

    let context = projection_context(
        "account:alice",
        test_content_id("invalid-epoch/alice/receipts"),
        test_content_id("invalid-epoch/alice/holds"),
        None,
        "epoch with spaces",
    );

    let error = snapshot
        .project_account_leaf_payloads(&[context])
        .expect_err("invalid canonical epoch ID must reject");

    match error {
        QuickChainLeafProjectionError::InvalidAccountPayload { account_id, reason } => {
            assert_eq!(account_id, "account:alice");
            assert!(!reason.is_empty());
        }

        other => panic!("unexpected projection error: {other:?}"),
    }
}

#[test]
fn empty_snapshot_projects_no_account_payloads() {
    let state = QuickChainAtomicState::new();

    let snapshot = state.state_snapshot().expect("empty state should snapshot");

    let payloads = snapshot
        .project_account_leaf_payloads(&[])
        .expect("empty snapshot needs no invented chain or root context");

    assert!(payloads.is_empty());
}

#[test]
fn repeated_projection_is_deterministic_and_read_only() {
    let snapshot = snapshot_with_one_account();
    let before = snapshot.clone();

    let context = projection_context(
        "account:alice",
        test_content_id("determinism/alice/receipts"),
        test_content_id("determinism/alice/holds"),
        Some(test_content_id("determinism/alice/permissions")),
        "epoch_0001",
    );

    let first = snapshot
        .project_account_leaf_payloads(std::slice::from_ref(&context))
        .expect("first projection should succeed");

    let second = snapshot
        .project_account_leaf_payloads(std::slice::from_ref(&context))
        .expect("second projection should succeed");

    assert_eq!(first, second);
    assert_eq!(snapshot, before);
}
