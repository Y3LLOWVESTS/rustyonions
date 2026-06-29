use ron_proto::{
    ContentId, QuickChainAnchorCommitmentV1, QuickChainAnchorMediumV1, QuickChainAnchorSemanticsV1,
    QuickChainAnchorVerificationV1, QuickChainCanonicalEncodingV1,
    QUICKCHAIN_ANCHOR_COMMITMENT_SCHEMA, QUICKCHAIN_ANCHOR_VERIFICATION_SCHEMA,
    QUICKCHAIN_DTO_VERSION,
};
use serde_json::json;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn commitment() -> QuickChainAnchorCommitmentV1 {
    QuickChainAnchorCommitmentV1 {
        schema: QUICKCHAIN_ANCHOR_COMMITMENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r1".to_string(),
        anchor_id: "anchor:phase5-r1:001".to_string(),
        anchor_provider_ref: "dry-run/local-file".to_string(),
        external_reference: "dry-run:phase5-r1:artifact-001".to_string(),
        dry_run_artifact_ref: "file:quickchain-phase5-r1-anchor-001.json".to_string(),
        checkpoint_height: 7,
        checkpoint_hash: cid('a'),
        new_state_root: cid('1'),
        receipt_root: cid('2'),
        accounting_snapshot_root: cid('3'),
        reward_manifest_root: cid('4'),
        data_availability_root: cid('5'),
        policy_hash: cid('6'),
        validator_set_hash: cid('7'),
        chain_params_hash: cid('8'),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        anchor_medium: QuickChainAnchorMediumV1::DryRunFile,
        anchor_semantics: QuickChainAnchorSemanticsV1::TimestampCommitmentOnly,
        produced_at_ms: 1_800_000_000_000,
        dry_run_only: true,
        timestamp_commitment_only: true,
        balance_mutation_authorized: false,
        wallet_ledger_truth_replaced: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        external_chain_truth: false,
        finality_claimed: false,
    }
}

fn verification() -> QuickChainAnchorVerificationV1 {
    QuickChainAnchorVerificationV1 {
        schema: QUICKCHAIN_ANCHOR_VERIFICATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r1".to_string(),
        anchor_id: "anchor:phase5-r1:001".to_string(),
        checkpoint_height: 7,
        checkpoint_hash: cid('a'),
        observed_external_reference: "dry-run:phase5-r1:artifact-001".to_string(),
        verified_commitment_only: true,
        balance_mutation_detected: false,
        wallet_ledger_truth_replaced: false,
        external_settlement_detected: false,
        bridge_detected: false,
        finality_detected: false,
    }
}

#[test]
fn anchor_commitment_validates_and_roundtrips_json() {
    let dto = commitment();

    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    let decoded: QuickChainAnchorCommitmentV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, dto);
    decoded.validate().unwrap();
}

#[test]
fn anchor_commitment_rejects_unknown_fields() {
    let mut value = serde_json::to_value(commitment()).unwrap();
    value["settlement_authority"] = json!(true);

    let err = serde_json::from_value::<QuickChainAnchorCommitmentV1>(value).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn anchor_commitment_rejects_authority_claims() {
    let mut dto = commitment();
    dto.dry_run_only = false;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.timestamp_commitment_only = false;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.balance_mutation_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.wallet_ledger_truth_replaced = true;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.external_settlement_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.bridge_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.external_chain_truth = true;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.finality_claimed = true;
    assert!(dto.validate().is_err());
}

#[test]
fn anchor_commitment_rejects_non_json_encoding_and_zero_timestamp() {
    let mut dto = commitment();
    dto.canonical_encoding = QuickChainCanonicalEncodingV1::CborV1;
    assert!(dto.validate().is_err());

    let mut dto = commitment();
    dto.produced_at_ms = 0;
    assert!(dto.validate().is_err());
}

#[test]
fn anchor_verification_validates_and_roundtrips_json() {
    let dto = verification();

    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    let decoded: QuickChainAnchorVerificationV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, dto);
    decoded.validate().unwrap();
}

#[test]
fn anchor_verification_rejects_authority_claims() {
    let mut dto = verification();
    dto.verified_commitment_only = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.balance_mutation_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.wallet_ledger_truth_replaced = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.external_settlement_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.bridge_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.finality_detected = true;
    assert!(dto.validate().is_err());
}

#[test]
fn anchor_verification_rejects_unknown_fields() {
    let mut value = serde_json::to_value(verification()).unwrap();
    value["external_finality"] = json!(true);

    let err = serde_json::from_value::<QuickChainAnchorVerificationV1>(value).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}
