//! RO:WHAT — Internal ROC Stabilization boundary tests for svc-wallet as mutation front-door.
//! RO:WHY — Product beta readiness requires wallet receipts, idempotency, nonce discipline, and paid/reward mutations to stay wallet/ledger-derived.
//! RO:INTERACTS — dto/responses.rs, ledger/client.rs, routes/v1, idem/store.rs, Internal ROC beta wallet regressions.
//! RO:INVARIANTS — svc-wallet is mutation front-door; ron-ledger is truth; idempotency_key is retry metadata; receipts are accepted-only backend evidence.
//! RO:SECURITY — no fake receipt/balance/finality, silent spend, cache-only unlock, bypass mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_stabilization_mutation_frontdoor_boundary.

#![allow(clippy::missing_panics_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WalletFrontDoorTruthContract {
    schema: String,
    source_crate: String,
    role: String,
    mutation_front_door: String,
    ledger_truth: String,
    receipt_source: String,
    idempotency_rule: String,
    accounting_role: String,
}

#[test]
fn wallet_sources_keep_front_door_receipts_idempotency_and_ledger_adapter_boundaries() {
    let responses = read_rel("src/dto/responses.rs");
    let ledger = read_rel("src/ledger/client.rs");
    let routes = read_rel("src/routes/v1/mod.rs");
    let issue_route = read_rel("src/routes/v1/issue.rs");
    let idem = read_rel("src/idem/store.rs");

    assert_all(
        "wallet receipt DTO source",
        &responses,
        &[
            "pub enum WalletOp",
            "Issue",
            "Transfer",
            "Burn",
            "Hold",
            "Capture",
            "Release",
            "pub enum ReceiptSettlementStatus",
            "Accepted",
            "future QuickChain settlement states and must not be invented by svc-wallet",
            "pub struct Receipt",
            "receipt_hash",
            "ledger_seq_start",
            "ledger_seq_end",
            "ledger_root",
        ],
    );

    assert_all(
        "wallet ledger adapter source",
        &ledger,
        &[
            "Local ron-ledger adapter for issue, transfer, burn, hold, capture, release, and balance reads",
            "Ensures svc-wallet never becomes its own durable truth store",
            "transfers are balanced",
            "escrow moves through ledger",
        ],
    );

    for (label, needles) in [
        ("ledger issue path", ["fn issue", ".issue("]),
        ("ledger transfer path", ["fn transfer", ".transfer("]),
        ("ledger burn path", ["fn burn", ".burn("]),
        ("ledger hold path", ["fn hold", ".hold("]),
        ("ledger capture path", ["fn capture", ".capture("]),
        ("ledger release path", ["fn release", ".release("]),
        ("ledger balance path", ["fn balance", ".balance("]),
    ] {
        assert_any(label, &ledger, &needles);
    }

    assert_all(
        "wallet v1 router source",
        &routes,
        &[
            "/balance",
            "/issue",
            "/transfer",
            "/burn",
            "/hold",
            "/capture",
            "/release",
            "/tx/:txid",
        ],
    );

    assert_all(
        "wallet issue route source",
        &issue_route,
        &[
            "resolve_idempotency_key",
            "request_fingerprint(WalletOp::Issue",
            "state.idem.insert",
            "state.remember_receipt",
            "AccountingEvent",
            "state.metrics.inc_op(WalletOp::Issue)",
        ],
    );

    assert_any(
        "wallet issue route commits through ledger before receipt memory",
        &issue_route,
        &[".ledger\n        .issue", "state.ledger.issue"],
    );

    assert_all(
        "wallet idempotency store source",
        &idem,
        &[
            "StoredDecision",
            "fingerprint",
            "receipt",
            "Insert a successful receipt",
            "lookup",
            "replay_and_conflict_paths",
        ],
    );

    assert_none(
        "svc-wallet front-door production source forbidden runtime markers",
        &format!("{responses}\n{ledger}\n{routes}\n{issue_route}\n{idem}").to_lowercase(),
        &[
            "fake_receipt",
            "fake_balance",
            "fake_finality",
            "cache_only_unlock",
            "unlock_from_cache",
            "gateway_mutate",
            "omnigate_mutate",
            "accounting_mutate",
            "rewarder_mutate",
            "policy_mutate",
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement_runtime",
            "mint_rox",
            "burn_rox",
        ],
    );
}

#[test]
fn existing_wallet_regressions_are_wired_into_stabilization_surface() {
    let paid_content = read_rel("tests/internal_roc_beta_paid_content_receipt_path.rs");
    let idempotency = read_rel("tests/internal_roc_beta_phase2_paid_action_idempotency.rs");
    let receipt_lookup = read_rel("tests/internal_roc_beta_phase2_receipt_lookup_after_replay.rs");
    let payout = read_rel("tests/internal_roc_beta_phase3_approved_payout_execution_boundary.rs");
    let observer = read_rel("tests/internal_roc_beta_phase3_accounting_observer_boundary.rs");
    let config = read_rel("tests/internal_roc_beta_phase5_config_non_authority.rs");

    assert_all(
        "paid-content receipt path regression",
        &paid_content,
        &[
            "paid_content_hold_capture_receipts_cover_beta_action_labels",
            "unfunded_paid_content_hold_rejects_without_creator_credit_or_fake_receipt",
            "paid_content_wallet_request_dtos_reject_authority_poison_fields",
            "WalletOp::Hold",
            "WalletOp::Capture",
            "ReceiptSettlementStatus::Accepted",
        ],
    );

    assert_all(
        "paid action idempotency regression",
        &idempotency,
        &[
            "accepted_paid_action_retry_replays_receipts_without_second_mutation",
            "same_idempotency_key_with_different_paid_action_body_conflicts",
            "cancelled_paid_action_before_mutation_leaves_no_receipt_or_balance_change",
        ],
    );

    assert_all(
        "receipt lookup after replay regression",
        &receipt_lookup,
        &[
            "backend_accepted_receipts_rehydrate_lookup_after_replay_without_fabrication",
            "unknown receipt",
        ],
    );

    assert_all(
        "approved payout wallet execution regression",
        &payout,
        &[
            "approved_payout_executes_only_as_wallet_issue_receipt",
            "duplicate_approved_payout_idempotency_does_not_double_issue",
            "approved_payouts_to_distinct_accounts_remain_backend_balance_truth",
            "reward_plan_policy_and_accounting_fields_cannot_smuggle_payout_issue_request_authority",
            "accounting_observer_after_payout_receipt_cannot_mutate_wallet_balance",
        ],
    );

    assert_all(
        "accounting observer non-mutation regression",
        &observer,
        &[
            "accounting_observer_events_do_not_mutate_wallet_or_ledger_balances",
            "wallet_receipt_wire_shape_stays_backend_receipt_not_reward_plan",
            "accepted_wallet_receipt_remains_wallet_ledger_derived_observation_source",
        ],
    );

    assert_all(
        "economics config non-authority regression",
        &config,
        &["config", "non_authority"],
    );
}

#[test]
fn wallet_front_door_contract_rejects_authority_poison_fields() {
    for field in [
        "gateway_mutation_authority",
        "omnigate_mutation_authority",
        "accounting_mutation_authority",
        "rewarder_mutation_authority",
        "policy_receipt_authority",
        "client_receipt_truth",
        "client_balance_truth",
        "cache_unlock_authority",
        "fake_receipt",
        "fake_balance",
        "fake_finality",
        "silent_spend",
        "bridge_authority",
        "external_settlement_authority",
        "rox_solana_authority",
    ] {
        let mut value = json!({
            "schema": "svc-wallet.internal-roc-stabilization-mutation-frontdoor.v1",
            "source_crate": "svc-wallet",
            "role": "mutation_front_door",
            "mutation_front_door": "svc-wallet_only",
            "ledger_truth": "ron-ledger_only",
            "receipt_source": "backend_wallet_ledger_accepted_receipt",
            "idempotency_rule": "same_key_same_fingerprint_replays_same_receipt_different_fingerprint_rejects",
            "accounting_role": "derivative_observation_only_after_accepted_receipt"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<WalletFrontDoorTruthContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_doc_states_wallet_product_beta_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-mutation-frontdoor.md");

    assert_all(
        "svc-wallet stabilization doc",
        &doc,
        &[
            "svc-wallet is the mutation front-door",
            "ron-ledger is durable economic truth",
            "idempotency_key is retry metadata",
            "receipt_hash is backend evidence",
            "settlement_status remains accepted-only",
            "gateway mutation bypass",
            "accounting mutation",
            "client-created receipt",
            "cache-only unlock",
            "bridge runtime",
            "ROX/Solana runtime",
            "external settlement runtime",
        ],
    );
}

fn assert_all(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "{label} must contain required marker {needle:?}"
        );
    }
}

fn assert_any(label: &str, haystack: &str, needles: &[&str]) {
    assert!(
        needles.iter().any(|needle| haystack.contains(needle)),
        "{label} must contain one of {needles:?}"
    );
}

fn assert_none(label: &str, haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !haystack.contains(needle),
            "{label} must not contain forbidden marker {needle:?}"
        );
    }
}

fn read_rel(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
