// crates/ron-ledger/tests/quickchain_terminal_hold_account_projection.rs
#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for account snapshot/leaf projection after terminal hold replay.
//! RO:WHY — ECON/RES: terminal capture evidence must produce final account rows without becoming active-hold leaf state.
//! RO:INTERACTS — QuickChainAcceptedOperation, QuickChainAtomicState, state snapshots, account leaf projection, and ron-proto DTO validation.
//! RO:INVARIANTS — terminal holds are not active leaves; capture counterparty account state is explicit; projection context is never invented.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture content IDs and receipt references are inert test values, not roots, receipts, proofs, or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAccountLeafProjectionContext, QuickChainAtomicState,
    QuickChainHoldEpochInput, QuickChainLeafProjectionError, QuickChainSupplyDecision,
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

#[allow(clippy::too_many_arguments)]
fn accept_balance(
    state: &mut QuickChainAtomicState,
    history: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    actor: &str,
    amount_minor: &str,
    supply_decision: QuickChainSupplyDecision,
    txid: &str,
    produced_at_ms: u64,
) {
    let operation = intent(
        operation_hex_digit,
        idempotency_key,
        QuickChainOperationClassV1::Issue,
        actor,
        None,
        amount_minor,
        None,
        produced_at_ms,
    );

    let outcome = state
        .execute_balance_operation(&operation, supply_decision, txid)
        .expect("balance operation should commit");

    history.push(QuickChainAcceptedOperation::balance(
        outcome.record().clone(),
        supply_decision,
    ));
}

#[allow(clippy::too_many_arguments)]
fn accept_hold(
    state: &mut QuickChainAtomicState,
    history: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: &str,
    epoch_input: QuickChainHoldEpochInput,
    txid: &str,
    produced_at_ms: u64,
) {
    let operation = intent(
        operation_hex_digit,
        idempotency_key,
        op_class,
        actor,
        counterparty,
        amount_minor,
        Some(hold_id),
        produced_at_ms,
    );

    let outcome = state
        .execute_hold_operation(&operation, epoch_input, txid)
        .expect("hold operation should commit");

    history.push(QuickChainAcceptedOperation::hold(
        outcome.record().clone(),
        epoch_input,
    ));
}

fn account_context(
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

fn terminal_capture_live_and_rebuilt() -> (QuickChainAtomicState, QuickChainAtomicState) {
    let hold = hold_id('a');

    let mut live = QuickChainAtomicState::new();
    let mut history = Vec::new();

    accept_balance(
        &mut live,
        &mut history,
        '1',
        "idem:terminal-account:issue-alice",
        "account:alice",
        "200",
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:terminal-account:issue-alice",
        1_000,
    );

    accept_hold(
        &mut live,
        &mut history,
        '2',
        "idem:terminal-account:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "60",
        &hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        "tx:roc:terminal-account:open",
        1_001,
    );

    accept_hold(
        &mut live,
        &mut history,
        '3',
        "idem:terminal-account:capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "25",
        &hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
        "tx:roc:terminal-account:capture",
        1_002,
    );

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&history)
        .expect("accepted terminal capture history should rebuild exactly");

    (live, rebuilt)
}

#[test]
fn terminal_capture_replay_projects_final_account_leaves_without_active_holds() {
    let (live, rebuilt) = terminal_capture_live_and_rebuilt();

    assert_eq!(rebuilt, live);
    assert_eq!(rebuilt.balance_minor("account:alice"), 175);
    assert_eq!(rebuilt.balance_minor("account:merchant"), 25);
    assert_eq!(rebuilt.held_minor("account:alice"), 0);
    assert_eq!(rebuilt.available_minor("account:alice").unwrap(), 175);
    assert_eq!(rebuilt.current_supply_minor(), 200);
    assert_eq!(rebuilt.operation_count(), 3);
    assert_eq!(rebuilt.next_ledger_sequence(), 5);
    assert_eq!(rebuilt.last_account_sequence("account:alice"), 3);
    assert_eq!(rebuilt.last_account_sequence("account:merchant"), 1);

    let before_projection = rebuilt.clone();

    let snapshot = rebuilt
        .state_snapshot()
        .expect("terminal capture final state should snapshot");

    assert_eq!(snapshot.chain_id(), Some(CHAIN_ID));
    assert_eq!(snapshot.current_supply_minor(), 200);
    assert_eq!(snapshot.operation_count(), 3);
    assert_eq!(snapshot.next_ledger_sequence(), 5);
    assert!(snapshot.active_holds().is_empty());
    assert_eq!(snapshot.accounts().len(), 2);

    let alice = &snapshot.accounts()[0];
    assert_eq!(alice.account_id(), "account:alice");
    assert_eq!(alice.balance_minor(), 175);
    assert_eq!(alice.held_minor(), 0);
    assert_eq!(alice.available_minor(), 175);
    assert_eq!(alice.account_sequence(), 3);

    let merchant = &snapshot.accounts()[1];
    assert_eq!(merchant.account_id(), "account:merchant");
    assert_eq!(merchant.balance_minor(), 25);
    assert_eq!(merchant.held_minor(), 0);
    assert_eq!(merchant.available_minor(), 25);
    assert_eq!(merchant.account_sequence(), 1);

    let alice_receipt_root = test_content_id("terminal-account/alice/receipt-root");
    let alice_holds_root = test_content_id("terminal-account/alice/holds-root");
    let alice_permissions_root = test_content_id("terminal-account/alice/permissions-root");

    let merchant_receipt_root = test_content_id("terminal-account/merchant/receipt-root");
    let merchant_holds_root = test_content_id("terminal-account/merchant/holds-root");
    let merchant_permissions_root = test_content_id("terminal-account/merchant/permissions-root");

    let contexts = vec![
        // Deliberately reverse caller order. Projection order must follow the
        // deterministic snapshot, not the caller's context order.
        account_context(
            "account:merchant",
            merchant_receipt_root.clone(),
            merchant_holds_root.clone(),
            Some(merchant_permissions_root.clone()),
            "epoch_0001",
        ),
        account_context(
            "account:alice",
            alice_receipt_root.clone(),
            alice_holds_root.clone(),
            Some(alice_permissions_root.clone()),
            "epoch_0003",
        ),
    ];

    let payloads = snapshot
        .project_account_leaf_payloads(&contexts)
        .expect("explicit final account contexts should project");

    assert_eq!(payloads.len(), 2);

    let alice_payload = &payloads[0];
    assert_eq!(alice_payload.schema, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA);
    assert_eq!(alice_payload.version, QUICKCHAIN_DTO_VERSION);
    assert_eq!(alice_payload.chain_id, CHAIN_ID);
    assert_eq!(alice_payload.account_id, "account:alice");
    assert_eq!(alice_payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
    assert_eq!(alice_payload.balance_minor, "175");
    assert_eq!(alice_payload.held_minor, "0");
    assert_eq!(alice_payload.available_minor, "175");
    assert_eq!(alice_payload.account_sequence, 3);
    assert_eq!(alice_payload.receipt_root, alice_receipt_root);
    assert_eq!(alice_payload.holds_root, alice_holds_root);
    assert_eq!(
        alice_payload.permissions_root.as_ref(),
        Some(&alice_permissions_root)
    );
    assert_eq!(alice_payload.updated_at_epoch, "epoch_0003");

    let merchant_payload = &payloads[1];
    assert_eq!(
        merchant_payload.schema,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA
    );
    assert_eq!(merchant_payload.version, QUICKCHAIN_DTO_VERSION);
    assert_eq!(merchant_payload.chain_id, CHAIN_ID);
    assert_eq!(merchant_payload.account_id, "account:merchant");
    assert_eq!(merchant_payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
    assert_eq!(merchant_payload.balance_minor, "25");
    assert_eq!(merchant_payload.held_minor, "0");
    assert_eq!(merchant_payload.available_minor, "25");
    assert_eq!(merchant_payload.account_sequence, 1);
    assert_eq!(merchant_payload.receipt_root, merchant_receipt_root);
    assert_eq!(merchant_payload.holds_root, merchant_holds_root);
    assert_eq!(
        merchant_payload.permissions_root.as_ref(),
        Some(&merchant_permissions_root)
    );
    assert_eq!(merchant_payload.updated_at_epoch, "epoch_0001");

    assert_eq!(rebuilt, before_projection);
}

#[test]
fn terminal_capture_counterparty_account_requires_explicit_projection_context() {
    let (_live, rebuilt) = terminal_capture_live_and_rebuilt();

    let snapshot = rebuilt
        .state_snapshot()
        .expect("terminal capture final state should snapshot");

    assert_eq!(snapshot.accounts().len(), 2);
    assert_eq!(snapshot.accounts()[0].account_id(), "account:alice");
    assert_eq!(snapshot.accounts()[1].account_id(), "account:merchant");

    let alice_only_context = [account_context(
        "account:alice",
        test_content_id("terminal-account/missing-counterparty/alice/receipt-root"),
        test_content_id("terminal-account/missing-counterparty/alice/holds-root"),
        Some(test_content_id(
            "terminal-account/missing-counterparty/alice/permissions-root",
        )),
        "epoch_0003",
    )];

    let error = snapshot
        .project_account_leaf_payloads(&alice_only_context)
        .expect_err("projection must not invent counterparty account context");

    assert_eq!(
        error,
        QuickChainLeafProjectionError::MissingAccountContext {
            account_id: "account:merchant".to_string(),
        }
    );
}
