#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Internal ROC Beta Phase 2 replay, conservation, terminality, and ordering proof.
//! RO:WHY — ECON/RES: accepted paid-flow histories must replay deterministically and conserve internal ROC.
//! RO:INTERACTS — QuickChainAtomicState, accepted replay, balance transitions, hold lifecycle, and ron-proto intents.
//! RO:INVARIANTS — no new mutation path; wallet/ledger truth only; explicit epochs; no wall-clock/DB-order replay authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — receipt refs and policy decisions are inert test inputs; no bridge, staking, ROX, Solana, or external settlement.
//! RO:TEST — this file and crates/ron-ledger/scripts/dev-internal-roc-beta-phase2-preflight.sh.

use ron_ledger::quickchain::{
    QuickChainAcceptedOperation, QuickChainAtomicState, QuickChainExecutionError,
    QuickChainHoldEpochInput, QuickChainHoldError, QuickChainHoldTerminalStatus,
    QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

#[allow(clippy::too_many_arguments)]
fn intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: Option<&str>,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_string(),
        op_class,
        actor_account_id: actor.to_string(),
        counterparty_account_id: counterparty.map(str::to_string),
        amount_minor: Some(amount_minor.to_string()),
        hold_id: hold_id.map(str::to_string),
        account_sequence: None,
        produced_at_ms,
    }
}

fn commit_balance(
    state: &mut QuickChainAtomicState,
    submitted: &QuickChainOperationIntentV1,
    supply_decision: QuickChainSupplyDecision,
    receipt_txid: &str,
) -> QuickChainAcceptedOperation {
    let outcome = state
        .execute_balance_operation(submitted, supply_decision, receipt_txid)
        .expect("balance operation should commit");

    assert!(outcome.is_committed());

    QuickChainAcceptedOperation::balance(outcome.record().clone(), supply_decision)
}

fn commit_hold(
    state: &mut QuickChainAtomicState,
    submitted: &QuickChainOperationIntentV1,
    epoch_input: QuickChainHoldEpochInput,
    receipt_txid: &str,
) -> QuickChainAcceptedOperation {
    let outcome = state
        .execute_hold_operation(submitted, epoch_input, receipt_txid)
        .expect("hold operation should commit");

    assert!(outcome.is_committed());

    QuickChainAcceptedOperation::hold(outcome.record().clone(), epoch_input)
}

fn assert_replay_matches(
    live: &QuickChainAtomicState,
    accepted: &[QuickChainAcceptedOperation],
) -> QuickChainAtomicState {
    let replayed = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        accepted,
        live.accepted_replay_boundary(),
    )
    .expect("accepted history should replay to the live boundary");

    assert_eq!(&replayed, live);
    assert_eq!(
        replayed.state_snapshot().expect("snapshot should capture"),
        live.state_snapshot().expect("snapshot should capture")
    );

    replayed
}

#[test]
fn internal_roc_beta_paid_flow_replay_equality() {
    let mut live = QuickChainAtomicState::new();
    let mut accepted = Vec::new();

    let funding = intent(
        '0',
        "idem:internal-roc-beta-phase2:fund-viewer",
        QuickChainOperationClassV1::Issue,
        "account:viewer-a",
        None,
        "1000",
        None,
        1_800_000_010_000,
    );

    accepted.push(commit_balance(
        &mut live,
        &funding,
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:phase2:fund-viewer",
    ));

    let paid_transfers = [
        ("paid_image", '1', "10"),
        ("paid_site", '2', "11"),
        ("paid_site_visit", '3', "12"),
        ("paid_post", '4', "13"),
        ("paid_comment", '5', "14"),
        ("paid_article", '6', "15"),
        ("paid_content_view", '7', "16"),
    ];

    for (action, operation_hex_digit, amount_minor) in paid_transfers {
        let submitted = intent(
            operation_hex_digit,
            &format!("idem:internal-roc-beta-phase2:{action}"),
            QuickChainOperationClassV1::Transfer,
            "account:viewer-a",
            Some("account:creator-a"),
            amount_minor,
            None,
            1_800_000_010_000 + u64::from(operation_hex_digit.to_digit(16).unwrap()),
        );

        accepted.push(commit_balance(
            &mut live,
            &submitted,
            QuickChainSupplyDecision::NoSupplyChange,
            &format!("tx:roc:phase2:{action}"),
        ));
    }

    let capture_hold = hold_id('1');
    let release_hold = hold_id('2');
    let expire_hold = hold_id('3');

    let open_capture = intent(
        '8',
        "idem:internal-roc-beta-phase2:hold-capture-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:viewer-a",
        Some("account:creator-a"),
        "90",
        Some(&capture_hold),
        1_800_000_010_008,
    );
    let open_capture_epoch = QuickChainHoldEpochInput::Open {
        created_at_epoch: 1,
        expires_at_epoch: 20,
    };
    accepted.push(commit_hold(
        &mut live,
        &open_capture,
        open_capture_epoch,
        "tx:roc:phase2:hold-capture-open",
    ));

    let capture = intent(
        '9',
        "idem:internal-roc-beta-phase2:hold-capture-terminal",
        QuickChainOperationClassV1::HoldCapture,
        "account:viewer-a",
        Some("account:creator-a"),
        "70",
        Some(&capture_hold),
        1_800_000_010_009,
    );
    let capture_epoch = QuickChainHoldEpochInput::Terminal { current_epoch: 10 };
    accepted.push(commit_hold(
        &mut live,
        &capture,
        capture_epoch,
        "tx:roc:phase2:hold-capture-terminal",
    ));

    let open_release = intent(
        'a',
        "idem:internal-roc-beta-phase2:hold-release-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:viewer-a",
        Some("account:creator-a"),
        "30",
        Some(&release_hold),
        1_800_000_010_010,
    );
    let open_release_epoch = QuickChainHoldEpochInput::Open {
        created_at_epoch: 2,
        expires_at_epoch: 20,
    };
    accepted.push(commit_hold(
        &mut live,
        &open_release,
        open_release_epoch,
        "tx:roc:phase2:hold-release-open",
    ));

    let release = intent(
        'b',
        "idem:internal-roc-beta-phase2:hold-release-terminal",
        QuickChainOperationClassV1::HoldRelease,
        "account:viewer-a",
        None,
        "30",
        Some(&release_hold),
        1_800_000_010_011,
    );
    let release_epoch = QuickChainHoldEpochInput::Terminal { current_epoch: 11 };
    accepted.push(commit_hold(
        &mut live,
        &release,
        release_epoch,
        "tx:roc:phase2:hold-release-terminal",
    ));

    let open_expire = intent(
        'c',
        "idem:internal-roc-beta-phase2:hold-expire-open",
        QuickChainOperationClassV1::HoldOpen,
        "account:viewer-a",
        Some("account:creator-a"),
        "40",
        Some(&expire_hold),
        1_800_000_010_012,
    );
    let open_expire_epoch = QuickChainHoldEpochInput::Open {
        created_at_epoch: 3,
        expires_at_epoch: 12,
    };
    accepted.push(commit_hold(
        &mut live,
        &open_expire,
        open_expire_epoch,
        "tx:roc:phase2:hold-expire-open",
    ));

    let expire = intent(
        'd',
        "idem:internal-roc-beta-phase2:hold-expire-terminal",
        QuickChainOperationClassV1::HoldExpire,
        "account:viewer-a",
        None,
        "40",
        Some(&expire_hold),
        1_800_000_010_013,
    );
    let expire_epoch = QuickChainHoldEpochInput::Terminal { current_epoch: 12 };
    accepted.push(commit_hold(
        &mut live,
        &expire,
        expire_epoch,
        "tx:roc:phase2:hold-expire-terminal",
    ));

    let replayed = assert_replay_matches(&live, &accepted);

    assert_eq!(replayed.operation_count(), accepted.len());
    assert_eq!(replayed.current_supply_minor(), live.current_supply_minor());
    assert_eq!(replayed.active_hold(&capture_hold), None);
    assert_eq!(replayed.active_hold(&release_hold), None);
    assert_eq!(replayed.active_hold(&expire_hold), None);

    assert_eq!(
        replayed
            .terminal_hold(&capture_hold)
            .expect("captured hold should be terminal")
            .status(),
        QuickChainHoldTerminalStatus::Captured
    );
    assert_eq!(
        replayed
            .terminal_hold(&release_hold)
            .expect("released hold should be terminal")
            .status(),
        QuickChainHoldTerminalStatus::Released
    );
    assert_eq!(
        replayed
            .terminal_hold(&expire_hold)
            .expect("expired hold should be terminal")
            .status(),
        QuickChainHoldTerminalStatus::Expired
    );

    for accepted_operation in &accepted {
        let operation_id = &accepted_operation.record().intent().operation_id;
        let replayed_record = replayed
            .committed_operation(operation_id)
            .expect("accepted operation should be indexed after replay");

        assert_eq!(replayed_record, accepted_operation.record());
        assert_eq!(
            replayed_record.receipt_txid(),
            accepted_operation.trusted_receipt_txid()
        );
    }
}

#[test]
fn internal_roc_beta_balance_conservation() {
    let mut state = QuickChainAtomicState::new();

    let issue = intent(
        '0',
        "idem:internal-roc-beta-phase2:conservation-issue",
        QuickChainOperationClassV1::Issue,
        "account:payer-a",
        None,
        "1000",
        None,
        1_800_000_020_000,
    );

    let issue_outcome = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase2:conservation-issue",
        )
        .expect("issue should commit");

    assert_eq!(
        issue_outcome
            .transition()
            .expect("fresh issue transition should exist")
            .supply_before,
        0
    );
    assert_eq!(
        issue_outcome
            .transition()
            .expect("fresh issue transition should exist")
            .supply_after,
        1000
    );
    assert_eq!(state.balance_state().total_issued_minor(), 1000);
    assert_eq!(state.balance_state().total_burned_minor(), 0);
    assert_eq!(state.current_supply_minor(), 1000);

    let transfer = intent(
        '1',
        "idem:internal-roc-beta-phase2:conservation-transfer",
        QuickChainOperationClassV1::Transfer,
        "account:payer-a",
        Some("account:creator-a"),
        "125",
        None,
        1_800_000_020_001,
    );

    let transfer_outcome = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:phase2:conservation-transfer",
        )
        .expect("transfer should commit");

    let transfer_transition = transfer_outcome
        .transition()
        .expect("fresh transfer transition should exist");

    assert_eq!(transfer_transition.supply_before, 1000);
    assert_eq!(transfer_transition.supply_after, 1000);
    assert_eq!(state.balance_minor("account:payer-a"), 875);
    assert_eq!(state.balance_minor("account:creator-a"), 125);
    assert_eq!(state.current_supply_minor(), 1000);

    let capture_hold = hold_id('1');
    let open_capture = intent(
        '2',
        "idem:internal-roc-beta-phase2:conservation-open-capture",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "200",
        Some(&capture_hold),
        1_800_000_020_002,
    );

    let open_outcome = state
        .execute_hold_operation(
            &open_capture,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 20,
            },
            "tx:roc:phase2:conservation-open-capture",
        )
        .expect("hold should open");

    let open_transition = open_outcome
        .transition()
        .expect("fresh hold-open transition should exist");

    assert_eq!(open_transition.actor_balance_before, 875);
    assert_eq!(open_transition.actor_balance_after, 875);
    assert_eq!(open_transition.held_minor_before, 0);
    assert_eq!(open_transition.held_minor_after, 200);
    assert_eq!(state.current_supply_minor(), 1000);

    let capture = intent(
        '3',
        "idem:internal-roc-beta-phase2:conservation-capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:payer-a",
        Some("account:creator-a"),
        "150",
        Some(&capture_hold),
        1_800_000_020_003,
    );

    let capture_outcome = state
        .execute_hold_operation(
            &capture,
            QuickChainHoldEpochInput::Terminal { current_epoch: 10 },
            "tx:roc:phase2:conservation-capture",
        )
        .expect("capture should commit");

    let capture_transition = capture_outcome
        .transition()
        .expect("fresh capture transition should exist");

    assert_eq!(capture_transition.actor_balance_before, 875);
    assert_eq!(capture_transition.actor_balance_after, 725);
    assert_eq!(capture_transition.counterparty_balance_before, Some(125));
    assert_eq!(capture_transition.counterparty_balance_after, Some(275));
    assert_eq!(capture_transition.held_minor_after, 0);
    assert_eq!(capture_transition.uncaptured_remainder_minor, 50);
    assert_eq!(state.current_supply_minor(), 1000);
    assert_eq!(
        state
            .terminal_hold(&capture_hold)
            .expect("capture should close the hold")
            .uncaptured_remainder_minor(),
        50
    );

    let release_hold = hold_id('2');
    let open_release = intent(
        '4',
        "idem:internal-roc-beta-phase2:conservation-open-release",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "50",
        Some(&release_hold),
        1_800_000_020_004,
    );

    state
        .execute_hold_operation(
            &open_release,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 2,
                expires_at_epoch: 20,
            },
            "tx:roc:phase2:conservation-open-release",
        )
        .expect("release hold should open");

    let release = intent(
        '5',
        "idem:internal-roc-beta-phase2:conservation-release",
        QuickChainOperationClassV1::HoldRelease,
        "account:payer-a",
        None,
        "50",
        Some(&release_hold),
        1_800_000_020_005,
    );

    state
        .execute_hold_operation(
            &release,
            QuickChainHoldEpochInput::Terminal { current_epoch: 10 },
            "tx:roc:phase2:conservation-release",
        )
        .expect("release should commit");

    assert_eq!(state.current_supply_minor(), 1000);
    assert_eq!(
        state
            .terminal_hold(&release_hold)
            .expect("release should close the hold")
            .status(),
        QuickChainHoldTerminalStatus::Released
    );

    let expire_hold = hold_id('3');
    let open_expire = intent(
        '6',
        "idem:internal-roc-beta-phase2:conservation-open-expire",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "60",
        Some(&expire_hold),
        1_800_000_020_006,
    );

    state
        .execute_hold_operation(
            &open_expire,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 3,
                expires_at_epoch: 12,
            },
            "tx:roc:phase2:conservation-open-expire",
        )
        .expect("expiry hold should open");

    let expire = intent(
        '7',
        "idem:internal-roc-beta-phase2:conservation-expire",
        QuickChainOperationClassV1::HoldExpire,
        "account:payer-a",
        None,
        "60",
        Some(&expire_hold),
        1_800_000_020_007,
    );

    state
        .execute_hold_operation(
            &expire,
            QuickChainHoldEpochInput::Terminal { current_epoch: 12 },
            "tx:roc:phase2:conservation-expire",
        )
        .expect("expiry should commit");

    assert_eq!(state.current_supply_minor(), 1000);
    assert_eq!(
        state
            .terminal_hold(&expire_hold)
            .expect("expiry should close the hold")
            .status(),
        QuickChainHoldTerminalStatus::Expired
    );

    let burn = intent(
        '8',
        "idem:internal-roc-beta-phase2:conservation-burn",
        QuickChainOperationClassV1::Burn,
        "account:payer-a",
        None,
        "25",
        None,
        1_800_000_020_008,
    );

    let burn_outcome = state
        .execute_balance_operation(
            &burn,
            QuickChainSupplyDecision::BurnApproved,
            "tx:roc:phase2:conservation-burn",
        )
        .expect("approved burn should commit");

    let burn_transition = burn_outcome
        .transition()
        .expect("fresh burn transition should exist");

    assert_eq!(burn_transition.supply_before, 1000);
    assert_eq!(burn_transition.supply_after, 975);
    assert_eq!(state.balance_state().total_issued_minor(), 1000);
    assert_eq!(state.balance_state().total_burned_minor(), 25);
    assert_eq!(state.current_supply_minor(), 975);
}

#[test]
fn internal_roc_beta_hold_terminality() {
    let captured_hold = hold_id('1');
    let released_hold = hold_id('2');
    let expired_hold = hold_id('3');
    let past_expiry_hold = hold_id('4');

    let mut state = QuickChainAtomicState::new();

    let funding = intent(
        '0',
        "idem:internal-roc-beta-phase2:terminality-funding",
        QuickChainOperationClassV1::Issue,
        "account:payer-a",
        None,
        "500",
        None,
        1_800_000_030_000,
    );

    state
        .execute_balance_operation(
            &funding,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase2:terminality-funding",
        )
        .expect("funding should commit");

    let open_capture = intent(
        '1',
        "idem:internal-roc-beta-phase2:terminality-open-capture",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "100",
        Some(&captured_hold),
        1_800_000_030_001,
    );

    state
        .execute_hold_operation(
            &open_capture,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 20,
            },
            "tx:roc:phase2:terminality-open-capture",
        )
        .expect("capture hold should open");

    let capture = intent(
        '2',
        "idem:internal-roc-beta-phase2:terminality-capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:payer-a",
        Some("account:creator-a"),
        "100",
        Some(&captured_hold),
        1_800_000_030_002,
    );

    let capture_outcome = state
        .execute_hold_operation(
            &capture,
            QuickChainHoldEpochInput::Terminal { current_epoch: 10 },
            "tx:roc:phase2:terminality-capture",
        )
        .expect("capture should commit");

    let after_capture = state.clone();

    let capture_retry = state
        .execute_hold_operation(
            &capture,
            QuickChainHoldEpochInput::Terminal { current_epoch: 999 },
            "invalid retry txid with spaces",
        )
        .expect("exact capture retry should return original evidence");

    assert!(capture_retry.is_retry());
    assert_eq!(capture_retry.record(), capture_outcome.record());
    assert_eq!(state, after_capture);

    let second_capture = intent(
        '3',
        "idem:internal-roc-beta-phase2:terminality-second-capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:payer-a",
        Some("account:creator-a"),
        "100",
        Some(&captured_hold),
        1_800_000_030_003,
    );

    let second_capture_error = state
        .execute_hold_operation(
            &second_capture,
            QuickChainHoldEpochInput::Terminal { current_epoch: 11 },
            "tx:roc:phase2:terminality-second-capture",
        )
        .expect_err("distinct capture after capture must reject");

    assert!(matches!(
        second_capture_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldAlreadyTerminal { .. })
    ));
    assert_eq!(state, after_capture);

    let open_release = intent(
        '4',
        "idem:internal-roc-beta-phase2:terminality-open-release",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "40",
        Some(&released_hold),
        1_800_000_030_004,
    );

    state
        .execute_hold_operation(
            &open_release,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 2,
                expires_at_epoch: 20,
            },
            "tx:roc:phase2:terminality-open-release",
        )
        .expect("release hold should open");

    let release = intent(
        '5',
        "idem:internal-roc-beta-phase2:terminality-release",
        QuickChainOperationClassV1::HoldRelease,
        "account:payer-a",
        None,
        "40",
        Some(&released_hold),
        1_800_000_030_005,
    );

    state
        .execute_hold_operation(
            &release,
            QuickChainHoldEpochInput::Terminal { current_epoch: 10 },
            "tx:roc:phase2:terminality-release",
        )
        .expect("release should commit");

    let capture_after_release = intent(
        '6',
        "idem:internal-roc-beta-phase2:terminality-capture-after-release",
        QuickChainOperationClassV1::HoldCapture,
        "account:payer-a",
        Some("account:creator-a"),
        "40",
        Some(&released_hold),
        1_800_000_030_006,
    );

    let capture_after_release_error = state
        .execute_hold_operation(
            &capture_after_release,
            QuickChainHoldEpochInput::Terminal { current_epoch: 11 },
            "tx:roc:phase2:terminality-capture-after-release",
        )
        .expect_err("capture after release must reject");

    assert!(matches!(
        capture_after_release_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldAlreadyTerminal { .. })
    ));

    let open_expire = intent(
        '7',
        "idem:internal-roc-beta-phase2:terminality-open-expire",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "50",
        Some(&expired_hold),
        1_800_000_030_007,
    );

    state
        .execute_hold_operation(
            &open_expire,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 3,
                expires_at_epoch: 12,
            },
            "tx:roc:phase2:terminality-open-expire",
        )
        .expect("expiry hold should open");

    let expire = intent(
        '8',
        "idem:internal-roc-beta-phase2:terminality-expire",
        QuickChainOperationClassV1::HoldExpire,
        "account:payer-a",
        None,
        "50",
        Some(&expired_hold),
        1_800_000_030_008,
    );

    state
        .execute_hold_operation(
            &expire,
            QuickChainHoldEpochInput::Terminal { current_epoch: 12 },
            "tx:roc:phase2:terminality-expire",
        )
        .expect("expiry should commit");

    let capture_after_expiry = intent(
        '9',
        "idem:internal-roc-beta-phase2:terminality-capture-after-expiry",
        QuickChainOperationClassV1::HoldCapture,
        "account:payer-a",
        Some("account:creator-a"),
        "50",
        Some(&expired_hold),
        1_800_000_030_009,
    );

    let capture_after_expiry_error = state
        .execute_hold_operation(
            &capture_after_expiry,
            QuickChainHoldEpochInput::Terminal { current_epoch: 13 },
            "tx:roc:phase2:terminality-capture-after-expiry",
        )
        .expect_err("capture after expiry terminal must reject");

    assert!(matches!(
        capture_after_expiry_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldAlreadyTerminal { .. })
    ));

    let open_past_expiry = intent(
        'a',
        "idem:internal-roc-beta-phase2:terminality-open-past-expiry",
        QuickChainOperationClassV1::HoldOpen,
        "account:payer-a",
        Some("account:creator-a"),
        "30",
        Some(&past_expiry_hold),
        1_800_000_030_010,
    );

    state
        .execute_hold_operation(
            &open_past_expiry,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 4,
                expires_at_epoch: 12,
            },
            "tx:roc:phase2:terminality-open-past-expiry",
        )
        .expect("past-expiry test hold should open");

    let capture_past_expiry = intent(
        'b',
        "idem:internal-roc-beta-phase2:terminality-capture-past-expiry",
        QuickChainOperationClassV1::HoldCapture,
        "account:payer-a",
        Some("account:creator-a"),
        "30",
        Some(&past_expiry_hold),
        1_800_000_030_011,
    );

    let before_past_expiry_capture = state.clone();

    let capture_past_expiry_error = state
        .execute_hold_operation(
            &capture_past_expiry,
            QuickChainHoldEpochInput::Terminal { current_epoch: 12 },
            "tx:roc:phase2:terminality-capture-past-expiry",
        )
        .expect_err("capture at or after expiry epoch must reject");

    assert!(matches!(
        capture_past_expiry_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldPastExpiry { .. })
    ));
    assert_eq!(state, before_past_expiry_capture);
}

#[test]
fn internal_roc_beta_replay_order_independence() {
    let mut live = QuickChainAtomicState::new();
    let mut accepted = Vec::new();

    // Intentional mismatch:
    // - operation_id lexical order is transfer -> burn -> issue
    // - produced_at_ms order is transfer -> burn -> issue
    // - accepted ledger order is issue -> transfer -> burn
    //
    // Replay must use the accepted ledger history order, not DB iteration order,
    // operation-id order, or wall-clock/event-time order.
    let issue = intent(
        'f',
        "idem:internal-roc-beta-phase2:ordering-issue",
        QuickChainOperationClassV1::Issue,
        "account:payer-a",
        None,
        "100",
        None,
        1_800_000_040_300,
    );

    let transfer = intent(
        '1',
        "idem:internal-roc-beta-phase2:ordering-transfer",
        QuickChainOperationClassV1::Transfer,
        "account:payer-a",
        Some("account:creator-a"),
        "25",
        None,
        1_800_000_040_100,
    );

    let burn = intent(
        'e',
        "idem:internal-roc-beta-phase2:ordering-burn",
        QuickChainOperationClassV1::Burn,
        "account:payer-a",
        None,
        "5",
        None,
        1_800_000_040_200,
    );

    accepted.push(commit_balance(
        &mut live,
        &issue,
        QuickChainSupplyDecision::IssueApproved,
        "tx:roc:phase2:ordering-issue",
    ));
    accepted.push(commit_balance(
        &mut live,
        &transfer,
        QuickChainSupplyDecision::NoSupplyChange,
        "tx:roc:phase2:ordering-transfer",
    ));
    accepted.push(commit_balance(
        &mut live,
        &burn,
        QuickChainSupplyDecision::BurnApproved,
        "tx:roc:phase2:ordering-burn",
    ));

    assert_replay_matches(&live, &accepted);

    let mut by_operation_id = accepted.clone();
    by_operation_id.sort_by(|left, right| {
        left.record()
            .intent()
            .operation_id
            .cmp(&right.record().intent().operation_id)
    });

    assert_ne!(
        by_operation_id
            .iter()
            .map(|operation| operation.record().intent().operation_id.as_str())
            .collect::<Vec<_>>(),
        accepted
            .iter()
            .map(|operation| operation.record().intent().operation_id.as_str())
            .collect::<Vec<_>>()
    );

    let operation_id_replay_error =
        QuickChainAtomicState::rebuild_from_accepted_operations(&by_operation_id)
            .expect_err("operation-id/DB-style ordering must not define replay truth");

    assert!(
        matches!(
            operation_id_replay_error,
            QuickChainExecutionError::Transition(_) | QuickChainExecutionError::Replay(_)
        ),
        "unexpected operation-id replay error: {operation_id_replay_error:?}"
    );

    let mut by_wall_clock = accepted.clone();
    by_wall_clock.sort_by_key(|operation| operation.record().intent().produced_at_ms);

    assert_ne!(
        by_wall_clock
            .iter()
            .map(|operation| operation.record().intent().produced_at_ms)
            .collect::<Vec<_>>(),
        accepted
            .iter()
            .map(|operation| operation.record().intent().produced_at_ms)
            .collect::<Vec<_>>()
    );

    let wall_clock_replay_error =
        QuickChainAtomicState::rebuild_from_accepted_operations(&by_wall_clock)
            .expect_err("wall-clock/event-time ordering must not define replay truth");

    assert!(
        matches!(
            wall_clock_replay_error,
            QuickChainExecutionError::Transition(_) | QuickChainExecutionError::Replay(_)
        ),
        "unexpected wall-clock replay error: {wall_clock_replay_error:?}"
    );
}
