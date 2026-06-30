#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Internal ROC Beta paid-content hold/capture replay proof for post/comment/article/content_view labels.
//! RO:WHY — Phase 1 proves paid content can be represented by existing wallet/ledger operation history.
//! RO:INTERACTS — ron_ledger::quickchain atomic state and ron_proto QuickChain operation intents.
//! RO:INVARIANTS — no new mutation authority; accepted retries are no-ops; paid value is conserved.
//! RO:METRICS — none.
//! RO:CONFIG — requires ron-ledger quickchain-preflight feature.
//! RO:SECURITY — receipt txids are backend evidence labels, not capabilities or client authority.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_paid_content_replay_label.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionDisposition, QuickChainHoldEpochInput,
    QuickChainHoldTerminalStatus, QuickChainHoldTransitionKind, QuickChainSupplyDecision,
};
use ron_proto::quickchain::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";
const VIEWER: &str = "account:viewer";
const CREATOR: &str = "account:creator";

#[derive(Debug, Clone, Copy)]
struct PaidContentCase {
    action: &'static str,
    open_operation_index: u8,
    capture_operation_index: u8,
    hold_index: u8,
    amount_minor: &'static str,
}

fn operation_id(index: u8) -> String {
    format!("op_{index:032x}")
}

fn hold_id(index: u8) -> String {
    format!("hold_{index:032x}")
}

#[allow(clippy::too_many_arguments)]
fn intent(
    operation_index: u8,
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
        operation_id: operation_id(operation_index),
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

fn seed_viewer_balance(state: &mut QuickChainAtomicState) {
    let issue = intent(
        1,
        "idem:internal-roc-beta:seed-viewer",
        QuickChainOperationClassV1::Issue,
        VIEWER,
        None,
        "1000",
        None,
        1_820_000_000_000,
    );

    let outcome = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:internal_roc_beta:seed_viewer",
        )
        .expect("approved seed issue should commit");

    assert_eq!(
        outcome.disposition(),
        QuickChainExecutionDisposition::Committed
    );
    assert_eq!(state.balance_minor(VIEWER), 1000);
    assert_eq!(state.current_supply_minor(), 1000);
}

#[test]
fn paid_content_hold_capture_history_is_replayable_and_conserved() {
    let cases = [
        PaidContentCase {
            action: "paid_post",
            open_operation_index: 10,
            capture_operation_index: 11,
            hold_index: 10,
            amount_minor: "10",
        },
        PaidContentCase {
            action: "paid_comment",
            open_operation_index: 20,
            capture_operation_index: 21,
            hold_index: 20,
            amount_minor: "15",
        },
        PaidContentCase {
            action: "paid_article",
            open_operation_index: 30,
            capture_operation_index: 31,
            hold_index: 30,
            amount_minor: "20",
        },
        PaidContentCase {
            action: "content_view",
            open_operation_index: 40,
            capture_operation_index: 41,
            hold_index: 40,
            amount_minor: "25",
        },
    ];

    let mut state = QuickChainAtomicState::new();
    seed_viewer_balance(&mut state);

    let mut total_paid_minor = 0_u128;

    for case in cases {
        let hold = hold_id(case.hold_index);

        let open = intent(
            case.open_operation_index,
            &format!("idem:{}:hold_open", case.action),
            QuickChainOperationClassV1::HoldOpen,
            VIEWER,
            Some(CREATOR),
            case.amount_minor,
            Some(&hold),
            1_820_000_001_000 + u64::from(case.open_operation_index),
        );

        let capture = intent(
            case.capture_operation_index,
            &format!("idem:{}:hold_capture", case.action),
            QuickChainOperationClassV1::HoldCapture,
            VIEWER,
            Some(CREATOR),
            case.amount_minor,
            Some(&hold),
            1_820_000_002_000 + u64::from(case.capture_operation_index),
        );

        let open_outcome = state
            .execute_hold_operation(
                &open,
                QuickChainHoldEpochInput::Open {
                    created_at_epoch: 1,
                    expires_at_epoch: 10,
                },
                format!("tx:roc:{}:hold_open", case.action),
            )
            .expect("paid content hold open should commit");

        assert_eq!(
            open_outcome.disposition(),
            QuickChainExecutionDisposition::Committed
        );

        let open_transition = open_outcome
            .transition()
            .expect("fresh hold open should include transition");
        assert_eq!(open_transition.kind, QuickChainHoldTransitionKind::Opened);
        assert_eq!(
            open_transition.counterparty_account_id.as_deref(),
            Some(CREATOR)
        );
        assert_eq!(
            state
                .active_hold(&hold)
                .expect("hold should be active")
                .hold_id(),
            hold
        );

        let before_capture = state.clone();

        let capture_outcome = state
            .execute_hold_operation(
                &capture,
                QuickChainHoldEpochInput::Terminal { current_epoch: 2 },
                format!("tx:roc:{}:hold_capture", case.action),
            )
            .expect("paid content hold capture should commit");

        assert_ne!(state, before_capture);
        assert_eq!(
            capture_outcome.disposition(),
            QuickChainExecutionDisposition::Committed
        );

        let capture_transition = capture_outcome
            .transition()
            .expect("fresh hold capture should include transition");
        assert_eq!(
            capture_transition.kind,
            QuickChainHoldTransitionKind::Captured
        );
        assert_eq!(
            capture_transition.terminal_status,
            Some(QuickChainHoldTerminalStatus::Captured)
        );
        assert_eq!(
            capture_transition.counterparty_account_id.as_deref(),
            Some(CREATOR)
        );

        assert!(
            state.active_hold(&hold).is_none(),
            "captured paid-content hold must leave active hold state"
        );

        let terminal = state
            .terminal_hold(&hold)
            .expect("captured paid-content hold should leave terminal evidence");
        assert_eq!(terminal.status(), QuickChainHoldTerminalStatus::Captured);
        assert_eq!(terminal.terminal_counterparty_account_id(), Some(CREATOR));
        assert_eq!(
            terminal.terminal_amount_minor(),
            case.amount_minor.parse::<u128>().expect("test amount")
        );

        let snapshot_after_capture = state.clone();

        let retry = state
            .execute_hold_operation(
                &capture,
                QuickChainHoldEpochInput::Terminal { current_epoch: 999 },
                format!("tx:roc:{}:ignored_retry_receipt", case.action),
            )
            .expect("exact capture retry should return original backend evidence");

        assert!(retry.is_retry());
        assert_eq!(retry.record(), capture_outcome.record());
        assert_eq!(
            state, snapshot_after_capture,
            "exact retry must not mutate paid-content ledger state"
        );

        total_paid_minor += case.amount_minor.parse::<u128>().expect("test amount");

        assert_eq!(state.balance_minor(CREATOR), total_paid_minor);
        assert_eq!(state.balance_minor(VIEWER), 1000 - total_paid_minor);
        assert_eq!(state.held_minor(VIEWER), 0);
        assert_eq!(
            state.available_minor(VIEWER).expect("available balance"),
            1000 - total_paid_minor
        );
        assert_eq!(
            state.current_supply_minor(),
            1000,
            "paid content transfers/captures must conserve internal ROC supply"
        );
    }

    assert_eq!(total_paid_minor, 70);
    assert_eq!(state.balance_minor(VIEWER), 930);
    assert_eq!(state.balance_minor(CREATOR), 70);
    assert_eq!(state.current_supply_minor(), 1000);
    assert_eq!(state.operation_count(), 1 + cases.len() * 2);
}
