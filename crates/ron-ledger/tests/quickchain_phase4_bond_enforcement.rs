#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 4 Round 3 controlled internal bond enforcement tests.
//! RO:WHY — ECON/GOV: prove reserve/release/capture conservation before downstream services consume this boundary.
//! RO:INTERACTS — ron-ledger bond_accounting and ron-proto bond_enforcement DTOs.
//! RO:INVARIANTS — capture only reserved bond; COW rejection; policy/operator/governance gates; no public staking/bridge.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — tests prove internal deterministic model behavior, not public staking or external settlement.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_enforcement.

use ron_ledger::quickchain::{QuickChainBondAccountingState, QuickChainBondLedgerError};
use ron_proto::quickchain::{
    QuickChainBondAccountStatusV1, QuickChainBondEnforcementDecisionStatusV1,
    QuickChainBondEnforcementIntentV1, QuickChainBondEnforcementKindV1, QuickChainBondIntentKindV1,
    QuickChainValidatorBondIntentV1, QUICKCHAIN_BOND_ASSET_ROC,
    QUICKCHAIN_BOND_ENFORCEMENT_INTENT_SCHEMA, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch:phase4-r3";
const BOND_ACCOUNT_ID: &str = "bond:validator-alice";
const VALIDATOR_ID: &str = "validator:alice";
const OWNER_ACCOUNT_ID: &str = "acct:operator-alice";

fn bond_intent(
    kind: QuickChainBondIntentKindV1,
    amount_minor: &str,
    suffix: &str,
) -> QuickChainValidatorBondIntentV1 {
    QuickChainValidatorBondIntentV1 {
        schema: QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_id: VALIDATOR_ID.to_owned(),
        bond_account_id: BOND_ACCOUNT_ID.to_owned(),
        actor_account_id: OWNER_ACCOUNT_ID.to_owned(),
        intent_id: format!("bond-intent:alice-{suffix}"),
        idempotency_key: format!("idem:bond-alice-{suffix}"),
        kind,
        asset: QUICKCHAIN_BOND_ASSET_ROC.to_owned(),
        amount_minor: Some(amount_minor.to_owned()),
        unlock_epoch_id: None,
        governance_approval_ref: Some("gov:phase4-bond-alpha".to_owned()),
    }
}

fn unlock_intent(amount_minor: &str) -> QuickChainValidatorBondIntentV1 {
    let mut intent = bond_intent(
        QuickChainBondIntentKindV1::RequestUnlock,
        amount_minor,
        "unlock",
    );
    intent.unlock_epoch_id = Some("epoch:phase4-r3-unlock".to_owned());
    intent
}

fn enforcement(
    kind: QuickChainBondEnforcementKindV1,
    amount_minor: &str,
    suffix: &str,
) -> QuickChainBondEnforcementIntentV1 {
    QuickChainBondEnforcementIntentV1 {
        schema: QUICKCHAIN_BOND_ENFORCEMENT_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        enforcement_id: format!("bond-enforcement:alice-{suffix}"),
        idempotency_key: format!("idem:bond-enforcement-alice-{suffix}"),
        bond_account_id: BOND_ACCOUNT_ID.to_owned(),
        validator_id: VALIDATOR_ID.to_owned(),
        actor_ref: "operator:alice".to_owned(),
        kind,
        amount_minor: amount_minor.to_owned(),
        dispute_id: "bond-dispute:alice-001".to_owned(),
        evidence_id: "slash-evidence:alice-001".to_owned(),
        policy_decision_ref: "policy-decision:bond-enforcement-001".to_owned(),
        governance_approval_ref: Some("gov:bond-enforcement-001".to_owned()),
        operator_confirmation_ref: "operator-confirmation:alice-001".to_owned(),
    }
}

fn opened_state(amount_minor: &str) -> QuickChainBondAccountingState {
    let mut state = QuickChainBondAccountingState::new();
    state
        .apply_explicit_bond_intent(
            &bond_intent(QuickChainBondIntentKindV1::OpenBond, amount_minor, "open"),
            10_000,
        )
        .expect("open bond should model successfully");
    state
}

#[test]
fn reserve_and_release_slash_reserved_amount_conserve_locked_components() {
    let mut state = opened_state("300");

    let reserve = state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::ReserveSlash,
            "75",
            "reserve",
        ))
        .expect("reserve should model successfully");

    assert_eq!(reserve.captured_minor(), 0);
    assert_eq!(reserve.released_minor(), 0);
    assert_eq!(
        reserve.decision().status,
        QuickChainBondEnforcementDecisionStatusV1::Accepted
    );
    assert_eq!(reserve.decision().resulting_locked_minor, "300");
    assert_eq!(
        reserve.decision().resulting_available_to_unlock_minor,
        "225"
    );
    assert_eq!(reserve.decision().resulting_slash_reserved_minor, "75");

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should exist after reserve");
    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 225);
    assert_eq!(account.pending_unlock_minor(), 0);
    assert_eq!(account.slash_reserved_minor(), 75);
    assert_eq!(
        account.status(),
        QuickChainBondAccountStatusV1::FrozenEvidenceOnly
    );

    let release = state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::ReleaseSlashReserve,
            "25",
            "release",
        ))
        .expect("release should model successfully");

    assert_eq!(release.captured_minor(), 0);
    assert_eq!(release.released_minor(), 25);

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should remain after release");
    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 250);
    assert_eq!(account.slash_reserved_minor(), 50);
    assert_eq!(
        account.status(),
        QuickChainBondAccountStatusV1::FrozenEvidenceOnly
    );
    assert_eq!(
        state.total_available_to_unlock_minor()
            + state.total_pending_unlock_minor()
            + state.total_slash_reserved_minor(),
        state.total_locked_minor()
    );
}

#[test]
fn capture_can_only_consume_already_reserved_amount() {
    let mut state = opened_state("300");

    let before = state.clone();
    let error = state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::CaptureSlashReserve,
            "1",
            "capture-without-reserve",
        ))
        .expect_err("capture without reserved amount must reject");

    assert!(matches!(
        error,
        QuickChainBondLedgerError::InsufficientSlashReserved {
            reserved_minor: 0,
            required_minor: 1
        }
    ));
    assert_eq!(state, before, "rejected capture must not mutate");

    state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::ReserveSlash,
            "90",
            "reserve",
        ))
        .expect("reserve should model successfully");

    let capture = state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::CaptureSlashReserve,
            "40",
            "capture",
        ))
        .expect("capture from reserved amount should model successfully");

    assert_eq!(capture.captured_minor(), 40);
    assert_eq!(capture.released_minor(), 0);
    assert_eq!(capture.decision().resulting_locked_minor, "260");
    assert_eq!(
        capture.decision().resulting_available_to_unlock_minor,
        "210"
    );
    assert_eq!(capture.decision().resulting_slash_reserved_minor, "50");

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should remain after partial capture");
    assert_eq!(account.locked_minor(), 260);
    assert_eq!(account.available_to_unlock_minor(), 210);
    assert_eq!(account.pending_unlock_minor(), 0);
    assert_eq!(account.slash_reserved_minor(), 50);
    assert_eq!(
        account.status(),
        QuickChainBondAccountStatusV1::FrozenEvidenceOnly
    );
}

#[test]
fn reserve_cannot_bypass_pending_unlock_or_overreserve_available_bond() {
    let mut state = opened_state("300");

    state
        .apply_explicit_bond_intent(&unlock_intent("125"), 0)
        .expect("unlock request should model successfully");

    assert_eq!(state.total_available_to_unlock_minor(), 175);
    assert_eq!(state.total_pending_unlock_minor(), 125);

    let before = state.clone();
    let error = state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::ReserveSlash,
            "176",
            "overreserve",
        ))
        .expect_err("reserve above available bonded amount must reject");

    assert!(matches!(
        error,
        QuickChainBondLedgerError::InsufficientBondAvailable {
            available_minor: 175,
            required_minor: 176
        }
    ));
    assert_eq!(state, before, "overreserve must not partially mutate");
}

#[test]
fn enforcement_rejects_invalid_policy_gate_before_mutation() {
    let mut state = opened_state("300");

    let mut invalid = enforcement(
        QuickChainBondEnforcementKindV1::ReserveSlash,
        "10",
        "missing-policy",
    );
    invalid.policy_decision_ref.clear();

    let before = state.clone();
    let error = state
        .apply_controlled_bond_enforcement(&invalid)
        .expect_err("invalid policy gate must reject");

    assert!(matches!(
        error,
        QuickChainBondLedgerError::InvalidBondEnforcementIntent(_)
    ));
    assert_eq!(state, before, "invalid DTO rejection must not mutate");
}

#[test]
fn capture_to_zero_closes_internal_bond_account_without_minting_rewards() {
    let mut state = opened_state("50");

    state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::ReserveSlash,
            "50",
            "reserve-all",
        ))
        .expect("reserve all should model successfully");

    let capture = state
        .apply_controlled_bond_enforcement(&enforcement(
            QuickChainBondEnforcementKindV1::CaptureSlashReserve,
            "50",
            "capture-all",
        ))
        .expect("capture all should model successfully");

    assert_eq!(capture.captured_minor(), 50);
    assert_eq!(capture.decision().resulting_locked_minor, "0");
    assert_eq!(capture.decision().resulting_available_to_unlock_minor, "0");
    assert_eq!(capture.decision().resulting_pending_unlock_minor, "0");
    assert_eq!(capture.decision().resulting_slash_reserved_minor, "0");

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("closed bond account remains auditable");
    assert_eq!(account.locked_minor(), 0);
    assert_eq!(account.available_to_unlock_minor(), 0);
    assert_eq!(account.pending_unlock_minor(), 0);
    assert_eq!(account.slash_reserved_minor(), 0);
    assert_eq!(account.status(), QuickChainBondAccountStatusV1::Closed);
    assert_eq!(state.total_locked_minor(), 0);
}
