//! RO:WHAT — Phase 4 Round 1 bond DTO validation tests.
//! RO:WHY — ECON/GOV: bond DTOs must be strict, integer-only, ROC-only, and non-authoritative before ledger/service use.
//! RO:INTERACTS — ron_proto::quickchain::bond DTOs.
//! RO:INVARIANTS — deny unknown fields; no floats; no ROX/external asset; no automatic slashing authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — tests prove DTO shape only, not wallet mutation or live slashing.
//! RO:TEST — cargo test -p ron-proto --test quickchain_phase4_bond_dto.

use ron_proto::{
    quickchain::{
        QuickChainBondAccountStatusV1, QuickChainBondIntentKindV1,
        QuickChainBondLifecycleDecisionStatusV1, QuickChainBondLifecycleDecisionV1,
        QuickChainBondLifecycleOperationV1, QuickChainBondLifecycleRejectionCodeV1,
        QuickChainSlashEvidenceKindV1, QuickChainSlashEvidenceV1, QuickChainValidationError,
        QuickChainValidatorBondAccountV1, QuickChainValidatorBondIntentV1,
        QUICKCHAIN_BOND_ASSET_ROC, QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_SLASH_EVIDENCE_SCHEMA,
        QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA, QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA,
    },
    ContentId,
};
use serde_json::{json, Value};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch:phase4-r1";

fn cid(ch: char) -> ContentId {
    format!("b3:{}", ch.to_string().repeat(64))
        .parse()
        .expect("test ContentId should be valid")
}

fn bond_intent(kind: QuickChainBondIntentKindV1) -> QuickChainValidatorBondIntentV1 {
    QuickChainValidatorBondIntentV1 {
        schema: QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_id: "validator:alice".to_owned(),
        bond_account_id: "bond:validator-alice".to_owned(),
        actor_account_id: "acct:operator-alice".to_owned(),
        intent_id: "bond-intent:alice-open".to_owned(),
        idempotency_key: "idem:bond-alice-open".to_owned(),
        kind,
        asset: QUICKCHAIN_BOND_ASSET_ROC.to_owned(),
        amount_minor: Some("250".to_owned()),
        unlock_epoch_id: None,
        governance_approval_ref: Some("gov:phase4-bond-alpha".to_owned()),
    }
}

fn bond_account() -> QuickChainValidatorBondAccountV1 {
    QuickChainValidatorBondAccountV1 {
        schema: QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_id: "validator:alice".to_owned(),
        bond_account_id: "bond:validator-alice".to_owned(),
        owner_account_id: "acct:operator-alice".to_owned(),
        asset: QUICKCHAIN_BOND_ASSET_ROC.to_owned(),
        locked_minor: "250".to_owned(),
        available_to_unlock_minor: "200".to_owned(),
        pending_unlock_minor: "50".to_owned(),
        slash_reserved_minor: "0".to_owned(),
        status: QuickChainBondAccountStatusV1::UnlockPending,
        account_sequence: 1,
    }
}

fn slash_evidence() -> QuickChainSlashEvidenceV1 {
    QuickChainSlashEvidenceV1 {
        schema: QUICKCHAIN_SLASH_EVIDENCE_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_set_hash: cid('a'),
        evidence_id: "slash-evidence:alice-001".to_owned(),
        validator_id: "validator:alice".to_owned(),
        evidence_kind: QuickChainSlashEvidenceKindV1::ValidatorEquivocation,
        evidence_ref: "evidence:equivocation:alice-001".to_owned(),
        submitter_ref: "passport:main:watcher".to_owned(),
        observed_at_ms: 1_800_000_000_000,
        recommended_freeze: true,
        recommended_amount_minor: Some("10".to_owned()),
    }
}

#[test]
fn bond_intents_validate_explicit_roc_integer_shape() {
    bond_intent(QuickChainBondIntentKindV1::OpenBond)
        .validate()
        .expect("open bond intent should validate");

    bond_intent(QuickChainBondIntentKindV1::IncreaseBond)
        .validate()
        .expect("increase bond intent should validate");

    let mut unlock = bond_intent(QuickChainBondIntentKindV1::RequestUnlock);
    unlock.intent_id = "bond-intent:alice-unlock".to_owned();
    unlock.idempotency_key = "idem:bond-alice-unlock".to_owned();
    unlock.unlock_epoch_id = Some("epoch:phase4-r1-unlock".to_owned());
    unlock
        .validate()
        .expect("unlock request should require and accept explicit unlock epoch");

    let mut cancel = bond_intent(QuickChainBondIntentKindV1::CancelUnlockRequest);
    cancel.intent_id = "bond-intent:alice-cancel".to_owned();
    cancel.idempotency_key = "idem:bond-alice-cancel".to_owned();
    cancel
        .validate()
        .expect("cancel unlock request should validate");
}

#[test]
fn bond_intents_reject_missing_amount_float_asset_and_wrong_unlock_epoch() {
    let mut missing = bond_intent(QuickChainBondIntentKindV1::OpenBond);
    missing.amount_minor = None;
    assert!(matches!(
        missing.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "amount_minor",
            ..
        })
    ));

    let mut zero = bond_intent(QuickChainBondIntentKindV1::OpenBond);
    zero.amount_minor = Some("0".to_owned());
    assert!(matches!(
        zero.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "amount_minor",
            reason: "amount must be greater than zero"
        })
    ));

    let mut float = bond_intent(QuickChainBondIntentKindV1::OpenBond);
    float.amount_minor = Some("1.5".to_owned());
    assert!(matches!(
        float.validate(),
        Err(QuickChainValidationError::InvalidMoney {
            field: "amount_minor",
            ..
        })
    ));

    let mut external_asset = bond_intent(QuickChainBondIntentKindV1::OpenBond);
    external_asset.asset = "rox".to_owned();
    assert!(matches!(
        external_asset.validate(),
        Err(QuickChainValidationError::InvalidField { field: "asset", .. })
    ));

    let mut bad_unlock_epoch = bond_intent(QuickChainBondIntentKindV1::IncreaseBond);
    bad_unlock_epoch.unlock_epoch_id = Some("epoch:not-valid-here".to_owned());
    assert!(matches!(
        bad_unlock_epoch.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "unlock_epoch_id",
            ..
        })
    ));
}

#[test]
fn bond_account_conserves_component_math() {
    bond_account()
        .validate()
        .expect("bond account with locked=available+pending+reserved should validate");

    let mut broken = bond_account();
    broken.locked_minor = "251".to_owned();

    assert!(matches!(
        broken.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "locked_minor",
            reason: "locked amount must equal available + pending_unlock + slash_reserved"
        })
    ));

    let mut zero_sequence = bond_account();
    zero_sequence.account_sequence = 0;

    assert!(matches!(
        zero_sequence.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "account_sequence",
            ..
        })
    ));
}

#[test]
fn slash_evidence_is_strict_evidence_only_shape() {
    slash_evidence()
        .validate()
        .expect("slash evidence DTO shape should validate");

    let mut bad_amount = slash_evidence();
    bad_amount.recommended_amount_minor = Some("01".to_owned());

    assert!(matches!(
        bad_amount.validate(),
        Err(QuickChainValidationError::InvalidMoney {
            field: "recommended_amount_minor",
            ..
        })
    ));

    let mut missing_time = slash_evidence();
    missing_time.observed_at_ms = 0;

    assert!(matches!(
        missing_time.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "observed_at_ms",
            ..
        })
    ));
}

#[test]
fn lifecycle_decision_requires_rejection_code_only_when_rejected() {
    let accepted = QuickChainBondLifecycleDecisionV1 {
        schema: QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_id: "validator:alice".to_owned(),
        bond_account_id: "bond:validator-alice".to_owned(),
        operation: QuickChainBondLifecycleOperationV1::OpenBond,
        status: QuickChainBondLifecycleDecisionStatusV1::Accepted,
        rejection_code: None,
        amount_minor: Some("250".to_owned()),
    };

    accepted
        .validate()
        .expect("accepted decision without rejection code should validate");

    let rejected = QuickChainBondLifecycleDecisionV1 {
        status: QuickChainBondLifecycleDecisionStatusV1::Rejected,
        rejection_code: Some(QuickChainBondLifecycleRejectionCodeV1::AutomaticSlashingForbidden),
        operation: QuickChainBondLifecycleOperationV1::EvaluateSlashEvidenceNoop,
        ..accepted.clone()
    };

    rejected
        .validate()
        .expect("rejected decision with deterministic code should validate");

    let mut invalid_accepted = accepted.clone();
    invalid_accepted.rejection_code =
        Some(QuickChainBondLifecycleRejectionCodeV1::PublicStakingForbidden);

    assert!(matches!(
        invalid_accepted.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "rejection_code",
            ..
        })
    ));

    let mut invalid_rejected = rejected;
    invalid_rejected.rejection_code = None;

    assert!(matches!(
        invalid_rejected.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "rejection_code",
            ..
        })
    ));
}

#[test]
fn bond_dtos_reject_unknown_fields() {
    let mut value: Value = serde_json::to_value(bond_intent(QuickChainBondIntentKindV1::OpenBond))
        .expect("intent should serialize");
    value
        .as_object_mut()
        .expect("intent JSON should be an object")
        .insert("public_staking_market".to_owned(), json!(true));

    serde_json::from_value::<QuickChainValidatorBondIntentV1>(value)
        .expect_err("unknown bond intent fields must reject");

    let mut account_value = serde_json::to_value(bond_account()).expect("account should serialize");
    account_value
        .as_object_mut()
        .expect("account JSON should be an object")
        .insert("liquidity_pool".to_owned(), json!("forbidden"));

    serde_json::from_value::<QuickChainValidatorBondAccountV1>(account_value)
        .expect_err("unknown bond account fields must reject");

    let mut evidence_value =
        serde_json::to_value(slash_evidence()).expect("evidence should serialize");
    evidence_value
        .as_object_mut()
        .expect("evidence JSON should be an object")
        .insert("auto_slash_now".to_owned(), json!(true));

    serde_json::from_value::<QuickChainSlashEvidenceV1>(evidence_value)
        .expect_err("unknown slash evidence fields must reject");
}
