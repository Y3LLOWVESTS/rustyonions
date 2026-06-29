//! RO:WHAT — Tests QuickChain Phase 5 Round 3 chosen external posture DTOs.
//! RO:WHY — ECON/GOV: Round 3 chooses anchor-only evidence while blocking bridge, settlement, unlock, receipt, and balance authority.
//! RO:INTERACTS — ron_proto::quickchain::external_posture.
//! RO:INVARIANTS — one chosen path; evidence-only; wallet/ledger truth canonical; no public runtime authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — strict unknown-field rejection and authority-flag rejection.
//! RO:TEST — cargo test -p ron-proto --test quickchain_phase5_external_posture.

use ron_proto::{
    ContentId, QuickChainCanonicalEncodingV1, QuickChainExternalIntegrationPostureV1,
    QuickChainExternalPostureDecisionV1, QuickChainExternalPostureSemanticsV1,
    QuickChainExternalPostureVerificationV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_EXTERNAL_POSTURE_DECISION_SCHEMA, QUICKCHAIN_EXTERNAL_POSTURE_VERIFICATION_SCHEMA,
};
use serde_json::json;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn decision() -> QuickChainExternalPostureDecisionV1 {
    QuickChainExternalPostureDecisionV1 {
        schema: QUICKCHAIN_EXTERNAL_POSTURE_DECISION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r3".to_string(),
        posture_id: "external-posture:phase5-r3:001".to_string(),
        decision_ref: "governance:phase5-r3:anchor-only".to_string(),
        anchor_commitment_hash: cid('a'),
        evidence_ref: "anchor-evidence:phase5-r3:001".to_string(),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        chosen_posture: QuickChainExternalIntegrationPostureV1::AnchorOnly,
        posture_semantics: QuickChainExternalPostureSemanticsV1::EvidenceAndAnchoringOnly,
        produced_at_ms: 1_800_000_300_000,
        anchor_only_selected: true,
        evidence_and_anchoring_only: true,
        wallet_ledger_truth_canonical: true,
        external_da_selected: false,
        external_l2_selected: false,
        hybrid_selected: false,
        balance_mutation_authorized: false,
        receipt_authority_authorized: false,
        paid_unlock_authorized: false,
        external_settlement_authorized: false,
        bridge_authorized: false,
        rox_solana_runtime_authorized: false,
        exchange_facing_authorized: false,
        liquidity_authorized: false,
        staking_authorized: false,
    }
}

fn verification() -> QuickChainExternalPostureVerificationV1 {
    QuickChainExternalPostureVerificationV1 {
        schema: QUICKCHAIN_EXTERNAL_POSTURE_VERIFICATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        epoch_id: "epoch:phase5-r3".to_string(),
        posture_id: "external-posture:phase5-r3:001".to_string(),
        chosen_posture: QuickChainExternalIntegrationPostureV1::AnchorOnly,
        anchor_commitment_hash: cid('a'),
        observed_evidence_ref: "anchor-evidence:phase5-r3:001".to_string(),
        single_path_selected: true,
        verified_evidence_only: true,
        wallet_ledger_truth_canonical: true,
        balance_mutation_detected: false,
        receipt_authority_detected: false,
        paid_unlock_detected: false,
        external_settlement_detected: false,
        bridge_detected: false,
        rox_solana_runtime_detected: false,
        exchange_facing_detected: false,
        liquidity_detected: false,
        staking_detected: false,
    }
}

#[test]
fn external_posture_decision_validates_and_roundtrips_json() {
    let dto = decision();

    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    assert!(encoded.contains("\"chosen_posture\":\"anchor_only\""));
    assert!(encoded.contains("\"posture_semantics\":\"evidence_and_anchoring_only\""));

    let decoded: QuickChainExternalPostureDecisionV1 = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, dto);
    decoded.validate().unwrap();
}

#[test]
fn external_posture_decision_rejects_unknown_fields() {
    let mut value = serde_json::to_value(decision()).unwrap();
    value["public_bridge"] = json!(true);

    let err = serde_json::from_value::<QuickChainExternalPostureDecisionV1>(value).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn external_posture_decision_rejects_non_anchor_only_choices() {
    let mut dto = decision();
    dto.chosen_posture = QuickChainExternalIntegrationPostureV1::ExternalDataAvailabilityDeferred;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.chosen_posture = QuickChainExternalIntegrationPostureV1::ExternalLayerTwoDeferred;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.chosen_posture = QuickChainExternalIntegrationPostureV1::HybridDeferred;
    assert!(dto.validate().is_err());
}

#[test]
fn external_posture_decision_rejects_missing_required_truth_flags() {
    let mut dto = decision();
    dto.anchor_only_selected = false;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.evidence_and_anchoring_only = false;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.wallet_ledger_truth_canonical = false;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.produced_at_ms = 0;
    assert!(dto.validate().is_err());
}

#[test]
fn external_posture_decision_rejects_authority_creep_flags() {
    let mut dto = decision();
    dto.external_da_selected = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.external_l2_selected = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.hybrid_selected = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.balance_mutation_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.receipt_authority_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.paid_unlock_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.external_settlement_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.bridge_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.rox_solana_runtime_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.exchange_facing_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.liquidity_authorized = true;
    assert!(dto.validate().is_err());

    let mut dto = decision();
    dto.staking_authorized = true;
    assert!(dto.validate().is_err());
}

#[test]
fn external_posture_verification_validates_and_roundtrips_json() {
    let dto = verification();

    dto.validate().unwrap();

    let encoded = serde_json::to_string(&dto).unwrap();
    let decoded: QuickChainExternalPostureVerificationV1 = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, dto);
    decoded.validate().unwrap();
}

#[test]
fn external_posture_verification_rejects_authority_creep_flags() {
    let mut dto = verification();
    dto.chosen_posture = QuickChainExternalIntegrationPostureV1::HybridDeferred;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.single_path_selected = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.verified_evidence_only = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.wallet_ledger_truth_canonical = false;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.balance_mutation_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.receipt_authority_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.paid_unlock_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.external_settlement_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.bridge_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.rox_solana_runtime_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.exchange_facing_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.liquidity_detected = true;
    assert!(dto.validate().is_err());

    let mut dto = verification();
    dto.staking_detected = true;
    assert!(dto.validate().is_err());
}

#[test]
fn external_posture_verification_rejects_unknown_fields() {
    let mut value = serde_json::to_value(verification()).unwrap();
    value["paid_unlock_from_anchor"] = json!(true);

    let err = serde_json::from_value::<QuickChainExternalPostureVerificationV1>(value).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}
