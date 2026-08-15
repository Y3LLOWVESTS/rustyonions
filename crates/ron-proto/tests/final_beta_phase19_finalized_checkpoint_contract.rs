//! RO:WHAT — FINAL_BETA Phase 19 finalized checkpoint protocol-contract tests.
//! RO:WHY — Locks threshold, validator-set, candidate, signature, ordering, and authority boundaries before runtime finalization.
//! RO:INTERACTS — ron-proto committee checkpoint payload, validator set, checkpoint signatures, finalized checkpoint DTO.
//! RO:INVARIANTS — two-of-three finalizes; one-of-three rejects; wrong set/context/signature counts reject; signatures deterministic; unknown fields reject.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — DTO tests only; no private keys, runtime finality producer, wallet/ledger mutation, bridge, staking, or CrabLink authority.
//! RO:TEST — this file.

use ron_proto::{
    ContentId, QuickChainCanonicalEncodingV1, QuickChainCheckpointFinalityStateV1,
    QuickChainCheckpointValidatorSignatureV1, QuickChainCommitteeCheckpointPayloadV1,
    QuickChainConservationV1, QuickChainFinalizedCheckpointV1, QuickChainReceiptRootSchemeV1,
    QuickChainSettlementModeV1, QuickChainStateRootSchemeV1, QuickChainSupplyDeltaV1,
    QuickChainValidatorIdentityV1, QuickChainValidatorLifecycleStatusV1, QuickChainValidatorSetV1,
    SignatureAlg, QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA,
    QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA, QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
    QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1, QUICKCHAIN_VALIDATOR_SET_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_phase19_finality";

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content id must parse")
}

fn member(suffix: &str) -> QuickChainValidatorIdentityV1 {
    QuickChainValidatorIdentityV1 {
        schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_id: format!("validator-{suffix}"),
        passport_subject: format!("@validator-{suffix}"),
        registry_entry_id: format!("registry:validator-{suffix}"),
        key_id: format!("key:validator-{suffix}:001"),
        capability_id: format!("cap:validator-{suffix}:verify:001"),
        signature_algorithm: SignatureAlg::Ed25519,
        lifecycle_status: QuickChainValidatorLifecycleStatusV1::Active,
        not_before_ms: 1_800_000_000_000,
        expires_at_ms: 1_800_086_400_000,
    }
}

fn validator_set() -> QuickChainValidatorSetV1 {
    QuickChainValidatorSetV1 {
        schema: QUICKCHAIN_VALIDATOR_SET_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_set_hash: cid("phase19-finality-validator-set"),
        policy_hash: cid("phase19-finality-policy"),
        registry_snapshot_hash: cid("phase19-finality-registry"),
        passport_required: true,
        bond_required: false,
        validator_set_algorithm: QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1.to_owned(),
        members: vec![member("alpha"), member("beta"), member("gamma")],
    }
}

fn candidate(set: &QuickChainValidatorSetV1) -> QuickChainCommitteeCheckpointPayloadV1 {
    QuickChainCommitteeCheckpointPayloadV1 {
        schema: QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        height: 19,
        epoch_id: EPOCH_ID.to_owned(),
        execution_spec_version: "quickchain-execution-v1".to_owned(),
        previous_checkpoint_hash: cid("previous-checkpoint"),
        previous_state_root: cid("previous-state"),
        new_state_root: cid("new-state"),
        receipt_root: cid("receipt-root"),
        accounting_snapshot_root: cid("accounting-root"),
        reward_manifest_root: cid("reward-root"),
        data_availability_root: cid("da-root"),
        policy_hash: cid("phase19-finality-policy"),
        validator_set_hash: set.validator_set_hash.clone(),
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

fn signature(
    suffix: &str,
    checkpoint_hash: &ContentId,
) -> QuickChainCheckpointValidatorSignatureV1 {
    QuickChainCheckpointValidatorSignatureV1 {
        schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        height: 19,
        epoch_id: EPOCH_ID.to_owned(),
        checkpoint_hash: checkpoint_hash.clone(),
        validator_id: format!("validator-{suffix}"),
        key_id: format!("key:validator-{suffix}:001"),
        algorithm: SignatureAlg::Ed25519,
        signature_wire: "00".repeat(64),
    }
}

fn valid_finalized() -> QuickChainFinalizedCheckpointV1 {
    let set = validator_set();
    let payload = candidate(&set);
    let checkpoint_hash = cid("phase19-finalized-candidate");

    QuickChainFinalizedCheckpointV1 {
        schema: QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        candidate: payload,
        checkpoint_hash: checkpoint_hash.clone(),
        validator_set: set,
        required_signatures: 2,
        verified_unique_signatures: 2,
        signatures: vec![
            signature("alpha", &checkpoint_hash),
            signature("beta", &checkpoint_hash),
        ],
        finalization_state: QuickChainCheckpointFinalityStateV1::Finalized,
    }
}

#[test]
fn valid_two_of_three_finalized_checkpoint_contract_validates() {
    valid_finalized()
        .validate()
        .expect("valid two-of-three finalized checkpoint must validate");
}

#[test]
fn one_of_three_cannot_claim_finalized_checkpoint() {
    let mut artifact = valid_finalized();

    artifact.verified_unique_signatures = 1;
    artifact.signatures.truncate(1);

    artifact
        .validate()
        .expect_err("one-of-three must not validate as finalized");
}

#[test]
fn caller_cannot_lower_required_threshold_to_one() {
    let mut artifact = valid_finalized();
    artifact.required_signatures = 1;

    artifact
        .validate()
        .expect_err("caller-supplied weaker threshold must reject");
}

#[test]
fn validator_set_commitment_must_match_candidate() {
    let mut artifact = valid_finalized();
    artifact.validator_set.validator_set_hash = cid("different-validator-set");

    artifact
        .validate()
        .expect_err("wrong validator set commitment must reject");
}

#[test]
fn signature_count_must_match_verified_unique_count() {
    let mut artifact = valid_finalized();
    artifact.verified_unique_signatures = 3;

    artifact
        .validate()
        .expect_err("claimed verified count without matching signatures must reject");
}

#[test]
fn duplicate_validator_signature_rejects() {
    let mut artifact = valid_finalized();
    artifact.signatures[1] = artifact.signatures[0].clone();

    artifact
        .validate()
        .expect_err("duplicate validator signature must reject");
}

#[test]
fn signatures_must_be_sorted_deterministically_by_validator_id() {
    let mut artifact = valid_finalized();
    artifact.signatures.reverse();

    artifact
        .validate()
        .expect_err("reordered signatures must reject canonical finality ordering");
}

#[test]
fn wrong_signature_checkpoint_hash_rejects() {
    let mut artifact = valid_finalized();
    artifact.signatures[0].checkpoint_hash = cid("other-checkpoint");

    artifact
        .validate()
        .expect_err("signature over another checkpoint must reject");
}

#[test]
fn wrong_signature_key_binding_rejects() {
    let mut artifact = valid_finalized();
    artifact.signatures[0].key_id = "key:validator-gamma:001".to_owned();

    artifact
        .validate()
        .expect_err("signature key not matching validator membership must reject");
}

#[test]
fn finalized_checkpoint_rejects_unknown_authority_fields() {
    let value = serde_json::json!({
        "schema": QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "candidate": valid_finalized().candidate,
        "checkpoint_hash": cid("phase19-finalized-candidate").to_string(),
        "validator_set": validator_set(),
        "required_signatures": 2,
        "verified_unique_signatures": 2,
        "signatures": [
            signature("alpha", &cid("phase19-finalized-candidate")),
            signature("beta", &cid("phase19-finalized-candidate"))
        ],
        "finalization_state": "finalized",
        "finalized_by_process": "service-node-a"
    });

    assert!(
        serde_json::from_value::<QuickChainFinalizedCheckpointV1>(value).is_err(),
        "process-local finality authority fields must be rejected",
    );
}
