#![allow(clippy::missing_panics_doc)]

//! RO:WHAT — FINAL_BETA Phase 19 checkpoint-validator signature-contract tests.
//! RO:WHY — Freeze the exact validator signing preimage before crypto, committee threshold, or finality work.
//! RO:INTERACTS — ron-proto checkpoint_signature module, ContentId, SignatureAlg.
//! RO:INVARIANTS — every required context field changes signing bytes; signature bytes never sign themselves; unknown fields reject.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — protocol tests only; no private keys, signing runtime, quorum, finality, wallet mutation, or ledger mutation.
//! RO:TEST — this file.

use ron_proto::{
    checkpoint_validator_signature_message_bytes, ContentId,
    QuickChainCheckpointValidatorSignatureV1, QuickChainCheckpointValidatorSigningPayloadV1,
    SignatureAlg, QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_DOMAIN,
    QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA,
    QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
};

fn cid(ch: char) -> ContentId {
    format!("b3:{}", ch.to_string().repeat(64),)
        .parse()
        .expect("fixture content id must parse")
}

fn valid_payload() -> QuickChainCheckpointValidatorSigningPayloadV1 {
    QuickChainCheckpointValidatorSigningPayloadV1 {
        schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "ron-devnet".to_owned(),
        height: 19,
        epoch_id: "epoch_phase19_checkpoint_signature".to_owned(),
        checkpoint_hash: cid('a'),
        validator_id: "validator-alpha".to_owned(),
        key_id: "validator-alpha-key-01".to_owned(),
        algorithm: SignatureAlg::Ed25519,
    }
}

fn valid_signature() -> QuickChainCheckpointValidatorSignatureV1 {
    let payload = valid_payload();

    QuickChainCheckpointValidatorSignatureV1 {
        schema: QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA.to_owned(),
        version: payload.version,
        chain_id: payload.chain_id,
        height: payload.height,
        epoch_id: payload.epoch_id,
        checkpoint_hash: payload.checkpoint_hash,
        validator_id: payload.validator_id,
        key_id: payload.key_id,
        algorithm: payload.algorithm,
        signature_wire: "00".repeat(64),
    }
}

fn message_bytes(payload: &QuickChainCheckpointValidatorSigningPayloadV1) -> Vec<u8> {
    checkpoint_validator_signature_message_bytes(payload)
        .expect("valid checkpoint-validator payload must canonicalize")
}

#[test]
fn valid_checkpoint_validator_signing_payload_and_signature_validate() {
    valid_payload()
        .validate()
        .expect("valid signing payload must validate");

    valid_signature()
        .validate()
        .expect("valid signature artifact must validate");
}

#[test]
fn same_signing_payload_produces_identical_canonical_message_bytes() {
    let first = valid_payload();
    let second = valid_payload();

    assert_eq!(message_bytes(&first), message_bytes(&second),);
}

#[test]
fn canonical_message_binds_exact_required_checkpoint_validator_context() {
    let payload = valid_payload();
    let bytes = message_bytes(&payload);

    let value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("canonical message must be json");

    assert_eq!(
        value["domain"],
        QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_DOMAIN,
    );
    assert_eq!(value["version"], QUICKCHAIN_DTO_VERSION);
    assert_eq!(value["chain_id"], payload.chain_id);
    assert_eq!(value["height"], payload.height);
    assert_eq!(value["epoch_id"], payload.epoch_id);
    assert_eq!(
        value["checkpoint_hash"],
        payload.checkpoint_hash.to_string(),
    );
    assert_eq!(value["validator_id"], payload.validator_id);
    assert_eq!(value["key_id"], payload.key_id);
    assert_eq!(value["algorithm"], "ed25519");

    assert!(
        value.get("signature_wire").is_none(),
        "signature material must never appear in its own signing preimage",
    );
    assert!(
        value.get("finality").is_none(),
        "signing contract must not contain finality state",
    );
    assert!(
        value.get("quorum").is_none(),
        "signing contract must not contain quorum state",
    );
}

#[test]
fn every_required_context_change_changes_canonical_signing_bytes() {
    let baseline = valid_payload();
    let baseline_bytes = message_bytes(&baseline);

    let mut changed_chain = baseline.clone();
    changed_chain.chain_id = "ron-other".to_owned();

    let mut changed_height = baseline.clone();
    changed_height.height += 1;

    let mut changed_epoch = baseline.clone();
    changed_epoch.epoch_id = "epoch_phase19_other".to_owned();

    let mut changed_checkpoint = baseline.clone();
    changed_checkpoint.checkpoint_hash = cid('b');

    let mut changed_validator = baseline.clone();
    changed_validator.validator_id = "validator-beta".to_owned();

    let mut changed_key = baseline.clone();
    changed_key.key_id = "validator-alpha-key-02".to_owned();

    let mut changed_algorithm = baseline.clone();
    changed_algorithm.algorithm = SignatureAlg::Dilithium3;

    for changed in [
        changed_chain,
        changed_height,
        changed_epoch,
        changed_checkpoint,
        changed_validator,
        changed_key,
        changed_algorithm,
    ] {
        assert_ne!(
            baseline_bytes,
            message_bytes(&changed),
            "every required signing-context change must change message bytes",
        );
    }
}

#[test]
fn signature_wire_is_excluded_from_reconstructed_signing_payload_and_message() {
    let first = valid_signature();

    let mut second = first.clone();
    second.signature_wire = "11".repeat(64);

    assert_ne!(first.signature_wire, second.signature_wire,);

    let first_payload = first.signing_payload();
    let second_payload = second.signing_payload();

    assert_eq!(
        first_payload, second_payload,
        "signature bytes must not alter the reconstructed signing payload",
    );

    assert_eq!(
        message_bytes(&first_payload),
        message_bytes(&second_payload),
        "signature bytes must not alter canonical signing message bytes",
    );
}

#[test]
fn zero_height_rejects_before_signing_message_is_returned() {
    let mut payload = valid_payload();
    payload.height = 0;

    let err = checkpoint_validator_signature_message_bytes(&payload)
        .expect_err("zero-height signing payload must reject");

    assert!(
        err.to_string().contains("height"),
        "failure should identify checkpoint height: {err}",
    );
}

#[test]
fn signing_payload_and_signature_reject_unknown_fields() {
    let payload_json = serde_json::json!({
        "schema": QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNING_PAYLOAD_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "chain_id": "ron-devnet",
        "height": 19,
        "epoch_id": "epoch_phase19_checkpoint_signature",
        "checkpoint_hash": cid('a').to_string(),
        "validator_id": "validator-alpha",
        "key_id": "validator-alpha-key-01",
        "algorithm": "ed25519",
        "alternate_preimage": "forbidden"
    });

    assert!(
        serde_json::from_value::<QuickChainCheckpointValidatorSigningPayloadV1>(payload_json,)
            .is_err(),
        "signing payload must deny alternate caller-supplied preimage fields",
    );

    let signature_json = serde_json::json!({
        "schema": QUICKCHAIN_CHECKPOINT_VALIDATOR_SIGNATURE_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "chain_id": "ron-devnet",
        "height": 19,
        "epoch_id": "epoch_phase19_checkpoint_signature",
        "checkpoint_hash": cid('a').to_string(),
        "validator_id": "validator-alpha",
        "key_id": "validator-alpha-key-01",
        "algorithm": "ed25519",
        "signature_wire": "00".repeat(64),
        "finalized": true
    });

    assert!(
        serde_json::from_value::<QuickChainCheckpointValidatorSignatureV1>(signature_json,)
            .is_err(),
        "signature artifact must deny caller-supplied finality fields",
    );
}
