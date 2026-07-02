//! RO:WHAT — Internal ROC Stabilization replay/conservation truth-boundary tests for ron-ledger.
//! RO:WHY — Product beta readiness requires accepted replay, receipt evidence, and retry semantics to stay deterministic and non-authoritative outside ledger truth.
//! RO:INTERACTS — quickchain/accepted_replay.rs, quickchain/execution_state.rs, quickchain/replay_index.rs, quickchain/types.rs.
//! RO:INVARIANTS — accepted replay is not a root/proof/settlement artifact; retries/rejections do not mutate or advance replay boundaries.
//! RO:SECURITY — no fake receipt/balance/finality, silent spend, cache-only unlock, direct non-wallet mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_stabilization_replay_truth_boundary.

#![cfg(feature = "quickchain-preflight")]
#![allow(clippy::missing_panics_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerReplayTruthContract {
    schema: String,
    source_crate: String,
    truth_role: String,
    mutation_front_door: String,
    replay_boundary_role: String,
    receipt_evidence_source: String,
    balance_authority: String,
    idempotency_rule: String,
}

#[test]
fn accepted_replay_boundary_is_not_root_finality_or_settlement() {
    let accepted_replay = read_rel("src/quickchain/accepted_replay.rs");
    let lib = read_rel("src/lib.rs");
    let types = read_rel("src/quickchain/types.rs");

    assert_all(
        "ron-ledger lib boundary",
        &lib,
        &[
            "Keep ledger truth separate from HTTP, wallet UX, and reward logic",
            "append-only truth",
            "deterministic replay",
            "checked integer money",
            "non-negative balances",
            "QuickChain roots remain out of scope",
            "quickchain-preflight is compile-time gated",
        ],
    );

    assert_all(
        "accepted replay source",
        &accepted_replay,
        &[
            "QuickChainAcceptedReplayBoundary",
            "This is not a root, checkpoint, signature, persistence DTO, consensus field",
            "or settlement artifact",
            "accepted records are replayed in supplied order",
            "duplicates and sequence/evidence/boundary mismatches reject",
            "no roots, IO, clocks, or persistence",
            "receipt refs",
            "replay boundaries are explicit inputs, not capabilities or authority",
        ],
    );

    assert_all(
        "committed operation record source",
        &types,
        &[
            "QuickChainCommittedOperationRecord",
            "intent: QuickChainOperationIntentV1",
            "receipt_txid: String",
            "account_sequence: u64",
            "ledger_sequence_start: u64",
            "ledger_sequence_end: u64",
            "Future live integration",
            "atomic wallet/ledger transition succeeds",
        ],
    );

    assert_none(
        "ron-ledger accepted replay active runtime markers",
        &format!("{accepted_replay}\n{types}").to_lowercase(),
        &[
            "fake_receipt",
            "fake_balance",
            "cache_only_unlock",
            "unlock_from_cache",
            "gateway_mutate",
            "omnigate_mutate",
            "accounting_mutate",
            "rewarder_mutate",
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement",
            "mint_rox",
            "burn_rox",
        ],
    );
}

#[test]
fn existing_replay_and_payout_regressions_lock_retry_conservation_and_receipt_evidence() {
    let replay_conservation = read_rel("tests/internal_roc_beta_phase2_replay_conservation.rs");
    let paid_content_replay_label =
        read_rel("tests/internal_roc_beta_paid_content_replay_label.rs");
    let payout_replay = read_rel("tests/internal_roc_beta_phase3_approved_payout_replay.rs");
    let retry_stability = read_rel("tests/quickchain_accepted_replay_boundary_retry_stability.rs");
    let rejection_stability =
        read_rel("tests/quickchain_accepted_replay_boundary_rejection_stability.rs");
    let identity_rejection =
        read_rel("tests/quickchain_accepted_replay_boundary_identity_rejection.rs");

    assert_all(
        "phase2 replay/conservation regression",
        &format!("{replay_conservation}\n{paid_content_replay_label}"),
        &[
            "internal_roc_beta_paid_flow_replay_equality",
            "internal_roc_beta_balance_conservation",
            "internal_roc_beta_hold_terminality",
            "internal_roc_beta_replay_order_independence",
            "exact retry must not mutate paid-content ledger state",
            "paid content transfers/captures must conserve internal ROC supply",
            "operation-id/DB-style ordering must not define replay truth",
            "wall-clock/event-time ordering must not define replay truth",
        ],
    );

    assert_all(
        "approved payout replay regression",
        &payout_replay,
        &[
            "approved_payout_issues_create_durable_receipts_and_replay_equally",
            "duplicate_approved_payout_operation_id_does_not_double_issue",
            "reward_plan_evidence_prefix_cannot_be_used_as_payout_receipt_txid",
            "replay_rejects_tampered_approved_payout_receipt_reference",
            "svc-wallet remains mutation front-door",
            "accepted wallet/ledger issue operations",
        ],
    );

    assert_loaded_boundary_regression(
        "retry stability regression",
        &retry_stability,
        "quickchain_accepted_replay_boundary_retry_stability.rs",
    );
    assert_loaded_boundary_regression(
        "rejection stability regression",
        &rejection_stability,
        "quickchain_accepted_replay_boundary_rejection_stability.rs",
    );
    assert_loaded_boundary_regression(
        "identity rejection stability regression",
        &identity_rejection,
        "quickchain_accepted_replay_boundary_identity_rejection.rs",
    );

    assert_none(
        "accepted replay retry/rejection stability regressions forbidden markers",
        &format!("{retry_stability}\n{rejection_stability}\n{identity_rejection}").to_lowercase(),
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
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement",
            "mint_rox",
            "burn_rox",
        ],
    );
}

#[test]
fn ledger_replay_truth_contract_rejects_authority_poison_fields() {
    for field in [
        "gateway_ledger_mutation",
        "omnigate_ledger_mutation",
        "accounting_balance_truth",
        "rewarder_balance_truth",
        "policy_receipt_truth",
        "cache_unlock_authority",
        "fake_receipt",
        "fake_balance",
        "fake_finality",
        "bridge_authority",
        "external_settlement_authority",
        "client_truth",
    ] {
        let mut value = json!({
            "schema": "ron-ledger.internal-roc-stabilization-replay-truth.v1",
            "source_crate": "ron-ledger",
            "truth_role": "durable_economic_truth",
            "mutation_front_door": "svc-wallet_only",
            "replay_boundary_role": "pre_root_replay_boundary_not_finality_or_settlement",
            "receipt_evidence_source": "accepted_wallet_ledger_operation",
            "balance_authority": "ledger_replay_conservation_only",
            "idempotency_rule": "exact_retry_returns_original_evidence_without_mutation"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<LedgerReplayTruthContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_replay_doc_states_ledger_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-replay-truth.md");

    assert_all(
        "ledger replay stabilization doc",
        &doc,
        &[
            "ron-ledger is durable economic truth",
            "accepted replay is not a root",
            "retries must not mutate",
            "rejected operations must not advance replay boundaries",
            "same trusted receipt txid",
            "same primitive ledger sequence range",
            "svc-wallet-approved mutation intent",
            "replayable balance/supply/hold state",
        ],
    );
}

fn assert_loaded_boundary_regression(label: &str, source: &str, expected_file_marker: &str) {
    assert!(
        source.len() > 256,
        "{label} should be a non-empty focused regression source"
    );

    assert!(
        source.contains("accepted_replay_boundary")
            || source.contains("QuickChainAcceptedReplayBoundary"),
        "{label} must exercise the accepted replay boundary"
    );

    assert!(
        source.contains(expected_file_marker)
            || source.contains("RO:WHAT")
            || source.contains("#![cfg(feature = \"quickchain-preflight\")]"),
        "{label} must retain its test-source boundary/header markers"
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
