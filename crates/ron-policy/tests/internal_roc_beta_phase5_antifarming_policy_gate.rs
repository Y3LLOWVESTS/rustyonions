#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Internal ROC Beta Phase 5 Round 2 anti-farming policy-gate tests.
//! RO:WHY — Proves ron-policy gates reward eligibility declaratively and rejects raw engagement, analytics-only, direct metering, uncapped, unverified, and unfunded ad-budgeted reward material.
//! RO:INTERACTS — policy parser/evaluator, Phase 3 reward-plan policy gates, Phase 5 economics TOML validation, svc-rewarder anti-farming inputs.
//! RO:INVARIANTS — policy validates/gates only; no receipt, balance, payout execution, finality, wallet mutation, ledger mutation, bridge, staking, liquidity, or external settlement truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — rejects authority-shaped tags/obligations and keeps policy as a declarative gate.
//! RO:TEST — `cargo test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate`.

use ron_policy::{ctx::clock::SystemClock, load_json, Context, DecisionEffect, Evaluator};
use serde_json::json;

const SAFE_PROOF_ELIGIBLE_TAGS: &[&str] = &[
    "reward-plan-reviewed",
    "proof-eligible",
    "verified-caps-checked",
    "anti-farming-cap-checked",
    "policy-gate-only",
    "svc-wallet-execution-required",
];

const SAFE_AD_BUDGETED_TAGS: &[&str] = &[
    "reward-plan-reviewed",
    "ad-budgeted",
    "explicit-budget-authorized",
    "verified-caps-checked",
    "anti-farming-cap-checked",
    "policy-gate-only",
    "non-protocol-budget",
];

const FORBIDDEN_AUTHORITY_TOKENS: &[&str] = &[
    "issue",
    "transfer",
    "burn",
    "capture",
    "release",
    "receipt",
    "balance",
    "finality",
    "finalized",
    "bridge",
    "staking",
    "liquidity",
    "settlement",
    "mint",
    "unlock",
];

fn antifarming_policy_bundle() -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "allow-proof-eligible-only-after-verification-caps-and-policy-gate",
                "when": {
                    "tenant": "internal-roc-beta",
                    "method": "POST",
                    "region": "US",
                    "require_tags_all": SAFE_PROOF_ELIGIBLE_TAGS
                },
                "action": "allow",
                "reason": "proof eligible reward material passed anti-farming policy gate only",
                "obligations": [
                    {
                        "kind": "record-policy-review-note",
                        "params": {
                            "note": "proof_eligible_policy_gate_only",
                            "verification": "required",
                            "caps": "required",
                            "wallet_handoff": "required_later"
                        }
                    },
                    {
                        "kind": "require-rewarder-capped-plan",
                        "params": {
                            "anti_farming": "required",
                            "policy_gate": "required"
                        }
                    }
                ]
            },
            {
                "id": "allow-ad-budgeted-only-after-explicit-non-protocol-budget",
                "when": {
                    "tenant": "internal-roc-beta",
                    "method": "POST",
                    "region": "US",
                    "require_tags_all": SAFE_AD_BUDGETED_TAGS
                },
                "action": "allow",
                "reason": "ad budgeted reward material passed explicit budget policy gate only",
                "obligations": [
                    {
                        "kind": "record-policy-review-note",
                        "params": {
                            "note": "ad_budgeted_policy_gate_only",
                            "explicit_budget": "required",
                            "protocol_pool": "forbidden"
                        }
                    }
                ]
            }
        ]
    })
    .to_string()
    .into_bytes()
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

fn evaluate(tags: &[&str]) -> ron_policy::Decision {
    let bundle =
        load_json(&antifarming_policy_bundle()).expect("anti-farming policy bundle should parse");
    let evaluator = Evaluator::new(&bundle).expect("anti-farming policy should validate");

    evaluator
        .evaluate(&context_with_tags(tags))
        .expect("policy should evaluate")
}

fn assert_decision_is_declarative_only(decision: &ron_policy::Decision) {
    let debug = format!("{decision:?}").to_ascii_lowercase();

    for forbidden in [
        "receipt_truth",
        "balance_truth",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "finality_truth",
        "bridge_settlement",
        "staking_position",
        "liquidity_pool",
        "external_settlement",
    ] {
        assert!(
            !debug.contains(forbidden),
            "policy decision debug must not claim economic authority via `{forbidden}`: {debug}"
        );
    }

    for obligation in &decision.obligations.items {
        let kind = obligation.kind.to_ascii_lowercase();
        for forbidden in FORBIDDEN_AUTHORITY_TOKENS {
            assert!(
                !kind.contains(forbidden),
                "safe policy obligation kind must not be execution authority: {kind:?}"
            );
        }

        for key in obligation.params.keys() {
            let key = key.to_ascii_lowercase();
            for forbidden in FORBIDDEN_AUTHORITY_TOKENS {
                assert!(
                    !key.contains(forbidden),
                    "safe policy obligation params must not be authority-shaped: {key:?}"
                );
            }
        }
    }
}

#[test]
fn policy_allows_verified_capped_proof_eligible_material_only_as_gate() {
    let decision = evaluate(SAFE_PROOF_ELIGIBLE_TAGS);

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("proof eligible reward material passed anti-farming policy gate only")
    );
    assert_eq!(decision.obligations.items.len(), 2);
    assert_decision_is_declarative_only(&decision);
}

#[test]
fn policy_denies_raw_engagement_analytics_and_direct_metering_reward_attempts() {
    for tags in [
        &[
            "reward-plan-reviewed",
            "analytics-only",
            "post-view",
            "policy-gate-only",
        ][..],
        &[
            "reward-plan-reviewed",
            "metering",
            "bytes-served",
            "policy-gate-only",
        ][..],
        &[
            "reward-plan-reviewed",
            "raw-engagement",
            "comment-impression",
            "policy-gate-only",
        ][..],
    ] {
        let decision = evaluate(tags);
        assert_eq!(decision.effect, DecisionEffect::Deny);
        assert_eq!(decision.reason.as_deref(), Some("default"));
        assert_decision_is_declarative_only(&decision);
    }
}

#[test]
fn policy_denies_unverified_or_uncapped_proof_eligible_material() {
    for tags in [
        &[
            "reward-plan-reviewed",
            "proof-eligible",
            "anti-farming-cap-checked",
            "policy-gate-only",
            "svc-wallet-execution-required",
        ][..],
        &[
            "reward-plan-reviewed",
            "proof-eligible",
            "verified-caps-checked",
            "policy-gate-only",
            "svc-wallet-execution-required",
        ][..],
        &[
            "reward-plan-reviewed",
            "proof-eligible",
            "verified-caps-checked",
            "anti-farming-cap-checked",
            "svc-wallet-execution-required",
        ][..],
    ] {
        let decision = evaluate(tags);
        assert_eq!(decision.effect, DecisionEffect::Deny);
        assert_eq!(decision.reason.as_deref(), Some("default"));
    }
}

#[test]
fn policy_allows_ad_budgeted_only_with_explicit_non_protocol_budget() {
    let decision = evaluate(SAFE_AD_BUDGETED_TAGS);

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("ad budgeted reward material passed explicit budget policy gate only")
    );
    assert_eq!(decision.obligations.items.len(), 1);
    assert_decision_is_declarative_only(&decision);
}

#[test]
fn policy_denies_ad_budgeted_without_explicit_budget_or_non_protocol_budget() {
    for tags in [
        &[
            "reward-plan-reviewed",
            "ad-budgeted",
            "verified-caps-checked",
            "anti-farming-cap-checked",
            "policy-gate-only",
            "non-protocol-budget",
        ][..],
        &[
            "reward-plan-reviewed",
            "ad-budgeted",
            "explicit-budget-authorized",
            "verified-caps-checked",
            "anti-farming-cap-checked",
            "policy-gate-only",
        ][..],
        &[
            "reward-plan-reviewed",
            "ad-budgeted",
            "explicit-budget-authorized",
            "verified-caps-checked",
            "anti-farming-cap-checked",
            "policy-gate-only",
            "protocol-pool",
        ][..],
    ] {
        let decision = evaluate(tags);
        assert_eq!(decision.effect, DecisionEffect::Deny);
        assert_eq!(decision.reason.as_deref(), Some("default"));
    }
}

#[test]
fn policy_parser_rejects_authority_shaped_reward_gate_tags() {
    for forbidden_tag in [
        "balance_minor",
        "receipt_hash",
        "wallet_balance",
        "paid_proof",
        "unlock_granted",
        "finality",
        "bridge_proof",
        "staking_position_id",
        "liquidity_pool_id",
        "operation_id",
        "account_sequence",
    ] {
        let bundle = json!({
            "version": 1,
            "defaults": { "default_action": "deny" },
            "rules": [
                {
                    "id": "forbidden-authority-tag",
                    "when": {
                        "tenant": "internal-roc-beta",
                        "method": "POST",
                        "require_tags_all": [forbidden_tag]
                    },
                    "action": "allow",
                    "reason": "must reject before evaluation"
                }
            ]
        })
        .to_string()
        .into_bytes();

        assert!(
            load_json(&bundle).is_err(),
            "authority-shaped reward gate tag `{forbidden_tag}` must be rejected"
        );
    }
}

#[test]
fn policy_parser_rejects_authority_shaped_reward_gate_obligations() {
    for forbidden_kind in [
        "issue-roc",
        "transfer-roc",
        "create-receipt",
        "mutate-balance",
        "unlock-paid-content",
        "bridge-settlement",
        "staking-position",
    ] {
        let bundle = json!({
            "version": 1,
            "defaults": { "default_action": "deny" },
            "rules": [
                {
                    "id": "forbidden-authority-obligation",
                    "when": {
                        "tenant": "internal-roc-beta",
                        "method": "POST",
                        "require_tags_all": ["policy-gate-only"]
                    },
                    "action": "allow",
                    "reason": "must reject before evaluation",
                    "obligations": [
                        {
                            "kind": forbidden_kind,
                            "params": {
                                "review": "required"
                            }
                        }
                    ]
                }
            ]
        })
        .to_string()
        .into_bytes();

        assert!(
            load_json(&bundle).is_err(),
            "authority-shaped reward gate obligation `{forbidden_kind}` must be rejected"
        );
    }
}
