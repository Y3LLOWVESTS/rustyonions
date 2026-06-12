#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Integration tests for atomic QuickChain hold reservation, capture, release, expiry, retries, and compaction.
//! RO:WHY — ECON/RES: concurrent holds and reserved-balance enforcement must remain deterministic and fully atomic.
//! RO:INTERACTS — ron_ledger::quickchain and ron_proto QuickChain operation intents.
//! RO:INVARIANTS — retries do not mutate; terminal IDs cannot reopen; expired holds cannot capture; held ROC is not ordinarily spendable.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — receipt references and epoch inputs are bounded test values, not real capabilities.
//! RO:TEST — this file.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionDisposition, QuickChainExecutionError,
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

fn commit_issue(
    state: &mut QuickChainAtomicState,
    operation_hex_digit: char,
    account_id: &str,
    amount_minor: &str,
) {
    state
        .execute_balance_operation(
            &intent(
                operation_hex_digit,
                &format!("idem:issue:{operation_hex_digit}"),
                QuickChainOperationClassV1::Issue,
                account_id,
                None,
                amount_minor,
                None,
                1_000,
            ),
            QuickChainSupplyDecision::IssueApproved,
            format!("tx:roc:issue:{operation_hex_digit}"),
        )
        .expect("test issue should commit");
}

#[test]
fn concurrent_holds_capture_release_and_retries_match_blueprint() {
    let h1 = hold_id('a');
    let h2 = hold_id('b');

    let open_h1 = intent(
        '1',
        "idem:hold:h1:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:bob"),
        "100",
        Some(&h1),
        1_001,
    );
    let open_h2 = intent(
        '2',
        "idem:hold:h2:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        Some("account:bob"),
        "250",
        Some(&h2),
        1_002,
    );
    let capture_h2 = intent(
        '3',
        "idem:hold:h2:capture",
        QuickChainOperationClassV1::HoldCapture,
        "account:alice",
        Some("account:bob"),
        "250",
        Some(&h2),
        1_003,
    );
    let release_h1 = intent(
        '4',
        "idem:hold:h1:release",
        QuickChainOperationClassV1::HoldRelease,
        "account:alice",
        None,
        "100",
        Some(&h1),
        1_004,
    );

    let mut state = QuickChainAtomicState::new();
    commit_issue(&mut state, '0', "account:alice", "1000");

    state
        .execute_hold_operation(
            &open_h1,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 2,
            },
            "tx:roc:h1:open",
        )
        .expect("H1 should open");

    state
        .execute_hold_operation(
            &open_h2,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 2,
            },
            "tx:roc:h2:open",
        )
        .expect("H2 should open");

    assert_eq!(state.balance_minor("account:alice"), 1000);
    assert_eq!(state.held_minor("account:alice"), 350);
    assert_eq!(state.available_minor("account:alice").unwrap(), 650);

    let ordered: Vec<_> = state
        .hold_state()
        .ordered_active_holds()
        .map(|(id, _)| id.to_string())
        .collect();
    assert_eq!(ordered, vec![h1.clone(), h2.clone()]);

    let capture = state
        .execute_hold_operation(
            &capture_h2,
            QuickChainHoldEpochInput::Terminal { current_epoch: 1 },
            "tx:roc:h2:capture",
        )
        .expect("H2 capture should commit");

    assert_eq!(
        capture.disposition(),
        QuickChainExecutionDisposition::Committed
    );
    assert_eq!(state.balance_minor("account:alice"), 750);
    assert_eq!(state.balance_minor("account:bob"), 250);
    assert_eq!(state.held_minor("account:alice"), 100);
    assert_eq!(state.available_minor("account:alice").unwrap(), 650);
    assert!(state.active_hold(&h2).is_none());

    let capture_snapshot = state.clone();
    let capture_retry = state
        .execute_hold_operation(
            &capture_h2,
            QuickChainHoldEpochInput::Terminal { current_epoch: 999 },
            "invalid receipt with spaces",
        )
        .expect("exact capture retry should return original evidence");

    assert!(capture_retry.is_retry());
    assert_eq!(capture_retry.record(), capture.record());
    assert_eq!(state, capture_snapshot);

    let release = state
        .execute_hold_operation(
            &release_h1,
            QuickChainHoldEpochInput::Terminal { current_epoch: 1 },
            "tx:roc:h1:release",
        )
        .expect("H1 release should commit");

    assert_eq!(state.balance_minor("account:alice"), 750);
    assert_eq!(state.held_minor("account:alice"), 0);
    assert_eq!(state.available_minor("account:alice").unwrap(), 750);
    assert_eq!(state.hold_state().active_hold_count(), 0);
    assert_eq!(state.hold_state().terminal_hold_count(), 2);

    let release_snapshot = state.clone();
    let release_retry = state
        .execute_hold_operation(
            &release_h1,
            QuickChainHoldEpochInput::Terminal { current_epoch: 999 },
            "another invalid receipt",
        )
        .expect("exact release retry should return original evidence");

    assert!(release_retry.is_retry());
    assert_eq!(release_retry.record(), release.record());
    assert_eq!(state, release_snapshot);

    assert_eq!(state.operation_count(), 5);
    assert_eq!(state.last_account_sequence("account:alice"), 5);
    assert_eq!(state.last_account_sequence("account:bob"), 1);
    assert_eq!(state.next_ledger_sequence(), 7);
}

#[test]
fn reserved_value_blocks_transfer_and_burn() {
    let hold = hold_id('1');

    let mut state = QuickChainAtomicState::new();
    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .execute_hold_operation(
            &intent(
                '2',
                "idem:reserve:open",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                None,
                "80",
                Some(&hold),
                2_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "tx:roc:reserve:open",
        )
        .expect("hold should open");

    let snapshot = state.clone();

    let transfer_error = state
        .execute_balance_operation(
            &intent(
                '3',
                "idem:reserve:transfer",
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:bob"),
                "21",
                None,
                2_001,
            ),
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:reserve:transfer",
        )
        .expect_err("transfer may not consume held value");

    assert_eq!(
        transfer_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::InsufficientAvailableFunds {
            account_id: "account:alice".to_string(),
            available_minor: 20,
            required_minor: 21,
        })
    );
    assert_eq!(state, snapshot);

    let burn_error = state
        .execute_balance_operation(
            &intent(
                '4',
                "idem:reserve:burn",
                QuickChainOperationClassV1::Burn,
                "account:alice",
                None,
                "21",
                None,
                2_002,
            ),
            QuickChainSupplyDecision::BurnApproved,
            "tx:roc:reserve:burn",
        )
        .expect_err("burn may not consume held value");

    assert_eq!(
        burn_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::InsufficientAvailableFunds {
            account_id: "account:alice".to_string(),
            available_minor: 20,
            required_minor: 21,
        })
    );
    assert_eq!(state, snapshot);

    state
        .execute_balance_operation(
            &intent(
                '5',
                "idem:reserve:allowed",
                QuickChainOperationClassV1::Transfer,
                "account:alice",
                Some("account:bob"),
                "20",
                None,
                2_003,
            ),
            QuickChainSupplyDecision::NoSupplyChange,
            "tx:roc:reserve:allowed",
        )
        .expect("exactly available value should transfer");

    assert_eq!(state.balance_minor("account:alice"), 80);
    assert_eq!(state.held_minor("account:alice"), 80);
    assert_eq!(state.available_minor("account:alice").unwrap(), 0);
}

#[test]
fn partial_capture_closes_hold_and_releases_remainder() {
    let hold = hold_id('2');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .execute_hold_operation(
            &intent(
                '2',
                "idem:partial:open",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                Some("account:bob"),
                "70",
                Some(&hold),
                3_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "tx:roc:partial:open",
        )
        .expect("hold should open");

    let outcome = state
        .execute_hold_operation(
            &intent(
                '3',
                "idem:partial:capture",
                QuickChainOperationClassV1::HoldCapture,
                "account:alice",
                Some("account:bob"),
                "40",
                Some(&hold),
                3_001,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
            "tx:roc:partial:capture",
        )
        .expect("partial capture should close the lifecycle");

    let transition = outcome
        .transition()
        .expect("fresh capture has transition evidence");

    assert_eq!(transition.uncaptured_remainder_minor, 30);
    assert_eq!(state.balance_minor("account:alice"), 60);
    assert_eq!(state.balance_minor("account:bob"), 40);
    assert_eq!(state.held_minor("account:alice"), 0);
    assert_eq!(state.available_minor("account:alice").unwrap(), 60);
    assert!(state.active_hold(&hold).is_none());

    let terminal = state
        .terminal_hold(&hold)
        .expect("terminal evidence must remain");

    assert_eq!(terminal.status(), QuickChainHoldTerminalStatus::Captured);
    assert_eq!(terminal.original_amount_minor(), 70);
    assert_eq!(terminal.terminal_amount_minor(), 40);
    assert_eq!(terminal.uncaptured_remainder_minor(), 30);
}

#[test]
fn expiry_is_epoch_gated_and_blocks_late_capture_or_release() {
    let hold = hold_id('3');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .execute_hold_operation(
            &intent(
                '2',
                "idem:expiry:open",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                Some("account:bob"),
                "50",
                Some(&hold),
                4_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 2,
            },
            "tx:roc:expiry:open",
        )
        .expect("hold should open");

    let snapshot = state.clone();

    for submitted in [
        intent(
            '3',
            "idem:expiry:late-capture",
            QuickChainOperationClassV1::HoldCapture,
            "account:alice",
            Some("account:bob"),
            "50",
            Some(&hold),
            4_001,
        ),
        intent(
            '4',
            "idem:expiry:late-release",
            QuickChainOperationClassV1::HoldRelease,
            "account:alice",
            None,
            "50",
            Some(&hold),
            4_002,
        ),
    ] {
        let error = state
            .execute_hold_operation(
                &submitted,
                QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
                "tx:roc:expiry:late",
            )
            .expect_err("capture/release at expiry must reject");

        assert!(matches!(
            error,
            QuickChainExecutionError::Hold(QuickChainHoldError::HoldPastExpiry {
                current_epoch: 2,
                expires_at_epoch: 2,
                ..
            })
        ));
        assert_eq!(state, snapshot);
    }

    let early_expiry = intent(
        '5',
        "idem:expiry:early",
        QuickChainOperationClassV1::HoldExpire,
        "account:alice",
        None,
        "50",
        Some(&hold),
        4_003,
    );

    let error = state
        .execute_hold_operation(
            &early_expiry,
            QuickChainHoldEpochInput::Terminal { current_epoch: 1 },
            "tx:roc:expiry:early",
        )
        .expect_err("expiry before eligibility must reject");

    assert!(matches!(
        error,
        QuickChainExecutionError::Hold(QuickChainHoldError::ExpiryNotEligible {
            current_epoch: 1,
            expires_at_epoch: 2,
            ..
        })
    ));
    assert_eq!(state, snapshot);

    state
        .execute_hold_operation(
            &intent(
                '6',
                "idem:expiry:commit",
                QuickChainOperationClassV1::HoldExpire,
                "account:alice",
                None,
                "50",
                Some(&hold),
                4_004,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
            "tx:roc:expiry:commit",
        )
        .expect("eligible expiry should commit");

    assert_eq!(state.held_minor("account:alice"), 0);
    assert_eq!(state.balance_minor("account:alice"), 100);
    assert_eq!(
        state
            .terminal_hold(&hold)
            .expect("terminal expiry evidence")
            .status(),
        QuickChainHoldTerminalStatus::Expired
    );
}

#[test]
fn terminal_hold_cannot_transition_or_reopen() {
    let hold = hold_id('4');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .execute_hold_operation(
            &intent(
                '2',
                "idem:terminal:open",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                None,
                "30",
                Some(&hold),
                5_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "tx:roc:terminal:open",
        )
        .expect("hold should open");

    state
        .execute_hold_operation(
            &intent(
                '3',
                "idem:terminal:release",
                QuickChainOperationClassV1::HoldRelease,
                "account:alice",
                None,
                "30",
                Some(&hold),
                5_001,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
            "tx:roc:terminal:release",
        )
        .expect("hold should release");

    let snapshot = state.clone();

    let capture_error = state
        .execute_hold_operation(
            &intent(
                '4',
                "idem:terminal:capture",
                QuickChainOperationClassV1::HoldCapture,
                "account:alice",
                Some("account:bob"),
                "30",
                Some(&hold),
                5_002,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch: 3 },
            "tx:roc:terminal:capture",
        )
        .expect_err("terminal hold cannot capture");

    assert_eq!(
        capture_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldAlreadyTerminal {
            hold_id: hold.clone(),
        })
    );
    assert_eq!(state, snapshot);

    let reopen_error = state
        .execute_hold_operation(
            &intent(
                '5',
                "idem:terminal:reopen",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                None,
                "30",
                Some(&hold),
                5_003,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 3,
                expires_at_epoch: 12,
            },
            "tx:roc:terminal:reopen",
        )
        .expect_err("terminal hold ID cannot be resurrected");

    assert_eq!(
        reopen_error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldIdAlreadyUsed { hold_id: hold })
    );
    assert_eq!(state, snapshot);
}

#[test]
fn capture_counterparty_mismatch_rolls_back() {
    let hold = hold_id('5');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .execute_hold_operation(
            &intent(
                '2',
                "idem:mismatch:open",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                Some("account:bob"),
                "40",
                Some(&hold),
                6_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "tx:roc:mismatch:open",
        )
        .expect("hold should open");

    let snapshot = state.clone();

    let error = state
        .execute_hold_operation(
            &intent(
                '3',
                "idem:mismatch:capture",
                QuickChainOperationClassV1::HoldCapture,
                "account:alice",
                Some("account:carol"),
                "40",
                Some(&hold),
                6_001,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
            "tx:roc:mismatch:capture",
        )
        .expect_err("capture beneficiary mismatch must reject");

    assert_eq!(
        error,
        QuickChainExecutionError::Hold(QuickChainHoldError::HoldCounterpartyMismatch {
            hold_id: hold,
            expected_counterparty_account_id: "account:bob".to_string(),
            actual_counterparty_account_id: "account:carol".to_string(),
        })
    );
    assert_eq!(state, snapshot);
}

#[test]
fn release_amount_mismatch_rolls_back() {
    let hold = hold_id('6');
    let mut state = QuickChainAtomicState::new();

    commit_issue(&mut state, '1', "account:alice", "100");

    state
        .execute_hold_operation(
            &intent(
                '2',
                "idem:release-amount:open",
                QuickChainOperationClassV1::HoldOpen,
                "account:alice",
                None,
                "40",
                Some(&hold),
                7_000,
            ),
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "tx:roc:release-amount:open",
        )
        .expect("hold should open");

    let snapshot = state.clone();

    let error = state
        .execute_hold_operation(
            &intent(
                '3',
                "idem:release-amount:release",
                QuickChainOperationClassV1::HoldRelease,
                "account:alice",
                None,
                "20",
                Some(&hold),
                7_001,
            ),
            QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
            "tx:roc:release-amount:release",
        )
        .expect_err("release must close the full reservation");

    assert_eq!(
        error,
        QuickChainExecutionError::Hold(QuickChainHoldError::TerminalAmountMismatch {
            hold_id: hold,
            expected_minor: 40,
            actual_minor: 20,
        })
    );
    assert_eq!(state, snapshot);
}

#[test]
fn invalid_receipt_and_epoch_input_leave_all_state_unchanged() {
    let hold = hold_id('7');
    let open = intent(
        '1',
        "idem:invalid-boundary:open",
        QuickChainOperationClassV1::HoldOpen,
        "account:alice",
        None,
        "10",
        Some(&hold),
        8_000,
    );

    let mut state = QuickChainAtomicState::new();
    commit_issue(&mut state, '0', "account:alice", "100");
    let snapshot = state.clone();

    state
        .execute_hold_operation(
            &open,
            QuickChainHoldEpochInput::Open {
                created_at_epoch: 1,
                expires_at_epoch: 10,
            },
            "invalid receipt with spaces",
        )
        .expect_err("invalid receipt must reject before commit");

    assert_eq!(state, snapshot);

    let error = state
        .execute_hold_operation(
            &open,
            QuickChainHoldEpochInput::Terminal { current_epoch: 1 },
            "tx:roc:invalid-boundary:open",
        )
        .expect_err("open operation requires open epoch input");

    assert_eq!(
        error,
        QuickChainExecutionError::Hold(QuickChainHoldError::EpochInputMismatch)
    );
    assert_eq!(state, snapshot);
}
