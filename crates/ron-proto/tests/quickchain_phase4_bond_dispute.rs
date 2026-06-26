//! RO:WHAT — Phase 4 Round 2 bond dispute/challenge DTO validation tests.
//! RO:WHY — ECON/GOV: slash simulation needs explicit windows, appeal/freeze states, and no irreversible slash authority.
//! RO:INTERACTS — ron_proto::quickchain::bond_dispute DTOs.
//! RO:INVARIANTS — deny unknown fields; windows explicit; frozen amounts conserve; one-step irreversible slash rejects.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — tests prove DTO shape only, not wallet mutation, live slash, staking, liquidity, bridge, or settlement.
//! RO:TEST — cargo test -p ron-proto --test quickchain_phase4_bond_dispute.

use ron_proto::{
    quickchain::{
        QuickChainBondDisputeEventKindV1, QuickChainBondDisputeEventV1,
        QuickChainBondDisputeRejectionCodeV1, QuickChainBondDisputeStatusV1,
        QuickChainBondDisputeV1, QuickChainBondDisputeWindowV1, QuickChainValidationError,
        QUICKCHAIN_BOND_DISPUTE_EVENT_SCHEMA, QUICKCHAIN_BOND_DISPUTE_SCHEMA,
        QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};
use serde_json::{json, Value};

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch:phase4-r2";
const DISPUTE_ID: &str = "bond-dispute:alice-001";
const BOND_ACCOUNT_ID: &str = "bond:validator-alice";
const VALIDATOR_ID: &str = "validator:alice";

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

fn open_dispute() -> QuickChainBondDisputeV1 {
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

#[test]
fn dispute_open_state_validates_explicit_challenge_window() {
    open_dispute()
        .validate()
        .expect("open dispute with explicit challenge window should validate");

    let mut missing_sequence = open_dispute();
    missing_sequence.last_dispute_sequence = 0;

    assert!(matches!(
        missing_sequence.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "last_dispute_sequence",
            ..
        })
    ));

    let mut bad_window = open_dispute();
    bad_window.challenge_window = QuickChainBondDisputeWindowV1 {
        start_epoch: 20,
        end_epoch: 10,
    };

    assert!(matches!(
        bad_window.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "challenge_window",
            ..
        })
    ));
}

#[test]
fn dispute_status_requires_matching_appeal_and_frozen_state() {
    let mut frozen = open_dispute();
    frozen.status = QuickChainBondDisputeStatusV1::FrozenPendingAppeal;
    frozen.appeal_window = Some(appeal_window());
    frozen.frozen_amount_minor = "40".to_owned();

    frozen
        .validate()
        .expect("frozen dispute with appeal window and frozen amount should validate");

    let mut no_window = frozen.clone();
    no_window.appeal_window = None;

    assert!(matches!(
        no_window.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "appeal_window",
            ..
        })
    ));

    let mut over_frozen = frozen;
    over_frozen.frozen_amount_minor = "101".to_owned();

    assert!(matches!(
        over_frozen.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "frozen_amount_minor",
            ..
        })
    ));

    let mut terminal = open_dispute();
    terminal.status = QuickChainBondDisputeStatusV1::ResolvedSlashRejected;
    terminal.frozen_amount_minor = "0".to_owned();

    terminal
        .validate()
        .expect("terminal slash-rejected state should validate with no frozen amount");
    assert!(terminal.is_terminal());
}

#[test]
fn dispute_events_validate_kind_specific_requirements() {
    freeze_event()
        .validate()
        .expect("freeze event with amount and appeal window should validate");

    let mut missing_amount = freeze_event();
    missing_amount.amount_minor = None;

    assert!(matches!(
        missing_amount.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "amount_minor",
            ..
        })
    ));

    let appeal = QuickChainBondDisputeEventV1 {
        event_sequence: 3,
        event_kind: QuickChainBondDisputeEventKindV1::SubmitAppeal,
        actor_ref: "passport:validator-alice".to_owned(),
        occurred_epoch: 25,
        amount_minor: None,
        appeal_window: None,
        rejection_code: None,
        ..freeze_event()
    };

    appeal
        .validate()
        .expect("appeal event without amount or rejection code should validate");

    let reject_irreversible = QuickChainBondDisputeEventV1 {
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
    };

    reject_irreversible
        .validate()
        .expect("irreversible slash rejection requires explicit forbidden code");

    let mut bad_reject = reject_irreversible;
    bad_reject.rejection_code =
        Some(QuickChainBondDisputeRejectionCodeV1::GovernanceApprovalRequired);

    assert!(matches!(
        bad_reject.validate(),
        Err(QuickChainValidationError::InvalidField {
            field: "rejection_code",
            ..
        })
    ));
}

#[test]
fn bond_dispute_dtos_reject_unknown_fields() {
    let mut dispute: Value =
        serde_json::to_value(open_dispute()).expect("dispute should serialize");
    dispute
        .as_object_mut()
        .expect("dispute JSON should be object")
        .insert("execute_slashing".to_owned(), json!(true));

    serde_json::from_value::<QuickChainBondDisputeV1>(dispute)
        .expect_err("unknown dispute fields must reject");

    let mut event: Value = serde_json::to_value(freeze_event()).expect("event should serialize");
    event
        .as_object_mut()
        .expect("event JSON should be object")
        .insert("commit_slash_decision".to_owned(), json!("forbidden"));

    serde_json::from_value::<QuickChainBondDisputeEventV1>(event)
        .expect_err("unknown dispute event fields must reject");
}
