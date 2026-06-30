//! RO:WHAT — Internal ROC Beta Phase 3 Round 1 reward-plan policy gate tests.
//! RO:WHY — Proves ron-policy may gate/review reward plans but cannot create receipt, balance, payout execution, finality, wallet mutation, ledger mutation, bridge, staking, liquidity, or external settlement truth.
//! RO:INTERACTS — load_json, Evaluator, Context, DecisionEffect, obligations.
//! RO:INVARIANTS — policy allow/deny/obligations are declarative only; svc-wallet remains mutation front-door; ron-ledger remains durable truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — rejects known authority-shaped policy tags and obligation params.
//! RO:TEST — cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use ron_policy::{ctx::clock::SystemClock, load_json, Context, DecisionEffect, Evaluator};
use serde_json::{json, Value};

const SAFE_REWARD_PLAN_TAGS: &[&str] = &[
    "reward-plan-reviewed",
    "bounded-pool-cap-checked",
    "accounting-snapshot-cid-checked",
    "policy-gate-only",
    "backend-wallet-execution-required",
];

const FORBIDDEN_POLICY_AUTHORITY_TOKENS: &[&str] = &[
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
    "payout_execution",
];

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "internal-roc-beta-phase3-reward-plan-gate",
                "when": {
                    "tenant": "*",
                    "method": "POST",
                    "region": "*",
                    "require_tags_all": tags
                },
                "action": "allow",
                "reason": "declarative reward-plan policy gate only",
                "obligations": [
                    {
                        "kind": "require-approved-payout-intent-through-svc-wallet",
                        "params": {
                            "plan_source": "svc_rewarder",
                            "execution_boundary": "svc_wallet",
                            "ledger_truth": "ron_ledger"
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
        "rules": [
            {
                "id": "internal-roc-beta-phase3-reward-plan-param-poison",
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
    for forbidden in FORBIDDEN_POLICY_AUTHORITY_TOKENS {
        assert!(
            !debug.contains(forbidden),
            "policy decision/debug output must not contain authority token `{forbidden}`: {debug}"
        );
    }
}

#[test]
fn reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth() {
    let bundle = load_json(&policy_with_required_tags(SAFE_REWARD_PLAN_TAGS))
        .expect("safe reward-plan policy gate should load");
    let evaluator = Evaluator::new(&bundle).expect("policy should validate");

    let mut builder = Context::builder()
        .tenant("internal-roc-beta")
        .method("POST")
        .region("US");
    for tag in SAFE_REWARD_PLAN_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative reward-plan policy gate only")
    );
    assert_eq!(decision.obligations.items.len(), 1);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-approved-payout-intent-through-svc-wallet"
    );

    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn reward_plan_policy_denial_is_not_refund_receipt_or_balance_truth() {
    let bundle = load_json(&policy_with_required_tags(SAFE_REWARD_PLAN_TAGS))
        .expect("safe reward-plan policy gate should load");
    let evaluator = Evaluator::new(&bundle).expect("policy should validate");

    let ctx = Context::builder()
        .tenant("internal-roc-beta")
        .method("POST")
        .region("US")
        .tag("reward-plan-reviewed")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Deny);
    assert!(
        decision.obligations.items.is_empty(),
        "deny decision must not emit payout/receipt obligations"
    );
    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn known_authority_shaped_reward_plan_policy_tags_reject() {
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
            .expect_err("authority-shaped reward-plan policy tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn known_authority_shaped_reward_plan_obligation_params_reject() {
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
            .expect_err("authority-shaped reward-plan obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn reward_plan_policy_gate_remains_declarative_not_execution_surface() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "reward-plan-gate-with-safe-obligations",
              "when": {
                "tenant": "internal-roc-beta",
                "method": "POST",
                "region": "US",
                "require_tags_all": [
                  "reward-plan-reviewed",
                  "bounded-pool-cap-checked",
                  "policy-gate-only"
                ]
              },
              "action": "allow",
              "reason": "reviewed reward plan may proceed to explicit wallet boundary",
              "obligations": [
                {
                  "kind": "require-approved-payout-intent-through-svc-wallet",
                  "params": {
                    "plan_source": "svc_rewarder",
                    "policy_role": "gate_only",
                    "execution_boundary": "svc_wallet",
                    "truth_boundary": "ron_ledger"
                  }
                }
              ]
            }
          ]
        }"#,
    )
    .expect("safe reward-plan gate policy should load");

    let evaluator = Evaluator::new(&bundle).expect("policy should validate");
    let decision = evaluator
        .evaluate(
            &Context::builder()
                .tenant("internal-roc-beta")
                .method("POST")
                .region("US")
                .tag("reward-plan-reviewed")
                .tag("bounded-pool-cap-checked")
                .tag("policy-gate-only")
                .build(&SystemClock),
        )
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(decision.obligations.items.len(), 1);
    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}
