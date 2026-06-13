#![cfg(feature = "quickchain-preflight")]

//! RO:WHAT — Integration tests proving QuickChain projections require explicit reviewed context and do not invent roots or hashes.
//! RO:WHY — ECON/SEC/GOV: pre-root adapters must copy reviewed commitments only; root production remains forbidden.
//! RO:INTERACTS — QuickChainAtomicState, leaf/hash projection contexts, committed operation records, and ron-proto DTOs.
//! RO:INVARIANTS — no default roots; no computed hashes; explicit epochs/policy/continuity only; projection is read-only.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture b3 values are inert test commitments and grant no wallet, receipt, proof, or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    project_operation_hash_payload, project_receipt_hash_payload,
    QuickChainAccountLeafProjectionContext, QuickChainActiveHoldLeafProjectionContext,
    QuickChainAtomicState, QuickChainCommittedOperationRecord, QuickChainEpochBinding,
    QuickChainHoldEpochInput, QuickChainLeafProjectionError,
    QuickChainOperationHashProjectionContext, QuickChainReceiptHashProjectionContext,
    QuickChainSupplyDecision,
};
use ron_proto::{
    quickchain::{
        QuickChainActiveHoldStatusV1, QuickChainOperationClassV1, QuickChainOperationIntentV1,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC,
        QUICKCHAIN_OPERATION_INTENT_SCHEMA,
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
/// These values verify explicit adapter plumbing only. They are not root
/// vectors, production roots, golden hashes, proof commitments, or settlement
/// evidence.
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
) -> QuickChainCommittedOperationRecord {
    state
        .execute_balance_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:projection-boundary:issue:{operation_hex_digit}"),
                QuickChainOperationClassV1::Issue,
                account_id,
                None,
                amount_minor,
                None,
                1_000,
            ),
            QuickChainSupplyDecision::IssueApproved,
            format!("tx:roc:projection-boundary:issue:{operation_hex_digit}"),
        )
        .expect("test issue should commit")
        .record()
        .clone()
}

#[allow(clippy::too_many_arguments)]
fn commit_open_hold(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    account_id: &str,
    counterparty_account_id: Option<&str>,
    amount_minor: &str,
    hold_id: &str,
    created_at_epoch: u64,
    expires_at_epoch: u64,
) -> QuickChainCommittedOperationRecord {
    state
        .execute_hold_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:projection-boundary:hold-open:{operation_hex_digit}"),
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
            format!("tx:roc:projection-boundary:hold-open:{operation_hex_digit}"),
        )
        .expect("test hold should open")
        .record()
        .clone()
}

#[test]
fn account_leaf_projection_requires_explicit_root_context_and_copies_it() {
    let mut state = QuickChainAtomicState::new();
    commit_issue(&mut state, '1', "account:alice", "100");

    let snapshot = state
        .state_snapshot()
        .expect("valid account state should snapshot");

    let missing = snapshot
        .project_account_leaf_payloads(&[])
        .expect_err("account projection must not invent receipt/hold roots");

    assert_eq!(
        missing,
        QuickChainLeafProjectionError::MissingAccountContext {
            account_id: "account:alice".to_string(),
        }
    );

    let receipt_root = test_content_id("projection-boundary/account/alice/receipt-root");
    let holds_root = test_content_id("projection-boundary/account/alice/holds-root");
    let permissions_root = test_content_id("projection-boundary/account/alice/permissions-root");

    let contexts = vec![QuickChainAccountLeafProjectionContext::new(
        "account:alice",
        receipt_root.clone(),
        holds_root.clone(),
        Some(permissions_root.clone()),
        "epoch_0001",
    )];

    let payloads = snapshot
        .project_account_leaf_payloads(&contexts)
        .expect("explicit reviewed account context should project");

    assert_eq!(payloads.len(), 1);

    let payload = &payloads[0];
    assert_eq!(payload.chain_id, CHAIN_ID);
    assert_eq!(payload.account_id, "account:alice");
    assert_eq!(payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
    assert_eq!(payload.balance_minor, "100");
    assert_eq!(payload.held_minor, "0");
    assert_eq!(payload.available_minor, "100");
    assert_eq!(payload.account_sequence, 1);
    assert_eq!(payload.receipt_root, receipt_root);
    assert_eq!(payload.holds_root, holds_root);
    assert_eq!(payload.permissions_root.as_ref(), Some(&permissions_root));
    assert_eq!(payload.updated_at_epoch, "epoch_0001");
}

#[test]
fn active_hold_projection_requires_explicit_policy_epoch_context_and_copies_it() {
    let hold = hold_id('a');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");
    let hold_record = commit_open_hold(
        &mut state,
        '2',
        "account:alice",
        Some("account:merchant"),
        "40",
        &hold,
        4,
        20,
    );

    let snapshot = state
        .state_snapshot()
        .expect("valid hold state should snapshot");

    let missing = snapshot
        .project_active_hold_leaf_payloads(&[])
        .expect_err("active-hold projection must not invent policy or epoch context");

    assert_eq!(
        missing,
        QuickChainLeafProjectionError::MissingActiveHoldContext {
            hold_id: hold.clone(),
        }
    );

    let policy_hash = test_content_id("projection-boundary/hold/policy");

    let contexts = vec![QuickChainActiveHoldLeafProjectionContext::new(
        hold.clone(),
        "paid_site_visit",
        QuickChainEpochBinding::new(4, "epoch_0004"),
        QuickChainEpochBinding::new(20, "epoch_0020"),
        policy_hash.clone(),
    )];

    let payloads = snapshot
        .project_active_hold_leaf_payloads(&contexts)
        .expect("explicit reviewed active-hold context should project");

    assert_eq!(payloads.len(), 1);

    let payload = &payloads[0];
    assert_eq!(payload.chain_id, CHAIN_ID);
    assert_eq!(payload.hold_id, hold);
    assert_eq!(payload.account_id, "account:alice");
    assert_eq!(
        payload.counterparty_account_id.as_deref(),
        Some("account:merchant")
    );
    assert_eq!(payload.amount_minor, "40");
    assert_eq!(payload.purpose, "paid_site_visit");
    assert_eq!(payload.created_at_epoch, "epoch_0004");
    assert_eq!(payload.expires_at_epoch, "epoch_0020");
    assert_eq!(payload.status, QuickChainActiveHoldStatusV1::Open);
    assert_eq!(payload.operation_id, hold_record.intent().operation_id);
    assert_eq!(
        payload.idempotency_key,
        hold_record.intent().idempotency_key
    );
    assert_eq!(payload.policy_hash, policy_hash);
}

#[test]
fn operation_hash_projection_copies_explicit_policy_context_without_computing_hashes() {
    let operation = intent(
        '3',
        "idem:projection-boundary:transfer",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "12",
        None,
        3_000,
    );

    let policy_hash = test_content_id("projection-boundary/operation/policy");
    let chain_params_hash = test_content_id("projection-boundary/operation/chain-params");

    let context = QuickChainOperationHashProjectionContext::new(
        operation.operation_id.clone(),
        "paid_transfer",
        Some("budget_projection_boundary_001".to_string()),
        policy_hash.clone(),
        chain_params_hash.clone(),
    );

    let payload = project_operation_hash_payload(&operation, &context)
        .expect("explicit operation context should project");

    assert_eq!(payload.chain_id, CHAIN_ID);
    assert_eq!(payload.operation_id, operation.operation_id);
    assert_eq!(payload.op_class, QuickChainOperationClassV1::Transfer);
    assert_eq!(payload.actor_account_id, "account:alice");
    assert_eq!(
        payload.counterparty_account_id.as_deref(),
        Some("account:bob")
    );
    assert_eq!(payload.amount_minor, "12");
    assert_eq!(payload.purpose, "paid_transfer");
    assert_eq!(
        payload.session_budget_id.as_deref(),
        Some("budget_projection_boundary_001")
    );
    assert_eq!(payload.policy_hash, policy_hash);
    assert_eq!(payload.chain_params_hash, chain_params_hash);
    assert_eq!(payload.idempotency_scope_account_id, "account:alice");
    assert_eq!(payload.idempotency_scope_operation_family, "transfer");
    assert_eq!(payload.idempotency_key, "idem:projection-boundary:transfer");
}

#[test]
fn receipt_hash_projection_copies_explicit_operation_and_continuity_context() {
    let operation = intent(
        '4',
        "idem:projection-boundary:receipt-transfer",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "25",
        None,
        4_000,
    );

    let record = QuickChainCommittedOperationRecord::new(
        operation,
        "tx:roc:projection-boundary:receipt-transfer",
        7,
        10,
        11,
    )
    .expect("valid committed test record");

    let operation_hash = test_content_id("projection-boundary/receipt/operation-hash");
    let previous_ledger_root = test_content_id("projection-boundary/receipt/previous-ledger-root");
    let new_ledger_root = test_content_id("projection-boundary/receipt/new-ledger-root");

    let context = QuickChainReceiptHashProjectionContext::new(
        record.intent().operation_id.clone(),
        operation_hash.clone(),
        "paid_transfer",
        Some("budget_projection_boundary_002".to_string()),
        previous_ledger_root.clone(),
        new_ledger_root.clone(),
        1_888_000_000_000,
    );

    let payload = project_receipt_hash_payload(&record, &context)
        .expect("explicit receipt context should project");

    assert_eq!(payload.chain_id, CHAIN_ID);
    assert_eq!(payload.txid, "tx:roc:projection-boundary:receipt-transfer");
    assert_eq!(payload.operation_id, record.intent().operation_id);
    assert_eq!(payload.operation_hash, operation_hash);
    assert_eq!(payload.op, "paid_transfer");
    assert_eq!(payload.op_class, QuickChainOperationClassV1::Transfer);
    assert_eq!(payload.from_account_id.as_deref(), Some("account:alice"));
    assert_eq!(payload.to_account_id.as_deref(), Some("account:bob"));
    assert_eq!(payload.asset, QUICKCHAIN_HASH_PAYLOAD_ASSET_ROC);
    assert_eq!(payload.amount_minor, "25");
    assert_eq!(payload.account_sequence, 7);
    assert_eq!(
        payload.session_budget_id.as_deref(),
        Some("budget_projection_boundary_002")
    );
    assert_eq!(
        payload.idempotency_key,
        "idem:projection-boundary:receipt-transfer"
    );
    assert_eq!(payload.ledger_seq_start, 10);
    assert_eq!(payload.ledger_seq_end, 11);
    assert_eq!(payload.previous_ledger_root, previous_ledger_root);
    assert_eq!(payload.new_ledger_root, new_ledger_root);

    // Receipt production time is explicit backend context. It is not derived
    // from the operation intent's produced_at_ms value.
    assert_eq!(payload.produced_at_ms, 1_888_000_000_000);
}
