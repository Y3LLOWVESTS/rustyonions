#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — FINAL_BETA Phase 19 tests for deterministic unsigned committee checkpoint candidates.
//! RO:WHY — Candidate convergence must be proven before validator signing, quorum acceptance, or finality work begins.
//! RO:INTERACTS — ron-ledger state/receipt roots and ron-proto committee checkpoint commitments.
//! RO:INVARIANTS — same reviewed inputs produce identical bytes/hash; typed receipt provenance is required; commitment changes alter the hash.
//! RO:SECURITY — no signing, quorum, finality, wallet mutation, ledger mutation, payout, bridge, or external settlement.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_committee_checkpoint_candidate, build_tree_material_batch,
    compute_ledger_sequence_receipt_root, compute_tree_root_from_batch,
    QuickChainCommitteeCheckpointCandidateContext, QuickChainCommitteeCheckpointCandidateError,
    QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        QuickChainConservationV1, QuickChainOperationClassV1, QuickChainReceiptHashPayloadV1,
        QuickChainSupplyDeltaV1, QuickChainTreeMaterialKindV1, QuickChainTreeRootV1,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const PREVIOUS_EPOCH_ID: &str = "epoch_phase18";
const EPOCH_ID: &str = "epoch_phase19";

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture ContentId must parse")
}

fn state_root(epoch_id: &str, first_label: &str, reverse_input: bool) -> QuickChainTreeRootV1 {
    let first = QuickChainTreeMaterialProjectionItem::new(
        "00",
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        cid(first_label),
    );

    let second = QuickChainTreeMaterialProjectionItem::new(
        "01",
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        cid("state-second"),
    );

    let items = if reverse_input {
        vec![second, first]
    } else {
        vec![first, second]
    };

    let batch = build_tree_material_batch(
        CHAIN_ID,
        epoch_id,
        QuickChainTreeMaterialKindV1::State,
        items,
    )
    .expect("state material batch must validate");

    compute_tree_root_from_batch(&batch).expect("state root must compute")
}

fn non_state_root(epoch_id: &str) -> QuickChainTreeRootV1 {
    let batch = build_tree_material_batch(
        CHAIN_ID,
        epoch_id,
        QuickChainTreeMaterialKindV1::Accounting,
        vec![QuickChainTreeMaterialProjectionItem::new(
            "00",
            "quickchain.test-accounting.v1",
            cid("accounting-item"),
        )],
    )
    .expect("accounting material batch must validate");

    compute_tree_root_from_batch(&batch).expect("accounting root must compute")
}

fn receipt(label: char, ledger_seq_start: u64) -> QuickChainReceiptHashPayloadV1 {
    let payload = QuickChainReceiptHashPayloadV1 {
        schema: QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: CHAIN_ID.to_owned(),

        txid: format!("tx_phase19_{label}",),

        operation_id: format!("op_{}", label.to_string().repeat(32),),

        operation_hash: cid(&format!("operation-{label}",)),

        op: "issue".to_owned(),

        op_class: QuickChainOperationClassV1::Issue,

        from_account_id: None,

        to_account_id: Some(format!("acct_phase19_{label}",)),

        asset: "roc".to_owned(),

        amount_minor: "10".to_owned(),

        account_sequence: ledger_seq_start,

        hold_id: None,

        session_budget_id: None,

        idempotency_key: format!("idem-phase19-{label}",),

        ledger_seq_start,

        ledger_seq_end: ledger_seq_start,

        previous_ledger_root: cid(&format!("previous-{label}",)),

        new_ledger_root: cid(&format!("new-{label}",)),

        produced_at_ms: 1_800_000_000_000 + ledger_seq_start,
    };

    payload
        .validate()
        .expect("Phase 19 receipt fixture must validate");

    payload
}

fn context(reverse_state_input: bool) -> QuickChainCommitteeCheckpointCandidateContext {
    QuickChainCommitteeCheckpointCandidateContext {
        chain_id: CHAIN_ID.to_owned(),

        height: 19,

        epoch_id: EPOCH_ID.to_owned(),

        execution_spec_version: "quickchain-execution-v1".to_owned(),

        previous_checkpoint_hash: cid("previous-checkpoint"),

        previous_state_root: state_root(PREVIOUS_EPOCH_ID, "previous-state-first", false),

        new_state_root: state_root(EPOCH_ID, "new-state-first", reverse_state_input),

        receipt_root: compute_ledger_sequence_receipt_root(
            CHAIN_ID,
            EPOCH_ID,
            &[receipt('a', 41), receipt('b', 42)],
        )
        .expect("typed ledger-sequence receipt root must compute"),

        accounting_snapshot_root: cid("accounting-root"),

        reward_manifest_root: cid("reward-root"),

        data_availability_root: cid("data-availability-root"),

        policy_hash: cid("policy"),

        validator_set_hash: cid("validator-set"),

        chain_params_hash: cid("chain-params"),

        supply_delta: QuickChainSupplyDeltaV1 {
            issued_minor: "0".to_owned(),

            burned_minor: "0".to_owned(),

            net_minor: "0".to_owned(),
        },

        conservation: QuickChainConservationV1 {
            debits_minor: "100".to_owned(),

            credits_minor: "100".to_owned(),

            issue_exceptions_minor: "0".to_owned(),

            burn_exceptions_minor: "0".to_owned(),

            valid: true,
        },

        started_at_ms: 1_800_000_000_000,

        ended_at_ms: 1_800_000_060_000,

        produced_at_ms: 1_800_000_061_000,
    }
}

fn candidate_hash(context: QuickChainCommitteeCheckpointCandidateContext) -> ContentId {
    build_committee_checkpoint_candidate(context)
        .expect("candidate must build")
        .checkpoint_hash()
        .clone()
}

#[test]
fn same_inputs_produce_identical_payload_bytes_and_hash() {
    let first = build_committee_checkpoint_candidate(context(false)).expect("first candidate");

    let second = build_committee_checkpoint_candidate(context(false)).expect("second candidate");

    assert_eq!(first.payload(), second.payload(),);

    assert_eq!(
        first.canonical_payload_bytes(),
        second.canonical_payload_bytes(),
    );

    assert_eq!(first.checkpoint_hash(), second.checkpoint_hash(),);
}

#[test]
fn reordered_state_material_converges_before_candidate_hashing() {
    let forward = build_committee_checkpoint_candidate(context(false)).expect("forward candidate");

    let reversed = build_committee_checkpoint_candidate(context(true)).expect("reversed candidate");

    assert_eq!(
        forward.payload().new_state_root,
        reversed.payload().new_state_root,
    );

    assert_eq!(forward.checkpoint_hash(), reversed.checkpoint_hash(),);
}

#[test]
fn candidate_consumes_typed_ledger_sequence_receipt_root() {
    let expected = context(false).receipt_root.root().root_hash.clone();

    let candidate = build_committee_checkpoint_candidate(context(false)).expect("candidate");

    assert_eq!(candidate.payload().receipt_root, expected,);
}

#[test]
fn commitment_changes_change_candidate_hash() {
    let baseline = candidate_hash(context(false));

    let mut changed = context(false);

    changed.new_state_root = state_root(EPOCH_ID, "different-state-first", false);

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.receipt_root =
        compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &[receipt('c', 43)])
            .expect("changed typed receipt root");

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.accounting_snapshot_root = cid("different-accounting-root");

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.reward_manifest_root = cid("different-reward-root");

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.data_availability_root = cid("different-da-root");

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.validator_set_hash = cid("different-validator-set");

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.policy_hash = cid("different-policy");

    assert_ne!(baseline, candidate_hash(changed,),);

    let mut changed = context(false);

    changed.chain_params_hash = cid("different-chain-params");

    assert_ne!(baseline, candidate_hash(changed,),);
}

#[test]
fn invalid_state_root_kind_rejects_before_candidate_return() {
    let mut input = context(false);

    input.new_state_root = non_state_root(EPOCH_ID);

    let error = build_committee_checkpoint_candidate(input)
        .expect_err("non-state root must reject before candidate return");

    assert_eq!(
        error,
        QuickChainCommitteeCheckpointCandidateError::WrongRootKind {
            field: "new_state_root",

            expected: "state",
        },
    );
}

#[test]
fn current_root_epoch_mismatch_rejects_before_candidate_return() {
    let mut input = context(false);

    input.new_state_root = state_root("epoch_wrong", "new-state-first", false);

    let error = build_committee_checkpoint_candidate(input)
        .expect_err("current state root epoch mismatch must reject");

    assert_eq!(
        error,
        QuickChainCommitteeCheckpointCandidateError::RootEpochMismatch {
            field: "new_state_root",
        },
    );
}

#[test]
fn candidate_payload_contains_no_signature_or_finality_fields() {
    let candidate = build_committee_checkpoint_candidate(context(false)).expect("candidate");

    let value =
        serde_json::to_value(candidate.payload()).expect("candidate payload must serialize");

    let object = value
        .as_object()
        .expect("candidate payload must be an object");

    assert!(!object.contains_key("signatures",));

    assert!(!object.contains_key("validator_signatures",));

    assert!(!object.contains_key("quorum",));

    assert!(!object.contains_key("finality",));

    assert!(!object.contains_key("finalized",));
}
