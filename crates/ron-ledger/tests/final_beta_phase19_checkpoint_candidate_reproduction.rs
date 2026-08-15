#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — FINAL_BETA Phase 19 independent committee-checkpoint reproduction tests.
//! RO:WHY — Multiple reconstruction contexts must independently derive identical reviewed
//!          roots, canonical checkpoint bytes, and checkpoint hash before validator signing.
//! RO:INTERACTS — ron-ledger tree material, typed receipt roots, checkpoint candidate builder,
//!                and ron-proto committee checkpoint commitments.
//! RO:INVARIANTS — contexts do not share a precomputed candidate hash; unordered reviewed
//!                 material converges deterministically; changed material changes the candidate.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — deterministic local proof only; no validator signing, quorum, finality,
//!               wallet mutation, ledger mutation, payout, bridge, or external settlement.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    build_committee_checkpoint_candidate, build_tree_material_batch,
    compute_ledger_sequence_receipt_root, compute_tree_root_from_batch,
    QuickChainCommitteeCheckpointCandidate, QuickChainCommitteeCheckpointCandidateContext,
    QuickChainTreeMaterialProjectionItem,
};
use ron_proto::{
    quickchain::{
        QuickChainOperationClassV1, QuickChainReceiptHashPayloadV1, QuickChainTreeMaterialKindV1,
        QuickChainTreeRootV1, QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA,
    },
    ContentId, QuickChainConservationV1, QuickChainSupplyDeltaV1,
    QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
};

const CHAIN_ID: &str = "ron-devnet";
const PREVIOUS_EPOCH_ID: &str = "epoch_phase19_candidate_previous";
const EPOCH_ID: &str = "epoch_phase19_candidate_reproduction";

const ALICE_SORT_KEY: &str = "6163636f756e743a616c69636500726f63";
const BOB_SORT_KEY: &str = "6163636f756e743a626f6200726f63";
const CAROL_SORT_KEY: &str = "6163636f756e743a6361726f6c00726f63";

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture ContentId must parse")
}

fn state_item(sort_key: &str, label: &str) -> QuickChainTreeMaterialProjectionItem {
    QuickChainTreeMaterialProjectionItem::new(
        sort_key,
        QUICKCHAIN_ACCOUNT_LEAF_PAYLOAD_SCHEMA,
        cid(label),
    )
}

fn state_root(
    epoch_id: &str,
    reverse_input_order: bool,
    mutate_current_carol: bool,
    current: bool,
) -> QuickChainTreeRootV1 {
    let prefix = if current { "current" } else { "previous" };

    let carol_label = if current && mutate_current_carol {
        "current-carol-leaf-mutated"
    } else if current {
        "current-carol-leaf"
    } else {
        "previous-carol-leaf"
    };

    let mut items = vec![
        state_item(ALICE_SORT_KEY, &format!("{prefix}-alice-leaf")),
        state_item(BOB_SORT_KEY, &format!("{prefix}-bob-leaf")),
        state_item(CAROL_SORT_KEY, carol_label),
    ];

    if reverse_input_order {
        items.reverse();
    }

    let batch = build_tree_material_batch(
        CHAIN_ID,
        epoch_id,
        QuickChainTreeMaterialKindV1::State,
        items,
    )
    .expect("independent state material must build");

    compute_tree_root_from_batch(&batch).expect("independent state root must compute")
}

fn receipt(label: char, ledger_seq_start: u64, txid: &str) -> QuickChainReceiptHashPayloadV1 {
    let payload = QuickChainReceiptHashPayloadV1 {
        schema: QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        txid: txid.to_owned(),
        operation_id: format!("op_{}", label.to_string().repeat(32)),
        operation_hash: cid(&format!("operation-{label}")),
        op: "issue".to_owned(),
        op_class: QuickChainOperationClassV1::Issue,
        from_account_id: None,
        to_account_id: Some(format!("acct_phase19_{label}")),
        asset: "roc".to_owned(),
        amount_minor: "10".to_owned(),
        account_sequence: ledger_seq_start,
        hold_id: None,
        session_budget_id: None,
        idempotency_key: format!("idem-phase19-{label}"),
        ledger_seq_start,
        ledger_seq_end: ledger_seq_start,
        previous_ledger_root: cid(&format!("previous-ledger-{label}")),
        new_ledger_root: cid(&format!("new-ledger-{label}")),
        produced_at_ms: 1_800_000_000_000 + ledger_seq_start,
    };

    payload.validate().expect("receipt fixture must validate");

    payload
}

fn build_node_candidate(
    reverse_input_order: bool,
    mutate_current_carol: bool,
) -> QuickChainCommitteeCheckpointCandidate {
    let previous_state_root = state_root(PREVIOUS_EPOCH_ID, reverse_input_order, false, false);

    let new_state_root = state_root(EPOCH_ID, reverse_input_order, mutate_current_carol, true);

    let mut receipts = vec![
        receipt('a', 101, "tx_phase19_a"),
        receipt('b', 102, "tx_phase19_b"),
        receipt('c', 103, "tx_phase19_c"),
    ];

    if reverse_input_order {
        receipts.reverse();
    }

    let receipt_root = compute_ledger_sequence_receipt_root(CHAIN_ID, EPOCH_ID, &receipts)
        .expect("typed receipt root must compute independently");

    build_committee_checkpoint_candidate(QuickChainCommitteeCheckpointCandidateContext {
        chain_id: CHAIN_ID.to_owned(),
        height: 19,
        epoch_id: EPOCH_ID.to_owned(),
        execution_spec_version: "quickchain-execution-v1".to_owned(),
        previous_checkpoint_hash: cid("phase19-previous-checkpoint"),
        previous_state_root,
        new_state_root,
        receipt_root,
        accounting_snapshot_root: cid("phase19-accounting-snapshot"),
        reward_manifest_root: cid("phase19-reward-manifest"),
        data_availability_root: cid("phase19-data-availability"),
        policy_hash: cid("phase19-policy"),
        validator_set_hash: cid("phase19-validator-set"),
        chain_params_hash: cid("phase19-chain-params"),
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
    })
    .expect("independent checkpoint candidate must build")
}

fn independent_checkpoint_hash(canonical_payload_bytes: &[u8]) -> ContentId {
    let mut hasher = blake3::Hasher::new();

    hasher.update(QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());
    hasher.update(&[0]);
    hasher.update(canonical_payload_bytes);

    format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .expect("independent checkpoint hash must parse")
}

#[test]
fn independent_contexts_reproduce_identical_roots_payload_bytes_and_hash() {
    let node_a = build_node_candidate(false, false);

    let node_b = build_node_candidate(true, false);

    assert_eq!(
        &node_a.payload().previous_state_root,
        &node_b.payload().previous_state_root,
    );

    assert_eq!(
        &node_a.payload().new_state_root,
        &node_b.payload().new_state_root,
    );

    assert_eq!(
        &node_a.payload().receipt_root,
        &node_b.payload().receipt_root,
    );

    assert_eq!(node_a.payload(), node_b.payload(),);

    assert_eq!(
        node_a.canonical_payload_bytes(),
        node_b.canonical_payload_bytes(),
    );

    assert_eq!(node_a.checkpoint_hash(), node_b.checkpoint_hash(),);
}

#[test]
fn each_context_rederives_hash_from_its_own_canonical_payload_bytes() {
    let node_a = build_node_candidate(false, false);

    let node_b = build_node_candidate(true, false);

    let independently_hashed_a = independent_checkpoint_hash(node_a.canonical_payload_bytes());

    let independently_hashed_b = independent_checkpoint_hash(node_b.canonical_payload_bytes());

    assert_eq!(&independently_hashed_a, node_a.checkpoint_hash(),);

    assert_eq!(&independently_hashed_b, node_b.checkpoint_hash(),);

    assert_eq!(independently_hashed_a, independently_hashed_b,);
}

#[test]
fn changed_reviewed_state_material_prevents_false_candidate_convergence() {
    let baseline = build_node_candidate(false, false);

    let changed = build_node_candidate(true, true);

    assert_ne!(
        &baseline.payload().new_state_root,
        &changed.payload().new_state_root,
    );

    assert_ne!(
        baseline.canonical_payload_bytes(),
        changed.canonical_payload_bytes(),
    );

    assert_ne!(baseline.checkpoint_hash(), changed.checkpoint_hash(),);
}
