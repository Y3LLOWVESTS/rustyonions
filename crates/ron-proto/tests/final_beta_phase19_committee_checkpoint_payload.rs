//! RO:WHAT — FINAL_BETA Phase 19 protocol tests for unsigned committee checkpoint commitments.
//! RO:WHY — A checkpoint reviewed by validators must commit the validator set and DA root
//!          without changing the frozen Phase-0 unsigned-checkpoint payload.
//! RO:INTERACTS — QuickChain canonical JSON, checkpoint hash domain, deterministic roots,
//!                validator-set commitment, and data-availability commitment.
//! RO:INVARIANTS — both commitments are required; signatures are structurally absent;
//!                 immutable commitment changes alter canonical bytes/hash.
//! RO:SECURITY — DTO/hash tests only; no validator signing, quorum acceptance, ledger mutation,
//!               checkpoint finalization, wallet mutation, payout, or external settlement.

use ron_proto::{
    to_canonical_json_vec, ContentId, QuickChainCanonicalEncodingV1,
    QuickChainCommitteeCheckpointPayloadV1, QuickChainConservationV1,
    QuickChainReceiptRootSchemeV1, QuickChainSettlementModeV1, QuickChainStateRootSchemeV1,
    QuickChainSupplyDeltaV1, QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
    QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
};
use serde_json::Value;

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture ContentId must parse")
}

fn valid_payload() -> QuickChainCommitteeCheckpointPayloadV1 {
    QuickChainCommitteeCheckpointPayloadV1 {
        schema: QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),

        version: QUICKCHAIN_DTO_VERSION,

        chain_id: "roc-dev".to_owned(),

        height: 19,

        epoch_id: "epoch:phase19".to_owned(),

        execution_spec_version: "quickchain-execution-v1".to_owned(),

        previous_checkpoint_hash: cid("previous-checkpoint"),

        previous_state_root: cid("previous-state"),

        new_state_root: cid("new-state"),

        receipt_root: cid("receipt-root"),

        accounting_snapshot_root: cid("accounting-root"),

        reward_manifest_root: cid("reward-root"),

        data_availability_root: cid("data-availability-root"),

        policy_hash: cid("policy"),

        validator_set_hash: cid("validator-set"),

        chain_params_hash: cid("chain-params"),

        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,

        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,

        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,

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

        settlement_mode: QuickChainSettlementModeV1::LocalRoot,

        started_at_ms: 1_800_000_000_000,

        ended_at_ms: 1_800_000_060_000,

        produced_at_ms: 1_800_000_061_000,
    }
}

fn checkpoint_hash(payload: &QuickChainCommitteeCheckpointPayloadV1) -> ContentId {
    let canonical = to_canonical_json_vec(payload).expect("committee checkpoint must canonicalize");

    let mut hasher = blake3::Hasher::new();

    hasher.update(QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());

    hasher.update(&[0]);

    hasher.update(&canonical);

    format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .expect("committee checkpoint hash must parse")
}

#[test]
fn valid_committee_checkpoint_payload_validates() {
    valid_payload()
        .validate()
        .expect("valid committee checkpoint payload must validate");
}

#[test]
fn canonical_payload_commits_validator_set_and_data_availability_roots() {
    let payload = valid_payload();

    let value = serde_json::to_value(&payload).expect("committee payload must serialize");

    assert_eq!(
        value["schema"],
        QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA,
    );

    assert_eq!(
        value["validator_set_hash"],
        serde_json::to_value(&payload.validator_set_hash,)
            .expect("validator-set hash must serialize",),
    );

    assert_eq!(
        value["data_availability_root"],
        serde_json::to_value(&payload.data_availability_root,).expect("DA root must serialize",),
    );
}

#[test]
fn validator_signatures_are_structurally_excluded_from_hash_payload() {
    let payload = valid_payload();

    let mut value = serde_json::to_value(payload).expect("committee payload must serialize");

    let object = value
        .as_object_mut()
        .expect("committee payload JSON must be an object");

    object.insert("signatures".to_owned(), Value::Array(Vec::new()));

    let error = serde_json::from_value::<QuickChainCommitteeCheckpointPayloadV1>(value)
        .expect_err("unknown signatures field must reject");

    assert!(error.to_string().contains("unknown field",));
}

#[test]
fn validator_set_change_changes_canonical_bytes_and_checkpoint_hash() {
    let original = valid_payload();

    let mut changed = original.clone();

    changed.validator_set_hash = cid("another-validator-set");

    changed
        .validate()
        .expect("changed validator-set commitment remains structurally valid");

    let original_bytes =
        to_canonical_json_vec(&original).expect("original payload must canonicalize");

    let changed_bytes = to_canonical_json_vec(&changed).expect("changed payload must canonicalize");

    assert_ne!(original_bytes, changed_bytes,);

    assert_ne!(checkpoint_hash(&original,), checkpoint_hash(&changed,),);
}

#[test]
fn data_availability_change_changes_canonical_bytes_and_checkpoint_hash() {
    let original = valid_payload();

    let mut changed = original.clone();

    changed.data_availability_root = cid("another-da-root");

    changed
        .validate()
        .expect("changed DA commitment remains structurally valid");

    assert_ne!(
        to_canonical_json_vec(&original,).expect("original payload must canonicalize",),
        to_canonical_json_vec(&changed,).expect("changed payload must canonicalize",),
    );

    assert_ne!(checkpoint_hash(&original,), checkpoint_hash(&changed,),);
}

#[test]
fn checkpoint_hash_is_deterministic_under_existing_checkpoint_domain() {
    let payload = valid_payload();

    let first = checkpoint_hash(&payload);

    let second = checkpoint_hash(&payload);

    assert_eq!(first, second,);

    assert_eq!(
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
        "quickchain.checkpoint.v1",
    );
}

#[test]
fn reserved_non_json_canonical_encoding_rejects() {
    let mut payload = valid_payload();

    payload.canonical_encoding = QuickChainCanonicalEncodingV1::CborV1;

    let error = payload
        .validate()
        .expect_err("reserved CBOR encoding must reject");

    assert!(error.to_string().contains("canonical_encoding",));
}

#[test]
fn zero_height_rejects_before_committee_checkpoint_use() {
    let mut payload = valid_payload();

    payload.height = 0;

    let error = payload
        .validate()
        .expect_err("zero checkpoint height must reject");

    assert!(error.to_string().contains("height",));
}
