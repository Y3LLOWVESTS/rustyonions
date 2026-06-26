#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 4 Round 1 internal bond accounting model tests.
//! RO:WHY — ECON/GOV: bond math must conserve before any live slash, wallet route, or public market exists.
//! RO:INTERACTS — ron-ledger quickchain::bond_accounting and ron-proto bond DTOs.
//! RO:INVARIANTS — explicit owner boundary; COW rejection; ROC-only; no automatic slashing; no public staking/liquidity.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — tests prove model math only, not wallet mutation or live enforcement.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_accounting.

use ron_ledger::quickchain::{
    evaluate_slash_evidence_noop, QuickChainBondAccountingState, QuickChainBondLedgerError,
};
use ron_proto::{
    quickchain::{
        QuickChainBondAccountStatusV1, QuickChainBondIntentKindV1,
        QuickChainBondLifecycleDecisionStatusV1, QuickChainBondLifecycleOperationV1,
        QuickChainBondLifecycleRejectionCodeV1, QuickChainSlashEvidenceKindV1,
        QuickChainSlashEvidenceV1, QuickChainValidatorBondIntentV1, QUICKCHAIN_BOND_ASSET_ROC,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_SLASH_EVIDENCE_SCHEMA,
        QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch:phase4-r1";
const BOND_ACCOUNT_ID: &str = "bond:validator-alice";
const VALIDATOR_ID: &str = "validator:alice";
const OWNER_ACCOUNT_ID: &str = "acct:operator-alice";

fn cid(ch: char) -> ContentId {
    format!("b3:{}", ch.to_string().repeat(64))
        .parse()
        .expect("test ContentId should be valid")
}

fn intent(
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

fn unlock_intent(amount_minor: &str, suffix: &str) -> QuickChainValidatorBondIntentV1 {
    let mut intent = intent(
        QuickChainBondIntentKindV1::RequestUnlock,
        amount_minor,
        suffix,
    );
    intent.unlock_epoch_id = Some("epoch:phase4-r1-unlock".to_owned());
    intent
}

fn slash_evidence() -> QuickChainSlashEvidenceV1 {
    QuickChainSlashEvidenceV1 {
        schema: QUICKCHAIN_SLASH_EVIDENCE_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_set_hash: cid('a'),
        evidence_id: "slash-evidence:alice-001".to_owned(),
        validator_id: VALIDATOR_ID.to_owned(),
        evidence_kind: QuickChainSlashEvidenceKindV1::ValidatorEquivocation,
        evidence_ref: "evidence:equivocation:alice-001".to_owned(),
        submitter_ref: "passport:main:watcher".to_owned(),
        observed_at_ms: 1_800_000_000_000,
        recommended_freeze: true,
        recommended_amount_minor: Some("10".to_owned()),
    }
}

#[test]
fn open_and_increase_bond_require_explicit_owner_available_boundary() {
    let mut state = QuickChainBondAccountingState::new();

    let error = state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::OpenBond, "250", "open"),
            249,
        )
        .expect_err("insufficient owner boundary must reject");

    assert!(matches!(
        error,
        QuickChainBondLedgerError::InsufficientOwnerAvailable {
            available_minor: 249,
            required_minor: 250
        }
    ));
    assert_eq!(state.account_count(), 0, "rejection must be copy-on-write");

    let open = state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::OpenBond, "250", "open"),
            1_000,
        )
        .expect("explicit open bond should model successfully");

    assert_eq!(open.owner_debit_minor(), 250);
    assert_eq!(open.account().locked_minor, "250");
    assert_eq!(state.total_locked_minor(), 250);
    assert_eq!(state.total_available_to_unlock_minor(), 250);
    assert_eq!(state.total_pending_unlock_minor(), 0);
    assert_eq!(state.total_slash_reserved_minor(), 0);

    let increase = state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::IncreaseBond, "50", "increase"),
            50,
        )
        .expect("explicit increase should model successfully");

    assert_eq!(increase.owner_debit_minor(), 50);

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should exist after open/increase");

    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 300);
    assert_eq!(account.pending_unlock_minor(), 0);
    assert_eq!(account.slash_reserved_minor(), 0);
    assert_eq!(account.status(), QuickChainBondAccountStatusV1::Active);
    assert_eq!(state.total_locked_minor(), 300);
}

#[test]
fn request_and_cancel_unlock_conserve_locked_components() {
    let mut state = QuickChainBondAccountingState::new();

    state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::OpenBond, "300", "open"),
            1_000,
        )
        .expect("open bond should model successfully");

    let unlock = state
        .apply_explicit_bond_intent(&unlock_intent("100", "unlock"), 0)
        .expect("unlock request should model successfully");

    assert_eq!(unlock.owner_debit_minor(), 0);

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should exist after unlock request");

    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 200);
    assert_eq!(account.pending_unlock_minor(), 100);
    assert_eq!(account.slash_reserved_minor(), 0);
    assert_eq!(
        account.status(),
        QuickChainBondAccountStatusV1::UnlockPending
    );

    let cancel = state
        .apply_explicit_bond_intent(
            &intent(
                QuickChainBondIntentKindV1::CancelUnlockRequest,
                "40",
                "cancel-unlock",
            ),
            0,
        )
        .expect("cancel unlock should model successfully");

    assert_eq!(cancel.owner_debit_minor(), 0);

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should remain present");

    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 240);
    assert_eq!(account.pending_unlock_minor(), 60);
    assert_eq!(account.slash_reserved_minor(), 0);
    assert_eq!(state.total_locked_minor(), 300);
    assert_eq!(
        state.total_available_to_unlock_minor()
            + state.total_pending_unlock_minor()
            + state.total_slash_reserved_minor(),
        state.total_locked_minor()
    );
}

#[test]
fn over_unlock_and_over_cancel_reject_without_partial_mutation() {
    let mut state = QuickChainBondAccountingState::new();

    state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::OpenBond, "100", "open"),
            1_000,
        )
        .expect("open bond should model successfully");

    let before = state.clone();

    let error = state
        .apply_explicit_bond_intent(&unlock_intent("101", "over-unlock"), 0)
        .expect_err("over-unlock must reject");

    assert!(matches!(
        error,
        QuickChainBondLedgerError::InsufficientBondAvailable {
            available_minor: 100,
            required_minor: 101
        }
    ));
    assert_eq!(state, before, "over-unlock rejection must not mutate");

    state
        .apply_explicit_bond_intent(&unlock_intent("25", "unlock"), 0)
        .expect("unlock request should model successfully");

    let before_cancel = state.clone();

    let error = state
        .apply_explicit_bond_intent(
            &intent(
                QuickChainBondIntentKindV1::CancelUnlockRequest,
                "26",
                "over-cancel",
            ),
            0,
        )
        .expect_err("over-cancel must reject");

    assert!(matches!(
        error,
        QuickChainBondLedgerError::InsufficientPendingUnlock {
            pending_minor: 25,
            required_minor: 26
        }
    ));
    assert_eq!(
        state, before_cancel,
        "over-cancel rejection must not mutate"
    );
}

#[test]
fn slash_evidence_is_noop_rejected_and_does_not_reserve_or_slash() {
    let mut state = QuickChainBondAccountingState::new();

    state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::OpenBond, "300", "open"),
            1_000,
        )
        .expect("open bond should model successfully");

    let before = state.clone();

    let decision = evaluate_slash_evidence_noop(&slash_evidence())
        .expect("valid slash evidence should produce no-op decision");

    assert_eq!(
        decision.operation,
        QuickChainBondLifecycleOperationV1::EvaluateSlashEvidenceNoop
    );
    assert_eq!(
        decision.status,
        QuickChainBondLifecycleDecisionStatusV1::Rejected
    );
    assert_eq!(
        decision.rejection_code,
        Some(QuickChainBondLifecycleRejectionCodeV1::AutomaticSlashingForbidden)
    );
    assert_eq!(decision.amount_minor.as_deref(), Some("10"));

    assert_eq!(
        state, before,
        "slash evidence no-op must not mutate bond state"
    );

    let account = state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should remain present");

    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 300);
    assert_eq!(account.pending_unlock_minor(), 0);
    assert_eq!(account.slash_reserved_minor(), 0);
}

#[test]
fn bond_model_rejects_mismatched_owner_validator_and_asset() {
    let mut state = QuickChainBondAccountingState::new();

    state
        .apply_explicit_bond_intent(
            &intent(QuickChainBondIntentKindV1::OpenBond, "100", "open"),
            1_000,
        )
        .expect("open bond should model successfully");

    let mut wrong_owner = intent(QuickChainBondIntentKindV1::IncreaseBond, "1", "wrong-owner");
    wrong_owner.actor_account_id = "acct:operator-bob".to_owned();

    assert!(matches!(
        state
            .apply_explicit_bond_intent(&wrong_owner, 1)
            .expect_err("wrong owner must reject"),
        QuickChainBondLedgerError::OwnerAccountMismatch
    ));

    let mut wrong_validator = intent(
        QuickChainBondIntentKindV1::IncreaseBond,
        "1",
        "wrong-validator",
    );
    wrong_validator.validator_id = "validator:bob".to_owned();

    assert!(matches!(
        state
            .apply_explicit_bond_intent(&wrong_validator, 1)
            .expect_err("wrong validator must reject"),
        QuickChainBondLedgerError::ValidatorMismatch
    ));

    let mut wrong_asset = intent(QuickChainBondIntentKindV1::IncreaseBond, "1", "wrong-asset");
    wrong_asset.asset = "rox".to_owned();

    assert!(matches!(
        state
            .apply_explicit_bond_intent(&wrong_asset, 1)
            .expect_err("wrong asset must reject"),
        QuickChainBondLedgerError::InvalidBondIntent(_)
    ));
}
