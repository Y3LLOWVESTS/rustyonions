//! RO:WHAT — Internal ROC Beta Phase 3 Round 2 approved-payout policy gate tests.
//! RO:WHY — Proves ron-policy gates approved payout intent candidates declaratively without creating receipt, balance, payout execution, finality, wallet mutation, ledger mutation, bridge, staking, liquidity, or external settlement truth.
//! RO:INTERACTS — load_json, Evaluator, Context, DecisionEffect, obligations.
//! RO:INVARIANTS — policy allow/deny/obligations are declarative only; svc-wallet remains mutation front-door; ron-ledger remains durable truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — rejects authority-shaped payout policy tags and obligation params.
//! RO:TEST — cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use ron_policy::{ctx::clock::SystemClock, load_json, Context, DecisionEffect, Evaluator};
use serde_json::{json, Value};

const SAFE_APPROVED_PAYOUT_TAGS: &[&str] = &[
    "reward-plan-reviewed",
    "bounded-pool-cap-checked",
    "duplicate-payout-guard-checked",
    "approved-payout-intent-candidate",
    "svc-wallet-execution-required",
    "policy-gate-only",
];

const FORBIDDEN_DECISION_TOKENS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "balance_minor",
    "wallet_balance",
    "ledger_balance",
    "paid_proof",
    "unlock_granted",
    "finality",
    "finalized",
    "settlement_status",
    "state_root",
    "checkpoint_root",
    "checkpoint_hash",
    "validator_signature",
    "bridge_proof",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "liquidity_pool_id",
    "operation_id",
    "account_sequence",
    "payout_execution_truth",
];

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "internal-roc-beta-phase3-approved-payout-gate",
                "when": {
                    "tenant": "*",
                    "method": "POST",
                    "region": "*",
                    "require_tags_all": tags
                },
                "action": "allow",
                "reason": "declarative approved-payout policy gate only",
                "obligations": [
                    {
                        "kind": "require-approved-payout-intent-through-svc-wallet",
                        "params": {
                            "plan_source": "svc_rewarder",
                            "execution_boundary": "svc_wallet",
                            "ledger_truth": "ron_ledger",
                            "duplicate_guard": "required",
                            "pool_cap": "required"
                        }
                    }
                ]
            }
        ]
    })
    .to_string()
    .into_bytes()
}

fn policy_with_obligation_param(param_key: &str) -> Vec<u8> {
    let mut params = serde_json::Map::new();
    params.insert(param_key.to_owned(), Value::String("forbidden".to_owned()));

    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "internal-roc-beta-phase3-approved-payout-param-poison",
                "when": {
                    "tenant": "*",
                    "method": "POST",
                    "region": "*"
                },
                "action": "allow",
                "reason": "obligation instruction only",
                "obligations": [
                    {
                        "kind": "require-approved-payout-intent-through-svc-wallet",
                        "params": params
                    }
                ]
            }
        ]
    })
    .to_string()
    .into_bytes()
}

fn assert_decision_debug_has_no_authority_fields(debug: &str) {
    let debug = debug.to_ascii_lowercase();
    for forbidden in FORBIDDEN_DECISION_TOKENS {
        assert!(
            !debug.contains(forbidden),
            "policy decision/debug output must not contain authority token `{forbidden}`: {debug}"
        );
    }
}

fn context_with_tags(tags: &[&str]) -> Context {
    let mut builder = Context::builder()
        .tenant("internal-roc-beta")
        .method("POST")
        .region("US");

    for tag in tags {
        builder = builder.tag(*tag);
    }

    builder.build(&SystemClock)
}

#[test]
fn approved_payout_policy_gate_allows_candidate_without_execution_truth() {
    let bundle = load_json(&policy_with_required_tags(SAFE_APPROVED_PAYOUT_TAGS))
        .expect("safe approved payout policy gate should load");
    let evaluator = Evaluator::new(&bundle).expect("policy should validate");

    let decision = evaluator
        .evaluate(&context_with_tags(SAFE_APPROVED_PAYOUT_TAGS))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative approved-payout policy gate only")
    );
    assert_eq!(decision.obligations.items.len(), 1);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-approved-payout-intent-through-svc-wallet"
    );
    assert_eq!(
        decision.obligations.items[0]
            .params
            .get("execution_boundary")
            .map(String::as_str),
        Some("svc_wallet")
    );
    assert_eq!(
        decision.obligations.items[0]
            .params
            .get("duplicate_guard")
            .map(String::as_str),
        Some("required")
    );
    assert_eq!(
        decision.obligations.items[0]
            .params
            .get("pool_cap")
            .map(String::as_str),
        Some("required")
    );

    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn missing_duplicate_guard_marker_denies_without_refund_or_receipt_authority() {
    let bundle = load_json(&policy_with_required_tags(SAFE_APPROVED_PAYOUT_TAGS))
        .expect("safe approved payout policy gate should load");
    let evaluator = Evaluator::new(&bundle).expect("policy should validate");

    let decision = evaluator
        .evaluate(&context_with_tags(&[
            "reward-plan-reviewed",
            "bounded-pool-cap-checked",
            "approved-payout-intent-candidate",
            "svc-wallet-execution-required",
            "policy-gate-only",
        ]))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Deny);
    assert!(
        decision.obligations.items.is_empty(),
        "deny decision must not emit payout, refund, receipt, or wallet obligations"
    );
    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn approved_payout_policy_tags_reject_authority_shapes() {
    for tag in [
        "receipt_hash",
        "balance_minor",
        "settlement_status",
        "checkpoint_root",
        "bridge_proof",
        "operation_id",
        "idempotency_key",
        "account_sequence",
    ] {
        let err = load_json(&policy_with_required_tags(&[tag]))
            .expect_err("authority-shaped payout policy tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn approved_payout_policy_obligation_params_reject_authority_shapes() {
    for key in [
        "receipt_hash",
        "balance_minor",
        "settlement_status",
        "checkpoint_root",
        "bridge_proof",
        "operation_id",
        "idempotency_key",
        "account_sequence",
    ] {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("authority-shaped payout obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn approved_payout_policy_gate_remains_declarative_not_execution_surface() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "approved-payout-gate-with-safe-obligations",
              "when": {
                "tenant": "internal-roc-beta",
                "method": "POST",
                "region": "US",
                "require_tags_all": [
                  "reward-plan-reviewed",
                  "bounded-pool-cap-checked",
                  "duplicate-payout-guard-checked",
                  "approved-payout-intent-candidate",
                  "svc-wallet-execution-required",
                  "policy-gate-only"
                ]
              },
              "action": "allow",
              "reason": "gate approved payout candidate only",
              "obligations": [
                {
                  "kind": "require-approved-payout-intent-through-svc-wallet",
                  "params": {
                    "review": "required",
                    "cap_check": "required",
                    "duplicate_check": "required",
                    "wallet_handoff": "required"
                  }
                },
                {
                  "kind": "record-policy-review-note",
                  "params": {
                    "note": "policy_review_only"
                  }
                }
              ]
            }
          ]
        }"#,
    )
    .expect("safe declarative approved payout policy should load");

    let evaluator = Evaluator::new(&bundle).expect("policy should validate");
    let decision = evaluator
        .evaluate(&context_with_tags(SAFE_APPROVED_PAYOUT_TAGS))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(decision.obligations.items.len(), 2);

    for obligation in &decision.obligations.items {
        for forbidden in [
            "issue",
            "transfer",
            "burn",
            "capture",
            "release",
            "receipt",
            "balance",
            "finality",
            "bridge",
            "staking",
            "liquidity",
        ] {
            assert!(
                !obligation.kind.to_ascii_lowercase().contains(forbidden),
                "safe policy obligation kind must not be execution authority: {:?}",
                obligation.kind
            );
        }
    }

    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}
