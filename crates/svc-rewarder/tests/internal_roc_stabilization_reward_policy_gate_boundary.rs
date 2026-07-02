//! RO:WHAT — Internal ROC Stabilization boundary tests for svc-rewarder reward planning and policy-gated payout handoff.
//! RO:WHY — Product beta readiness requires rewarder to stay deterministic, capped, policy-gated, and non-mutating.
//! RO:INTERACTS — inputs/anti_farming.rs, outputs/intents.rs, outputs/wallet.rs, Internal ROC beta rewarder regressions.
//! RO:INVARIANTS — rewarder plans only; ron-policy gates; svc-wallet mutates; ron-ledger records truth.
//! RO:SECURITY — no raw engagement payout, fake receipt/balance/finality, direct ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.
//! RO:TEST — cargo test -p svc-rewarder --test internal_roc_stabilization_reward_policy_gate_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RewardPolicyGateContract {
    schema: String,
    source_crate: String,
    role: String,
    policy_gate: String,
    accounting_input_role: String,
    wallet_handoff_role: String,
    ledger_truth: String,
    payout_execution_role: String,
}

#[test]
fn rewarder_sources_keep_policy_gated_planning_and_wallet_handoff_boundaries() {
    let anti_farming = read_rel("src/inputs/anti_farming.rs");
    let accounting = read_rel("src/inputs/accounting.rs");
    let compute = read_rel("src/core/compute.rs");
    let intents = read_rel("src/outputs/intents.rs");
    let wallet = read_rel("src/outputs/wallet.rs");

    assert_all(
        "rewarder anti-farming source",
        &anti_farming,
        &[
            "RewardInputEventClass",
            "Metering",
            "ProofEligible",
            "AdBudgeted",
            "AnalyticsOnly",
            "AntiFarmingCapPolicy",
            "CappedRewardInputCandidate",
            "policy_gate_passed",
            "explicit_budget_authorized",
            "capped_contributions_from_candidates",
        ],
    );

    assert_all(
        "rewarder accounting input source",
        &accounting,
        &[
            "AccountingSnapshot",
            "AccountContribution",
            "canonical_snapshot_cid",
            "inputs_cid == canonical_snapshot_cid(snapshot)",
            "#[serde(deny_unknown_fields)]",
        ],
    );

    assert_all(
        "rewarder compute source",
        &compute,
        &[
            "compute_manifest",
            "ComputeInput",
            "RewardManifest",
            "IntentResult",
        ],
    );

    assert_all(
        "rewarder settlement intent source",
        &intents,
        &[
            "SettlementIntent",
            "SettlementBatch",
            "WalletIssueRequest",
            "WalletIssueBatch",
            "WALLET_ISSUE_PATH",
            "to_wallet_issue_request",
            "#[serde(deny_unknown_fields)]",
        ],
    );

    assert_all(
        "rewarder wallet handoff source",
        &wallet,
        &[
            "svc-wallet",
            "WalletIssueClient",
            "preview_issue_batch",
            "emit_issue_batch",
            "dry_run",
            "WalletHttpIssueOutcome",
            "Wallet receipts returned by `svc-wallet`",
        ],
    );

    assert_none(
        "rewarder stabilization production source forbidden authority markers",
        &format!("{anti_farming}\n{accounting}\n{compute}\n{intents}\n{wallet}").to_lowercase(),
        &[
            "ron_ledger::",
            "ledgerclient",
            "ledger_commit(",
            "direct_ledger_mutation",
            "rewarder_ledger_mutation",
            "fake_receipt",
            "fake_balance",
            "fake_finality",
            "cache_only_unlock",
            "unlock_from_cache",
            "bridge_runtime",
            "rox_runtime",
            "solana_runtime",
            "staking_runtime",
            "liquidity_runtime",
            "external_settlement_runtime",
            "mint_rox(",
            "burn_rox(",
        ],
    );
}

#[test]
fn existing_rewarder_regressions_are_wired_into_stabilization_surface() {
    let planning = read_rel("tests/internal_roc_beta_rewarder_planning_non_authority.rs");
    let reward_plan = read_rel("tests/internal_roc_beta_phase3_reward_plan_boundary.rs");
    let payout_intent =
        read_rel("tests/internal_roc_beta_phase3_approved_payout_intent_boundary.rs");
    let config = read_rel("tests/internal_roc_beta_phase5_config_driven_planning.rs");
    let antifarming = read_rel("tests/internal_roc_beta_phase5_antifarming_event_gates.rs");
    let policy_gate = read_rel("tests/internal_roc_beta_phase5_policy_gate_interlock.rs");
    let no_direct_mutation = read_rel("tests/quickchain_preflight_no_direct_mutation.rs");
    let replay = read_rel("tests/quickchain_preflight_replay_no_double_issue.rs");

    assert_loaded_boundary_regression(
        "rewarder planning non-authority regression",
        &planning,
        "rewarder_planning",
    );
    assert_loaded_boundary_regression(
        "reward-plan boundary regression",
        &reward_plan,
        "reward_plan",
    );
    assert_loaded_boundary_regression(
        "approved payout intent boundary regression",
        &payout_intent,
        "approved_payout",
    );
    assert_loaded_boundary_regression("config-driven planning regression", &config, "config");

    assert_all(
        "anti-farming event gate regression",
        &antifarming,
        &[
            "RewardInputEventClass::AnalyticsOnly",
            "RewardInputEventClass::Metering",
            "RewardInputEventClass::ProofEligible",
            "RewardInputEventClass::AdBudgeted",
        ],
    );

    assert_all(
        "policy gate interlock regression",
        &policy_gate,
        &[
            "rewarder_rejects_verified_capped_candidate_without_policy_gate",
            "verification/caps alone must not bypass ron-policy",
            "policy_gate_passed",
            "IntentResult::DryRun",
        ],
    );

    assert_loaded_boundary_regression(
        "no direct mutation regression",
        &no_direct_mutation,
        "direct",
    );
    assert_loaded_boundary_regression("replay no-double-issue regression", &replay, "replay");
}

#[test]
fn reward_policy_gate_contract_rejects_authority_poison_fields() {
    for field in [
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "direct_ledger_mutation",
        "receipt_truth",
        "balance_truth",
        "payout_execution_truth",
        "paid_unlock_authority",
        "finality_truth",
        "raw_engagement_mint_authority",
        "analytics_payout_authority",
        "metering_direct_payout_authority",
        "ad_budgeted_protocol_pool_authority",
        "bridge_authority",
        "external_settlement_authority",
        "rox_solana_authority",
    ] {
        let mut value = json!({
            "schema": "svc-rewarder.internal-roc-stabilization-reward-policy-gate.v1",
            "source_crate": "svc-rewarder",
            "role": "deterministic_capped_reward_planning_only",
            "policy_gate": "ron-policy_required_for_proof_eligible_and_ad_budgeted_material",
            "accounting_input_role": "derivative_snapshot_input_only",
            "wallet_handoff_role": "wallet_issue_request_dto_only",
            "ledger_truth": "ron-ledger_only_after_svc-wallet_accepts",
            "payout_execution_role": "svc-wallet_only"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<RewardPolicyGateContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_doc_states_rewarder_product_beta_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-reward-policy-gate.md");

    assert_all(
        "svc-rewarder stabilization doc",
        &doc,
        &[
            "deterministic capped reward planner",
            "ron-policy gates eligibility declaratively",
            "svc-wallet remains mutation front-door",
            "ron-ledger remains durable truth",
            "idempotent wallet issue requests are handoff DTOs",
            "raw engagement → payout",
            "analytics_only → payout",
            "metering → direct payout",
            "proof_eligible without policy gate → payout",
            "ad_budgeted without explicit non-protocol budget",
            "rewarder plan → direct ledger mutation",
            "rewarder plan → receipt truth",
            "rewarder plan → balance truth",
            "rewarder plan → finality truth",
        ],
    );
}

fn assert_loaded_boundary_regression(label: &str, source: &str, expected_marker: &str) {
    assert!(
        source.len() > 256,
        "{label} should be a non-empty focused regression source"
    );
    assert!(
        source.contains("RO:WHAT")
            || source.contains("Internal ROC")
            || source.contains("#![allow"),
        "{label} must retain boundary/test header markers"
    );
    assert!(
        source.to_lowercase().contains(expected_marker),
        "{label} must contain expected marker {expected_marker:?}"
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
