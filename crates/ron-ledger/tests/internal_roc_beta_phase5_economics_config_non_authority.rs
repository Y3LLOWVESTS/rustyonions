#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Internal ROC Beta Phase 5 ledger proof that economics config is non-authority.
//! RO:WHY — ECON/GOV: tokenomics config must not create receipt, balance, supply, or finality truth.
//! RO:INTERACTS — ron_ledger::quickchain atomic state and configs/roc-economics.toml.
//! RO:INVARIANTS — svc-wallet remains mutation front-door; config refs are not receipt txids; rejected config refs mutate nothing.
//! RO:METRICS — none.
//! RO:CONFIG — reads configs/roc-economics.toml as a static presence/safety marker only.
//! RO:SECURITY — no bridge, staking, liquidity, external settlement, or config-derived ledger authority.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_beta_phase5_economics_config_non_authority.

use ron_ledger::quickchain::{
    QuickChainAtomicState, QuickChainExecutionError, QuickChainReplayError,
    QuickChainSupplyDecision,
};
use ron_proto::{
    QuickChainOperationClassV1, QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const ROC_ECONOMICS_CONFIG: &str = include_str!("../../../configs/roc-economics.toml");
const CHAIN_ID: &str = "ron-devnet";
const CREATOR: &str = "account:creator-phase5";
const VIEWER: &str = "account:viewer-phase5";

fn operation_id(index: u8) -> String {
    format!("op_{index:032x}")
}

fn intent(
    operation_index: u8,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
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
        hold_id: None,
        account_sequence: None,
        produced_at_ms,
    }
}

fn expect_non_receipt_evidence(error: QuickChainExecutionError) {
    assert!(
        matches!(
            error,
            QuickChainExecutionError::Replay(QuickChainReplayError::InvalidReceiptEvidenceKind)
        ),
        "unexpected error: {error}"
    );
}

#[test]
fn canonical_roc_economics_config_file_is_present_and_inert_by_default() {
    for required in [
        "schema = \"internal_roc.economics-config.v1\"",
        "version = 1",
        "[units]",
        "basis_point_denominator = 10000",
        "[paid_content]",
        "[reward_pools]",
        "[anti_farming]",
        "[rounding]",
        "remainder_sink = \"treasury\"",
        "[future_bridge]",
        "[future_staking]",
    ] {
        assert!(
            ROC_ECONOMICS_CONFIG.contains(required),
            "configs/roc-economics.toml missing required fragment: {required}"
        );
    }

    for section_name in ["future_bridge", "future_staking"] {
        let marker = format!("[{section_name}]");
        let section_tail = ROC_ECONOMICS_CONFIG
            .split_once(&marker)
            .map(|(_, tail)| tail)
            .unwrap_or_else(|| panic!("configs/roc-economics.toml missing section {marker}"));

        let section = section_tail.split("\n[").next().unwrap_or(section_tail);

        assert!(
            section.lines().any(|line| line.trim() == "enabled = false"),
            "{section_name} must remain disabled by default"
        );

        assert!(
            !section.lines().any(|line| line.trim() == "enabled = true"),
            "{section_name} must not be enabled"
        );
    }

    assert!(
        !ROC_ECONOMICS_CONFIG.contains("solana"),
        "Phase 5 Round 1 config must not activate Solana runtime"
    );

    let mut current_section = "";

    for line in ROC_ECONOMICS_CONFIG.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed;
            continue;
        }

        assert!(
            !(current_section.contains("liquidity") && trimmed == "enabled = true"),
            "liquidity-related economics sections must not be enabled"
        );
    }
}

#[test]
fn economics_config_reference_cannot_create_issue_receipt_or_supply() {
    let mut state = QuickChainAtomicState::new();
    let before = state.clone();

    let issue = intent(
        1,
        "idem:phase5:config-direct-issue",
        QuickChainOperationClassV1::Issue,
        CREATOR,
        None,
        "100",
        1_920_000_000_000,
    );

    let error = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "roc_economics_config:internal_roc_beta_v1",
        )
        .expect_err("economics config reference must not be accepted as receipt evidence");

    expect_non_receipt_evidence(error);
    assert_eq!(state, before);
    assert_eq!(state.balance_minor(CREATOR), 0);
    assert_eq!(state.current_supply_minor(), 0);
    assert_eq!(state.operation_count(), 0);
}

#[test]
fn economics_config_reference_cannot_create_transfer_receipt_or_balance_truth() {
    let mut state = QuickChainAtomicState::new();

    let seed = intent(
        1,
        "idem:phase5:seed-viewer",
        QuickChainOperationClassV1::Issue,
        VIEWER,
        None,
        "100",
        1_920_000_000_100,
    );

    state
        .execute_balance_operation(
            &seed,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase5:seed_viewer",
        )
        .expect("wallet-like receipt should seed balance");

    let before = state.clone();

    let transfer = intent(
        2,
        "idem:phase5:config-direct-transfer",
        QuickChainOperationClassV1::Transfer,
        VIEWER,
        Some(CREATOR),
        "10",
        1_920_000_000_200,
    );

    let error = state
        .execute_balance_operation(
            &transfer,
            QuickChainSupplyDecision::NoSupplyChange,
            "internal_roc_economics:paid_content_default_splits",
        )
        .expect_err("economics config reference must not be accepted as receipt evidence");

    expect_non_receipt_evidence(error);
    assert_eq!(state, before);
    assert_eq!(state.balance_minor(VIEWER), 100);
    assert_eq!(state.balance_minor(CREATOR), 0);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 1);
}

#[test]
fn wallet_like_receipt_remains_the_accepted_mutation_source() {
    let mut state = QuickChainAtomicState::new();

    let issue = intent(
        1,
        "idem:phase5:wallet-issue",
        QuickChainOperationClassV1::Issue,
        VIEWER,
        None,
        "100",
        1_920_000_000_300,
    );

    let outcome = state
        .execute_balance_operation(
            &issue,
            QuickChainSupplyDecision::IssueApproved,
            "tx:roc:phase5:wallet_issue",
        )
        .expect("wallet-like receipt should commit");

    assert!(outcome.is_committed());
    assert_eq!(
        outcome.record().receipt_txid(),
        "tx:roc:phase5:wallet_issue"
    );
    assert_eq!(state.balance_minor(VIEWER), 100);
    assert_eq!(state.current_supply_minor(), 100);
    assert_eq!(state.operation_count(), 1);
}
