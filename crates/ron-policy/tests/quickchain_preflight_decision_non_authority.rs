//! RO:WHAT — Decision-shape non-authority tests for ron-policy QuickChain preflight.
//! RO:WHY — Ensures policy decisions/obligations do not become receipt, balance, unlock, root, or finality truth.
//! RO:INTERACTS — `PolicyBundle`, `Evaluator`, `Decision`, obligation validation.
//! RO:INVARIANTS — allow/deny is policy output only; backend wallet/ledger/storage paths prove truth.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};

const AUTHORITY_DEBUG_TOKENS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "balance_minor",
    "wallet_balance",
    "ledger_balance",
    "finality",
    "finalized",
    "unlock_granted",
    "settlement_status",
    "state_root",
    "checkpoint_root",
    "checkpoint_hash",
    "spend_authority",
    "capture_authority",
    "validator_signature",
    "mint_authority",
];

#[test]
fn allow_decision_is_policy_result_not_paid_unlock_or_receipt_truth() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "allow-after-caller-proof-check",
              "when": {
                "method": "GET",
                "require_tags_all": ["backend-proof-checked"]
              },
              "action": "allow",
              "reason": "policy allowed route after caller proof checks",
              "obligations": [
                {
                  "kind": "require-backend-wallet-ledger-proof",
                  "params": {
                    "proof_source": "external_wallet_ledger_path",
                    "consumer": "service_boundary"
                  }
                }
              ]
            }
          ]
        }"#,
    )
    .expect("safe policy bundle should load");

    let evaluator = Evaluator::new(&bundle).expect("bundle should validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("backend-proof-checked")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_eq!(decision.obligations.items.len(), 1);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-backend-wallet-ledger-proof"
    );
    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn deny_decision_is_still_not_receipt_balance_or_finality_truth() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "rules": [
            {
              "id": "deny-paid-route-without-caller-proof",
              "when": { "method": "GET" },
              "action": "deny",
              "reason": "caller did not provide backend proof context"
            }
          ]
        }"#,
    )
    .expect("safe deny policy bundle should load");

    let evaluator = Evaluator::new(&bundle).expect("bundle should validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Deny));
    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn authority_shaped_obligation_kind_rejects_policy() {
    let err = load_json(
        br#"{
          "version": 1,
          "rules": [
            {
              "id": "bad-authority-kind",
              "when": { "method": "GET" },
              "action": "allow",
              "obligations": [
                { "kind": "create_receipt", "params": {} }
              ]
            }
          ]
        }"#,
    )
    .expect_err("policy must reject authority-shaped obligation kinds");

    assert!(
        err.to_string().contains("economic authority"),
        "unexpected error: {err}"
    );
}

#[test]
fn authority_shaped_obligation_param_key_rejects_policy() {
    let err = load_json(
        br#"{
          "version": 1,
          "rules": [
            {
              "id": "bad-authority-param",
              "when": { "method": "GET" },
              "action": "allow",
              "obligations": [
                {
                  "kind": "require-backend-wallet-ledger-proof",
                  "params": { "unlock_granted": "true" }
                }
              ]
            }
          ]
        }"#,
    )
    .expect_err("policy must reject authority-shaped obligation parameter keys");

    assert!(
        err.to_string().contains("economic authority"),
        "unexpected error: {err}"
    );
}

fn assert_decision_debug_has_no_authority_fields(debug: &str) {
    let lower_debug = debug.to_ascii_lowercase();

    for &token in AUTHORITY_DEBUG_TOKENS {
        assert!(
            !lower_debug.contains(token),
            "policy decision/debug shape must not carry economic authority field token: {token}\n{debug}"
        );
    }
}
