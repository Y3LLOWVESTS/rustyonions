//! RO:WHAT — Internal ROC Stabilization boundary tests for ron-policy declarative gate non-authority.
//! RO:WHY — Product beta readiness requires policy/economics decisions to remain gates only, never economic truth or mutation.
//! RO:INTERACTS — model.rs, engine/eval.rs, parse/validate.rs, economics/validate.rs, Internal ROC beta policy regressions.
//! RO:INVARIANTS — policy allows/denies/explains only; svc-wallet mutates; ron-ledger records truth.
//! RO:SECURITY — no receipt/balance/payout/finality/unlock/wallet/ledger/bridge/staking/liquidity/external-settlement authority.
//! RO:TEST — cargo test -p ron-policy --test internal_roc_stabilization_policy_gate_non_authority_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyGateNonAuthorityContract {
    schema: String,
    source_crate: String,
    role: String,
    decision_role: String,
    economics_role: String,
    wallet_truth: String,
    ledger_truth: String,
    payout_execution_role: String,
}

#[test]
fn policy_sources_keep_declarative_gate_and_economics_non_authority_boundaries() {
    let model = read_rel("src/model.rs");
    let eval = read_rel("src/engine/eval.rs");
    let obligations = read_rel("src/engine/obligations.rs");
    let parse_validate = read_rel("src/parse/validate.rs");
    let economics_types = read_rel("src/economics/types.rs");
    let economics_validate = read_rel("src/economics/validate.rs");

    assert_all(
        "policy model source",
        &model,
        &[
            "PolicyBundle",
            "Rule",
            "RuleCondition",
            "Action",
            "Obligation",
            "#[serde(deny_unknown_fields)]",
        ],
    );

    assert_all(
        "policy evaluator source",
        &eval,
        &[
            "Evaluator",
            "Decision",
            "DecisionEffect",
            "evaluate",
            "default_action",
        ],
    );

    assert_all(
        "policy obligations source",
        &obligations,
        &["ObligationSet", "obligations"],
    );

    assert_all(
        "policy parse validator source",
        &parse_validate,
        &[
            "validate",
            "economic authority",
            "receipt",
            "balance",
            "wallet",
            "ledger",
        ],
    );

    assert_all(
        "policy economics DTO source",
        &economics_types,
        &[
            "EconomicsPolicy",
            "ActionEconomics",
            "PayoutSplit",
            "PricingKind",
            "RoundingMode",
            "#[serde(deny_unknown_fields)]",
        ],
    );

    assert_any_lower(
        "policy economics validator entrypoint",
        &economics_validate,
        &[
            "validate_economics_policy",
            "validate_policy",
            "pub fn validate",
            "validate(",
        ],
    );
    assert_any_lower(
        "policy economics action validation",
        &economics_validate,
        &["validate_action", "action", "actions"],
    );
    assert_any_lower(
        "policy economics split validation",
        &economics_validate,
        &["split", "splits", "payout"],
    );
    assert_any_lower(
        "policy economics bps validation",
        &economics_validate,
        &["basis points", "bps", "10_000", "10000"],
    );

    assert_none(
        "ron-policy stabilization production source forbidden authority markers",
        &format!(
            "{model}\n{eval}\n{obligations}\n{parse_validate}\n{economics_types}\n{economics_validate}"
        )
        .to_lowercase(),
        &[
            "svc_wallet::",
            "ron_ledger::",
            "ron_accounting::",
            "svc_rewarder::",
            "wallet_mutate(",
            "ledger_mutate(",
            "mutate_balance(",
            "set_balance(",
            "credit_account(",
            "debit_account(",
            "create_receipt(",
            "insert_receipt(",
            "commit_receipt(",
            "grant_paid_access(",
            "unlock_paid_content(",
            "payout_execution_authority:true",
            "\"payout_execution_authority\":true",
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
fn existing_policy_regressions_are_wired_into_stabilization_surface() {
    let paid_content = read_rel("tests/internal_roc_beta_paid_content_policy_non_authority.rs");
    let reward_plan = read_rel("tests/internal_roc_beta_phase3_reward_plan_policy_gate.rs");
    let approved_payout = read_rel("tests/internal_roc_beta_phase3_approved_payout_policy_gate.rs");
    let economics = read_rel("tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs");
    let antifarming = read_rel("tests/internal_roc_beta_phase5_antifarming_policy_gate.rs");
    let decision = read_rel("tests/quickchain_preflight_decision_non_authority.rs");
    let economics_config = read_rel("tests/quickchain_preflight_economics_config_non_authority.rs");

    assert_loaded_boundary_regression(
        "paid-content policy non-authority regression",
        &paid_content,
        "paid",
    );

    assert_all(
        "reward-plan policy gate regression",
        &reward_plan,
        &[
            "reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth",
            "reward_plan_policy_denial_is_not_refund_receipt_or_balance_truth",
            "reward_plan_policy_gate_remains_declarative_not_execution_surface",
        ],
    );

    assert_all(
        "approved-payout policy gate regression",
        &approved_payout,
        &[
            "approved_payout_policy_gate_allows_candidate_without_execution_truth",
            "missing_duplicate_guard_marker_denies_without_refund_or_receipt_authority",
            "approved_payout_policy_gate_remains_declarative_not_execution_surface",
        ],
    );

    assert_loaded_boundary_regression(
        "economics TOML policy validation regression",
        &economics,
        "economics",
    );

    assert_loaded_boundary_regression(
        "anti-farming policy gate regression",
        &antifarming,
        "policy",
    );
    assert_any_lower(
        "anti-farming analytics-only lane",
        &antifarming,
        &["analytics_only", "analytics-only", "analyticsonly"],
    );
    assert_any_lower("anti-farming metering lane", &antifarming, &["metering"]);
    assert_any_lower(
        "anti-farming proof-eligible lane",
        &antifarming,
        &["proof_eligible", "proof-eligible", "proofeligible"],
    );
    assert_any_lower(
        "anti-farming ad-budgeted lane",
        &antifarming,
        &["ad_budgeted", "ad-budgeted", "adbudgeted"],
    );

    assert_loaded_boundary_regression(
        "QuickChain decision non-authority regression",
        &decision,
        "decision",
    );

    assert_loaded_boundary_regression(
        "QuickChain economics config non-authority regression",
        &economics_config,
        "economics",
    );
}

#[test]
fn policy_gate_contract_rejects_authority_poison_fields() {
    for field in [
        "wallet_mutation_authority",
        "ledger_mutation_authority",
        "issue_authority",
        "transfer_authority",
        "burn_authority",
        "hold_authority",
        "capture_authority",
        "release_authority",
        "receipt_truth",
        "balance_truth",
        "payout_execution_truth",
        "refund_authority",
        "paid_unlock_authority",
        "finality_truth",
        "settlement_truth",
        "bridge_authority",
        "external_settlement_authority",
        "rox_solana_authority",
        "staking_authority",
        "liquidity_authority",
    ] {
        let mut value = json!({
            "schema": "ron-policy.internal-roc-stabilization-policy-gate-non-authority.v1",
            "source_crate": "ron-policy",
            "role": "declarative_policy_and_economics_gate_only",
            "decision_role": "allow_deny_reason_obligation_only",
            "economics_role": "validated_config_and_pricing_input_only",
            "wallet_truth": "svc-wallet_only",
            "ledger_truth": "ron-ledger_only",
            "payout_execution_role": "svc-wallet_only_after_policy_gate_and_rewarder_plan"
        });

        value
            .as_object_mut()
            .expect("contract object")
            .insert(field.to_owned(), json!(true));

        let err = serde_json::from_value::<PolicyGateNonAuthorityContract>(value)
            .expect_err("authority poison fields must reject");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stabilization_doc_states_policy_product_beta_truth_boundary() {
    let doc = read_rel("docs/internal-roc-stabilization-policy-gate-non-authority.md");

    assert_all(
        "ron-policy stabilization doc",
        &doc,
        &[
            "declarative policy/economics gate",
            "Policy must not manufacture receipts",
            "policy decision is not economic truth",
            "policy allow is not paid proof",
            "policy obligation is not receipt proof",
            "economics config is not wallet authority",
            "policy allow → wallet mutation",
            "policy allow → ledger mutation",
            "policy allow → receipt truth",
            "policy allow → balance truth",
            "policy allow → payout execution",
            "policy allow → paid unlock",
            "economics config → balance mutation",
            "Services enforce decisions. svc-wallet mutates. ron-ledger records truth.",
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

fn assert_any_lower(label: &str, haystack: &str, needles: &[&str]) {
    let lower = haystack.to_lowercase();
    assert!(
        needles.iter().any(|needle| lower.contains(needle)),
        "{label} must contain one of {needles:?}"
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
