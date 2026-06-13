#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests proving QuickChain live execution and accepted replay feed identical projection payloads.
//! RO:WHY — ECON/RES: replayed ledger state must reproduce the same pre-root snapshot, leaf, operation, and receipt payload inputs.
//! RO:INTERACTS — QuickChainAtomicState, accepted replay, state snapshots, leaf projection, and hash-payload projection adapters.
//! RO:INVARIANTS — replay equality; explicit context only; no serialization, hashing, roots, clocks, IO, persistence, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture content IDs are inert reviewed inputs, not production roots, receipts, proofs, or spend authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    project_operation_hash_payload, project_receipt_hash_payload, QuickChainAcceptedOperation,
    QuickChainAccountLeafProjectionContext, QuickChainActiveHoldLeafProjectionContext,
    QuickChainAtomicState, QuickChainCommittedOperationRecord, QuickChainEpochBinding,
    QuickChainHoldEpochInput, QuickChainOperationHashProjectionContext,
    QuickChainReceiptHashProjectionContext, QuickChainStateSnapshot, QuickChainSupplyDecision,
};
use ron_proto::{
    quickchain::{
        QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
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

/// Produce a genuine BLAKE3 content identifier for an inert test label.
///
/// These IDs test explicit projection plumbing only. They are not production
/// roots, operation hashes, receipt hashes, policy hashes, or vectors.
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
fn commit_balance(
    state: &mut QuickChainAtomicState,
    accepted: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    supply_decision: QuickChainSupplyDecision,
    produced_at_ms: u64,
) {
    let submitted = intent(
        operation_hex_digit,
        idempotency_key,
        op_class,
        actor,
        counterparty,
        amount_minor,
        None,
        produced_at_ms,
    );

    let outcome = state
        .execute_balance_operation(
            &submitted,
            supply_decision,
            format!("tx:roc:projection-replay:{operation_hex_digit}"),
        )
        .expect("balance operation should commit");

    accepted.push(QuickChainAcceptedOperation::balance(
        outcome.record().clone(),
        supply_decision,
    ));
}

#[allow(clippy::too_many_arguments)]
fn commit_hold(
    state: &mut QuickChainAtomicState,
    accepted: &mut Vec<QuickChainAcceptedOperation>,
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: &str,
    epoch_input: QuickChainHoldEpochInput,
    produced_at_ms: u64,
) {
    let submitted = intent(
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
        .execute_hold_operation(
            &submitted,
            epoch_input,
            format!("tx:roc:projection-replay:{operation_hex_digit}"),
        )
        .expect("hold operation should commit");

    accepted.push(QuickChainAcceptedOperation::hold(
        outcome.record().clone(),
        epoch_input,
    ));
}

fn accepted_records(
    accepted: &[QuickChainAcceptedOperation],
) -> Vec<QuickChainCommittedOperationRecord> {
    accepted
        .iter()
        .map(QuickChainAcceptedOperation::record)
        .cloned()
        .collect()
}

fn account_contexts(
    snapshot: &QuickChainStateSnapshot,
) -> Vec<QuickChainAccountLeafProjectionContext> {
    // Deliberately reverse caller context order. Projection order must come
    // from the deterministic snapshot, not from context input order.
    snapshot
        .accounts()
        .iter()
        .rev()
        .map(|account| {
            QuickChainAccountLeafProjectionContext::new(
                account.account_id(),
                test_content_id(&format!("{}/receipt-tree", account.account_id())),
                test_content_id(&format!("{}/hold-tree", account.account_id())),
                Some(test_content_id(&format!(
                    "{}/permissions-tree",
                    account.account_id()
                ))),
                format!("epoch_{:04}", account.account_sequence()),
            )
        })
        .collect()
}

fn active_hold_contexts(
    snapshot: &QuickChainStateSnapshot,
) -> Vec<QuickChainActiveHoldLeafProjectionContext> {
    // Deliberately reverse caller context order for the same reason as account
    // contexts above.
    snapshot
        .active_holds()
        .iter()
        .rev()
        .map(|hold| {
            QuickChainActiveHoldLeafProjectionContext::new(
                hold.hold_id(),
                "paid_storage",
                QuickChainEpochBinding::new(
                    hold.created_at_epoch_number(),
                    epoch_id(hold.created_at_epoch_number()),
                ),
                QuickChainEpochBinding::new(
                    hold.expires_at_epoch_number(),
                    epoch_id(hold.expires_at_epoch_number()),
                ),
                test_content_id(&format!("{}/policy", hold.hold_id())),
            )
        })
        .collect()
}

fn epoch_id(epoch_number: u64) -> String {
    format!("epoch_{epoch_number:04}")
}

fn operation_projection_context(
    record: &QuickChainCommittedOperationRecord,
) -> QuickChainOperationHashProjectionContext {
    QuickChainOperationHashProjectionContext::new(
        &record.intent().operation_id,
        projection_purpose(record.intent()),
        session_budget_id(record.intent()),
        test_content_id(&format!("{}/policy", record.intent().operation_id)),
        test_content_id(&format!("{}/chain-params", record.intent().operation_id)),
    )
}

fn receipt_projection_context(
    record: &QuickChainCommittedOperationRecord,
) -> QuickChainReceiptHashProjectionContext {
    QuickChainReceiptHashProjectionContext::new(
        &record.intent().operation_id,
        test_content_id(&format!("{}/operation-hash", record.intent().operation_id)),
        receipt_op(record.intent()),
        session_budget_id(record.intent()),
        test_content_id(&format!(
            "{}/previous-ledger-root",
            record.intent().operation_id
        )),
        test_content_id(&format!("{}/new-ledger-root", record.intent().operation_id)),
        1_900_000_000_000 + record.ledger_sequence_start(),
    )
}

fn projection_purpose(intent: &QuickChainOperationIntentV1) -> &'static str {
    match intent.op_class {
        QuickChainOperationClassV1::Issue => "issuer_funding",
        QuickChainOperationClassV1::Transfer => "paid_transfer",
        QuickChainOperationClassV1::Burn => "supply_burn",
        QuickChainOperationClassV1::HoldOpen => "paid_storage",
        QuickChainOperationClassV1::HoldCapture => "hold_capture",
        QuickChainOperationClassV1::HoldRelease => "hold_release",
        QuickChainOperationClassV1::HoldExpire => "hold_expire",
        _ => panic!("unsupported future QuickChain operation class in test fixture"),
    }
}

fn receipt_op(intent: &QuickChainOperationIntentV1) -> &'static str {
    match intent.op_class {
        QuickChainOperationClassV1::Issue => "issue",
        QuickChainOperationClassV1::Transfer => "paid_transfer",
        QuickChainOperationClassV1::Burn => "burn",
        QuickChainOperationClassV1::HoldOpen => "paid_storage_hold",
        QuickChainOperationClassV1::HoldCapture => "hold_capture",
        QuickChainOperationClassV1::HoldRelease => "hold_release",
        QuickChainOperationClassV1::HoldExpire => "hold_expire",
        _ => panic!("unsupported future QuickChain operation class in test fixture"),
    }
}

fn session_budget_id(intent: &QuickChainOperationIntentV1) -> Option<String> {
    match intent.op_class {
        QuickChainOperationClassV1::Transfer
        | QuickChainOperationClassV1::HoldOpen
        | QuickChainOperationClassV1::HoldCapture => {
            Some(format!("budget_{}", intent.operation_id))
        }

        QuickChainOperationClassV1::Issue
        | QuickChainOperationClassV1::Burn
        | QuickChainOperationClassV1::HoldRelease
        | QuickChainOperationClassV1::HoldExpire => None,

        _ => panic!("unsupported future QuickChain operation class in test fixture"),
    }
}

#[test]
fn mixed_live_history_and_accepted_replay_project_identically() {
    let alice_hold = hold_id('a');
    let bob_hold = hold_id('b');
    let carol_hold = hold_id('c');

    let mut live = QuickChainAtomicState::new();
    let mut accepted = Vec::new();

    commit_balance(
        &mut live,
        &mut accepted,
        '1',
        "idem:projection-replay:issue:alice",
        QuickChainOperationClassV1::Issue,
        "account:alice",
        None,
        "300",
        QuickChainSupplyDecision::IssueApproved,
        1_000,
    );

    commit_balance(
        &mut live,
        &mut accepted,
        '2',
        "idem:projection-replay:issue:bob",
        QuickChainOperationClassV1::Issue,
        "account:bob",
        None,
        "160",
        QuickChainSupplyDecision::IssueApproved,
        1_001,
    );

    commit_balance(
        &mut live,
        &mut accepted,
        '3',
        "idem:projection-replay:issue:carol",
        QuickChainOperationClassV1::Issue,
        "account:carol",
        None,
        "90",
        QuickChainSupplyDecision::IssueApproved,
        1_002,
    );

    commit_balance(
        &mut live,
        &mut accepted,
        '4',
        "idem:projection-replay:transfer:alice-bob",
        QuickChainOperationClassV1::Transfer,
        "account:alice",
        Some("account:bob"),
        "40",
        QuickChainSupplyDecision::NoSupplyChange,
        1_003,
    );

    commit_hold(
        &mut live,
        &mut accepted,
        '5',
        "idem:projection-replay:hold-open:alice",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:merchant"),
        "80",
        &alice_hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 1,
            expires_at_epoch: 10,
        },
        1_004,
    );

    commit_hold(
        &mut live,
        &mut accepted,
        '6',
        "idem:projection-replay:hold-open:bob",
        QuickChainOperationClassV1::HoldOpen,
        "account:bob",
        Some("account:merchant"),
        "30",
        &bob_hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 2,
            expires_at_epoch: 20,
        },
        1_005,
    );

    commit_hold(
        &mut live,
        &mut accepted,
        '7',
        "idem:projection-replay:hold-capture:alice",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:merchant"),
        "60",
        &alice_hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 3 },
        1_006,
    );

    commit_hold(
        &mut live,
        &mut accepted,
        '8',
        "idem:projection-replay:hold-release:bob",
        QuickChainOperationClassV1::HoldRelease,
        "account:bob",
        None,
        "30",
        &bob_hold,
        QuickChainHoldEpochInput::Terminal { current_epoch: 4 },
        1_007,
    );

    commit_hold(
        &mut live,
        &mut accepted,
        '9',
        "idem:projection-replay:hold-open:carol",
        QuickChainOperationClassV1::HoldOpen,
        "account:carol",
        Some("account:storage-provider"),
        "25",
        &carol_hold,
        QuickChainHoldEpochInput::Open {
            created_at_epoch: 5,
            expires_at_epoch: 30,
        },
        1_008,
    );

    commit_balance(
        &mut live,
        &mut accepted,
        'a',
        "idem:projection-replay:burn:bob",
        QuickChainOperationClassV1::Burn,
        "account:bob",
        None,
        "10",
        QuickChainSupplyDecision::BurnApproved,
        1_009,
    );

    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations(&accepted)
        .expect("accepted mixed history should rebuild");

    assert_eq!(rebuilt, live);
    assert_eq!(rebuilt.operation_count(), 10);
    assert_eq!(rebuilt.next_ledger_sequence(), 13);
    assert_eq!(rebuilt.current_supply_minor(), 540);
    assert_eq!(rebuilt.balance_minor("account:alice"), 200);
    assert_eq!(rebuilt.balance_minor("account:bob"), 190);
    assert_eq!(rebuilt.balance_minor("account:carol"), 90);
    assert_eq!(rebuilt.balance_minor("account:merchant"), 60);
    assert_eq!(rebuilt.held_minor("account:carol"), 25);
    assert!(rebuilt.active_hold(&carol_hold).is_some());
    assert!(rebuilt.terminal_hold(&alice_hold).is_some());
    assert!(rebuilt.terminal_hold(&bob_hold).is_some());

    let live_before_projection = live.clone();
    let rebuilt_before_projection = rebuilt.clone();

    let live_snapshot = live
        .state_snapshot()
        .expect("live state should snapshot after valid history");
    let rebuilt_snapshot = rebuilt
        .state_snapshot()
        .expect("rebuilt state should snapshot after valid history");

    assert_eq!(live_snapshot, rebuilt_snapshot);
    assert_eq!(live_snapshot.chain_id(), Some(CHAIN_ID));
    assert_eq!(live_snapshot.accounts().len(), 4);
    assert_eq!(live_snapshot.active_holds().len(), 1);

    let account_contexts = account_contexts(&live_snapshot);
    let live_account_payloads = live_snapshot
        .project_account_leaf_payloads(&account_contexts)
        .expect("live account snapshot should project");
    let rebuilt_account_payloads = rebuilt_snapshot
        .project_account_leaf_payloads(&account_contexts)
        .expect("rebuilt account snapshot should project");

    assert_eq!(live_account_payloads, rebuilt_account_payloads);

    let active_hold_contexts = active_hold_contexts(&live_snapshot);
    let live_active_hold_payloads = live_snapshot
        .project_active_hold_leaf_payloads(&active_hold_contexts)
        .expect("live active-hold snapshot should project");
    let rebuilt_active_hold_payloads = rebuilt_snapshot
        .project_active_hold_leaf_payloads(&active_hold_contexts)
        .expect("rebuilt active-hold snapshot should project");

    assert_eq!(live_active_hold_payloads, rebuilt_active_hold_payloads);
    assert_eq!(live_active_hold_payloads[0].hold_id, carol_hold);
    assert_eq!(live_active_hold_payloads[0].amount_minor, "25");

    let live_records = accepted_records(&accepted);
    let rebuilt_records: Vec<_> = live_records
        .iter()
        .map(|record| {
            rebuilt
                .committed_operation(&record.intent().operation_id)
                .expect("rebuilt state must expose committed operation")
                .clone()
        })
        .collect();

    assert_eq!(live_records, rebuilt_records);

    for (live_record, rebuilt_record) in live_records.iter().zip(rebuilt_records.iter()) {
        let operation_context = operation_projection_context(live_record);
        let live_operation_payload =
            project_operation_hash_payload(live_record.intent(), &operation_context)
                .expect("live operation intent should project");
        let rebuilt_operation_payload =
            project_operation_hash_payload(rebuilt_record.intent(), &operation_context)
                .expect("rebuilt operation intent should project");

        assert_eq!(live_operation_payload, rebuilt_operation_payload);

        let receipt_context = receipt_projection_context(live_record);
        let live_receipt_payload = project_receipt_hash_payload(live_record, &receipt_context)
            .expect("live committed record should project");
        let rebuilt_receipt_payload =
            project_receipt_hash_payload(rebuilt_record, &receipt_context)
                .expect("rebuilt committed record should project");

        assert_eq!(live_receipt_payload, rebuilt_receipt_payload);
    }

    assert_eq!(live, live_before_projection);
    assert_eq!(rebuilt, rebuilt_before_projection);
}
