//! RO:WHAT — Phase 4 Round 3 controlled internal bond enforcement DTO validation tests.
//! RO:WHY — ECON/GOV: enforcement wire shape must be strict before ledger/service behavior consumes it.
//! RO:INTERACTS — ron_proto::quickchain::bond_enforcement.
//! RO:INVARIANTS — policy/operator gates required; capture requires governance; no unknown fields; component math conserves.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — DTO validity grants no spend, staking, bridge, settlement, finality, liquidity, or client authority.
//! RO:TEST — this file.

use ron_proto::quickchain::{
    QuickChainBondEnforcementDecisionStatusV1, QuickChainBondEnforcementDecisionV1,
    QuickChainBondEnforcementIntentV1, QuickChainBondEnforcementKindV1,
    QuickChainBondEnforcementRejectionCodeV1, QUICKCHAIN_BOND_ENFORCEMENT_DECISION_SCHEMA,
    QUICKCHAIN_BOND_ENFORCEMENT_INTENT_SCHEMA, QUICKCHAIN_DTO_VERSION,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch:phase4-r3";

fn intent(kind: QuickChainBondEnforcementKindV1) -> QuickChainBondEnforcementIntentV1 {
    QuickChainBondEnforcementIntentV1 {
        schema: QUICKCHAIN_BOND_ENFORCEMENT_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        enforcement_id: "bond-enforcement:alice-001".to_owned(),
        idempotency_key: "idem:bond-enforcement-alice-001".to_owned(),
        bond_account_id: "bond:validator-alice".to_owned(),
        validator_id: "validator:alice".to_owned(),
        actor_ref: "operator:alice".to_owned(),
        kind,
        amount_minor: "25".to_owned(),
        dispute_id: "bond-dispute:alice-001".to_owned(),
        evidence_id: "slash-evidence:alice-001".to_owned(),
        policy_decision_ref: "policy-decision:bond-enforcement-001".to_owned(),
        governance_approval_ref: Some("gov:bond-enforcement-001".to_owned()),
        operator_confirmation_ref: "operator-confirmation:alice-001".to_owned(),
    }
}

#[test]
fn reserve_release_and_capture_enforcement_intents_validate() {
    for kind in [
        QuickChainBondEnforcementKindV1::ReserveSlash,
        QuickChainBondEnforcementKindV1::ReleaseSlashReserve,
        QuickChainBondEnforcementKindV1::CaptureSlashReserve,
    ] {
        intent(kind)
            .validate()
            .expect("controlled internal enforcement intent should validate");
    }
}

#[test]
fn capture_and_reserve_require_governance_approval() {
    for kind in [
        QuickChainBondEnforcementKindV1::ReserveSlash,
        QuickChainBondEnforcementKindV1::CaptureSlashReserve,
    ] {
        let mut candidate = intent(kind);
        candidate.governance_approval_ref = None;

        assert!(
            candidate.validate().is_err(),
            "reserve/capture enforcement must require explicit governance approval"
        );
    }

    let mut release = intent(QuickChainBondEnforcementKindV1::ReleaseSlashReserve);
    release.governance_approval_ref = None;
    release.validate().expect(
        "release of reserved slash may be policy/operator gated without governance approval",
    );
}

#[test]
fn enforcement_intent_rejects_zero_amount_and_missing_policy_gate() {
    let mut zero = intent(QuickChainBondEnforcementKindV1::ReserveSlash);
    zero.amount_minor = "0".to_owned();
    assert!(zero.validate().is_err());

    let mut missing_policy = intent(QuickChainBondEnforcementKindV1::ReserveSlash);
    missing_policy.policy_decision_ref.clear();
    assert!(missing_policy.validate().is_err());

    let mut missing_operator = intent(QuickChainBondEnforcementKindV1::ReserveSlash);
    missing_operator.operator_confirmation_ref.clear();
    assert!(missing_operator.validate().is_err());
}

#[test]
fn enforcement_intent_rejects_unknown_fields() {
    let json = r#"{
      "schema":"quickchain.bond-enforcement-intent.v1",
      "version":1,
      "chain_id":"ron-devnet",
      "epoch_id":"epoch:phase4-r3",
      "enforcement_id":"bond-enforcement:alice-001",
      "idempotency_key":"idem:bond-enforcement-alice-001",
      "bond_account_id":"bond:validator-alice",
      "validator_id":"validator:alice",
      "actor_ref":"operator:alice",
      "kind":"reserve_slash",
      "amount_minor":"25",
      "dispute_id":"bond-dispute:alice-001",
      "evidence_id":"slash-evidence:alice-001",
      "policy_decision_ref":"policy-decision:bond-enforcement-001",
      "governance_approval_ref":"gov:bond-enforcement-001",
      "operator_confirmation_ref":"operator-confirmation:alice-001",
      "public_staking_market":"forbidden"
    }"#;

    assert!(
        serde_json::from_str::<QuickChainBondEnforcementIntentV1>(json).is_err(),
        "unknown public-staking field must reject"
    );
}

#[test]
fn enforcement_decision_validates_status_and_component_conservation() {
    let decision = QuickChainBondEnforcementDecisionV1 {
        schema: QUICKCHAIN_BOND_ENFORCEMENT_DECISION_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        enforcement_id: "bond-enforcement:alice-001".to_owned(),
        bond_account_id: "bond:validator-alice".to_owned(),
        validator_id: "validator:alice".to_owned(),
        kind: QuickChainBondEnforcementKindV1::ReserveSlash,
        status: QuickChainBondEnforcementDecisionStatusV1::Accepted,
        rejection_code: None,
        amount_minor: "25".to_owned(),
        resulting_locked_minor: "100".to_owned(),
        resulting_available_to_unlock_minor: "75".to_owned(),
        resulting_pending_unlock_minor: "0".to_owned(),
        resulting_slash_reserved_minor: "25".to_owned(),
        account_sequence: 2,
    };

    decision
        .validate()
        .expect("accepted enforcement decision should validate");

    let mut broken_math = decision.clone();
    broken_math.resulting_locked_minor = "101".to_owned();
    assert!(broken_math.validate().is_err());

    let mut accepted_with_rejection = decision.clone();
    accepted_with_rejection.rejection_code =
        Some(QuickChainBondEnforcementRejectionCodeV1::SilentMutationForbidden);
    assert!(accepted_with_rejection.validate().is_err());

    let rejected = QuickChainBondEnforcementDecisionV1 {
        status: QuickChainBondEnforcementDecisionStatusV1::Rejected,
        rejection_code: Some(
            QuickChainBondEnforcementRejectionCodeV1::OneStepIrreversibleSlashForbidden,
        ),
        ..decision
    };

    rejected
        .validate()
        .expect("rejected enforcement decision should carry a deterministic code");
}
