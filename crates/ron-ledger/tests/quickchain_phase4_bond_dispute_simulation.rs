#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 4 Round 2 disputed-bond replay simulation tests.
//! RO:WHY — ECON/GOV: challenge/freeze/appeal flow must be deterministic before live slash enforcement exists.
//! RO:INTERACTS — ron-ledger quickchain::bond_dispute and ron-proto bond dispute DTOs.
//! RO:INVARIANTS — replayable; sequence-bound; no one-step irreversible slash; no mutation of Phase 4 bond accounting state.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — tests prove simulation only, not live slash, wallet mutation, staking, liquidity, bridge, or settlement.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test quickchain_phase4_bond_dispute_simulation.

use ron_ledger::quickchain::{
    evaluate_bond_dispute_event_simulation, replay_bond_dispute_simulation,
    QuickChainBondAccountingState, QuickChainBondDisputeSimulationError,
};
use ron_proto::{
    quickchain::{
        QuickChainBondDisputeEventKindV1, QuickChainBondDisputeEventV1,
        QuickChainBondDisputeRejectionCodeV1, QuickChainBondDisputeStatusV1,
        QuickChainBondDisputeV1, QuickChainBondDisputeWindowV1, QuickChainBondIntentKindV1,
        QuickChainValidatorBondIntentV1, QUICKCHAIN_BOND_ASSET_ROC,
        QUICKCHAIN_BOND_DISPUTE_EVENT_SCHEMA, QUICKCHAIN_BOND_DISPUTE_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA,
    },
    ContentId,
};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch:phase4-r2";
const DISPUTE_ID: &str = "bond-dispute:alice-001";
const BOND_ACCOUNT_ID: &str = "bond:validator-alice";
const VALIDATOR_ID: &str = "validator:alice";
const OWNER_ACCOUNT_ID: &str = "acct:operator-alice";

fn cid(ch: char) -> ContentId {
    format!("b3:{}", ch.to_string().repeat(64))
        .parse()
        .expect("test ContentId should be valid")
}

fn challenge_window() -> QuickChainBondDisputeWindowV1 {
    QuickChainBondDisputeWindowV1 {
        start_epoch: 10,
        end_epoch: 20,
    }
}

fn appeal_window() -> QuickChainBondDisputeWindowV1 {
    QuickChainBondDisputeWindowV1 {
        start_epoch: 21,
        end_epoch: 30,
    }
}

fn initial_dispute() -> QuickChainBondDisputeV1 {
    QuickChainBondDisputeV1 {
        schema: QUICKCHAIN_BOND_DISPUTE_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        dispute_id: DISPUTE_ID.to_owned(),
        bond_account_id: BOND_ACCOUNT_ID.to_owned(),
        validator_id: VALIDATOR_ID.to_owned(),
        validator_set_hash: cid('a'),
        evidence_id: "slash-evidence:alice-001".to_owned(),
        challenger_ref: "passport:watcher".to_owned(),
        status: QuickChainBondDisputeStatusV1::ChallengeOpen,
        challenge_window: challenge_window(),
        appeal_window: None,
        disputed_amount_minor: "100".to_owned(),
        frozen_amount_minor: "0".to_owned(),
        last_dispute_sequence: 1,
    }
}

fn freeze_event() -> QuickChainBondDisputeEventV1 {
    QuickChainBondDisputeEventV1 {
        schema: QUICKCHAIN_BOND_DISPUTE_EVENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        dispute_id: DISPUTE_ID.to_owned(),
        bond_account_id: BOND_ACCOUNT_ID.to_owned(),
        validator_id: VALIDATOR_ID.to_owned(),
        event_sequence: 2,
        event_kind: QuickChainBondDisputeEventKindV1::FreezePendingAppeal,
        actor_ref: "governance:reviewer".to_owned(),
        occurred_epoch: 15,
        amount_minor: Some("40".to_owned()),
        appeal_window: Some(appeal_window()),
        rejection_code: None,
    }
}

fn appeal_event() -> QuickChainBondDisputeEventV1 {
    QuickChainBondDisputeEventV1 {
        event_sequence: 3,
        event_kind: QuickChainBondDisputeEventKindV1::SubmitAppeal,
        actor_ref: "passport:validator-alice".to_owned(),
        occurred_epoch: 25,
        amount_minor: None,
        appeal_window: None,
        rejection_code: None,
        ..freeze_event()
    }
}

fn reject_irreversible_event() -> QuickChainBondDisputeEventV1 {
    QuickChainBondDisputeEventV1 {
        event_sequence: 4,
        event_kind: QuickChainBondDisputeEventKindV1::RejectIrreversibleSlash,
        actor_ref: "governance:reviewer".to_owned(),
        occurred_epoch: 26,
        amount_minor: None,
        appeal_window: None,
        rejection_code: Some(
            QuickChainBondDisputeRejectionCodeV1::OneStepIrreversibleSlashForbidden,
        ),
        ..freeze_event()
    }
}

fn bond_intent() -> QuickChainValidatorBondIntentV1 {
    QuickChainValidatorBondIntentV1 {
        schema: QUICKCHAIN_VALIDATOR_BOND_INTENT_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_owned(),
        epoch_id: EPOCH_ID.to_owned(),
        validator_id: VALIDATOR_ID.to_owned(),
        bond_account_id: BOND_ACCOUNT_ID.to_owned(),
        actor_account_id: OWNER_ACCOUNT_ID.to_owned(),
        intent_id: "bond-intent:alice-open".to_owned(),
        idempotency_key: "idem:bond-alice-open".to_owned(),
        kind: QuickChainBondIntentKindV1::OpenBond,
        asset: QUICKCHAIN_BOND_ASSET_ROC.to_owned(),
        amount_minor: Some("300".to_owned()),
        unlock_epoch_id: None,
        governance_approval_ref: Some("gov:phase4-bond-alpha".to_owned()),
    }
}

#[test]
fn dispute_simulation_replays_freeze_appeal_and_rejected_irreversible_slash() {
    let initial = initial_dispute();

    let frozen = evaluate_bond_dispute_event_simulation(&initial, &freeze_event())
        .expect("freeze event should simulate");

    assert_eq!(
        frozen.status,
        QuickChainBondDisputeStatusV1::FrozenPendingAppeal
    );
    assert_eq!(frozen.frozen_amount_minor, "40");
    assert_eq!(frozen.last_dispute_sequence, 2);

    let appealed = evaluate_bond_dispute_event_simulation(&frozen, &appeal_event())
        .expect("appeal event should simulate");

    assert_eq!(appealed.status, QuickChainBondDisputeStatusV1::AppealOpen);
    assert_eq!(appealed.frozen_amount_minor, "40");
    assert_eq!(appealed.last_dispute_sequence, 3);

    let final_state =
        evaluate_bond_dispute_event_simulation(&appealed, &reject_irreversible_event())
            .expect("irreversible slash rejection should simulate as terminal no-slash authority");

    assert_eq!(
        final_state.status,
        QuickChainBondDisputeStatusV1::ResolvedSlashRejected
    );
    assert_eq!(final_state.frozen_amount_minor, "0");
    assert_eq!(final_state.last_dispute_sequence, 4);
    assert!(final_state.is_terminal());

    assert_eq!(
        initial.status,
        QuickChainBondDisputeStatusV1::ChallengeOpen,
        "simulation must be copy-on-write"
    );
}

#[test]
fn replay_is_deterministic_and_sequence_bound() {
    let initial = initial_dispute();

    let events = vec![freeze_event(), appeal_event(), reject_irreversible_event()];

    let first =
        replay_bond_dispute_simulation(&initial, &events).expect("first replay should pass");
    let second =
        replay_bond_dispute_simulation(&initial, &events).expect("second replay should pass");

    assert_eq!(first, second);
    assert_eq!(
        first.status,
        QuickChainBondDisputeStatusV1::ResolvedSlashRejected
    );

    let mut bad_sequence = freeze_event();
    bad_sequence.event_sequence = 9;

    assert!(matches!(
        evaluate_bond_dispute_event_simulation(&initial, &bad_sequence)
            .expect_err("non-adjacent sequence must reject"),
        QuickChainBondDisputeSimulationError::SequenceMismatch {
            expected: 2,
            actual: 9
        }
    ));
}

#[test]
fn dispute_windows_are_enforced() {
    let initial = initial_dispute();

    let mut late_freeze = freeze_event();
    late_freeze.occurred_epoch = 21;

    assert!(matches!(
        evaluate_bond_dispute_event_simulation(&initial, &late_freeze)
            .expect_err("late freeze must reject"),
        QuickChainBondDisputeSimulationError::ChallengeWindowClosed
    ));

    let frozen = evaluate_bond_dispute_event_simulation(&initial, &freeze_event())
        .expect("freeze should simulate");

    let mut late_appeal = appeal_event();
    late_appeal.occurred_epoch = 31;

    assert!(matches!(
        evaluate_bond_dispute_event_simulation(&frozen, &late_appeal)
            .expect_err("late appeal must reject"),
        QuickChainBondDisputeSimulationError::AppealWindowClosed
    ));
}

#[test]
fn terminal_disputes_reject_later_events() {
    let initial = initial_dispute();

    let final_state = replay_bond_dispute_simulation(
        &initial,
        &[freeze_event(), appeal_event(), reject_irreversible_event()],
    )
    .expect("replay should reach terminal state");

    assert!(matches!(
        evaluate_bond_dispute_event_simulation(&final_state, &reject_irreversible_event())
            .expect_err("terminal dispute must reject future events"),
        QuickChainBondDisputeSimulationError::DisputeAlreadyTerminal
    ));
}

#[test]
fn dispute_simulation_does_not_mutate_bond_accounting_state() {
    let mut bond_state = QuickChainBondAccountingState::new();

    bond_state
        .apply_explicit_bond_intent(&bond_intent(), 1_000)
        .expect("open bond should model successfully");

    let before = bond_state.clone();

    let final_dispute = replay_bond_dispute_simulation(
        &initial_dispute(),
        &[freeze_event(), appeal_event(), reject_irreversible_event()],
    )
    .expect("dispute simulation should replay");

    assert_eq!(
        final_dispute.status,
        QuickChainBondDisputeStatusV1::ResolvedSlashRejected
    );

    assert_eq!(
        bond_state, before,
        "dispute simulation must not mutate bond accounting model"
    );

    let account = bond_state
        .account(BOND_ACCOUNT_ID)
        .expect("bond account should remain present");

    assert_eq!(account.locked_minor(), 300);
    assert_eq!(account.available_to_unlock_minor(), 300);
    assert_eq!(account.pending_unlock_minor(), 0);
    assert_eq!(account.slash_reserved_minor(), 0);
}
