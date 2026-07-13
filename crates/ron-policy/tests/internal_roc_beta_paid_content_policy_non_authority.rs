//! RO:WHAT — Internal ROC Beta paid-content policy/economics non-authority tests.
//! RO:WHY — Policy and economics config may gate/price paid content, but must not become receipt, balance, entitlement, payout, finality, wallet, or ledger truth.
//! RO:INTERACTS — ron_policy::load_json, Evaluator, Context, economics TOML loader.
//! RO:INVARIANTS — policy allow is not paid proof; economics config is not wallet authority; svc-wallet remains mutation front-door.
//! RO:METRICS — none.
//! RO:CONFIG — reads checked-in configs/roc-economics.toml for economics validation.
//! RO:SECURITY — rejects authority-shaped tags, obligations, and config fields.
//! RO:TEST — cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use std::collections::BTreeMap;

use ron_policy::{
    ctx::clock::SystemClock, economics::load_internal_roc_economics_toml, load_json, Context,
    DecisionEffect, Evaluator,
};
use serde_json::{json, Value};

const CHECKED_IN_POLICY: &str = include_str!("../../../configs/roc-economics.toml");

#[test]
fn policy_allow_after_backend_context_is_not_paid_unlock_or_receipt_truth() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "allow-paid-content-after-backend-proof-check",
              "when": {
                "method": "GET",
                "require_tags_all": [
                  "backend-proof-checked",
                  "paid-content-policy-context",
                  "content-kind-post"
                ]
              },
              "action": "allow",
              "reason": "declarative paid-content policy gate only",
              "obligations": [
                {
                  "kind": "require-backend-wallet-ledger-proof",
                  "params": {
                    "proof_source": "backend_wallet_ledger_path",
                    "consumer": "gateway_or_omnigate"
                  }
                }
              ]
            }
          ]
        }"#,
    )
    .expect("safe paid-content policy bundle should load");

    let evaluator = Evaluator::new(&bundle).expect("policy should validate");
    let ctx = Context::builder()
        .tenant("beta")
        .method("GET")
        .region("US")
        .tag("backend-proof-checked")
        .tag("paid-content-policy-context")
        .tag("content-kind-post")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative paid-content policy gate only")
    );
    assert_eq!(decision.obligations.items.len(), 1);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-backend-wallet-ledger-proof"
    );

    assert_decision_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn policy_rejects_paid_content_authority_shaped_tags() {
    for tag in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "receipt_proof",
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
        "operation_id",
        "idempotency_key",
        "account_sequence",
        "hold_id",
    ] {
        let err = load_json(&policy_with_required_tags(&[tag]))
            .expect_err("authority-shaped paid-content tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority() {
    for kind in [
        "unlock_paid_content",
        "create_receipt",
        "accept_receipt",
        "verify_payment",
        "mutate_balance",
        "credit_account",
        "open_hold",
        "capture_hold",
        "release_hold",
        "settlement_complete",
        "bridge_settlement",
    ] {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn economics_paid_content_view_prices_and_validates_capture_plan_without_authority() {
    let policy = load_internal_roc_economics_toml(CHECKED_IN_POLICY.as_bytes())
        .expect("checked-in economics policy loads")
        .paid_actions;

    assert!(
        policy
            .action_ids()
            .iter()
            .any(|action_id| action_id == "paid_content_view"),
        "checked-in economics config must include paid_content_view"
    );

    let price = policy
        .price_for("paid_content_view", 1)
        .expect("paid_content_view should produce deterministic price");

    assert!(price > 0, "paid_content_view price must be positive");

    let action = policy
        .enabled_action("paid_content_view")
        .expect("paid_content_view action should be enabled");

    assert_eq!(
        action
            .splits
            .iter()
            .map(|split| u32::from(split.bps))
            .sum::<u32>(),
        10_000,
        "paid_content_view split bps must sum exactly to 10000"
    );

    let mut recipients = BTreeMap::new();
    for split in &action.splits {
        if policy.roles.contains_key(&split.to) {
            recipients.insert(
                split.to.clone(),
                format!("acct_{}", split.to.replace('-', "_")),
            );
        }
    }

    policy
        .validate_capture_plan("paid_content_view", &recipients, price)
        .expect("capture plan should validate only as config/gating input");

    let value = serde_json::to_value(&policy).expect("economics policy JSON");
    assert_economics_json_has_no_authority_keys(&value);
}

#[test]
fn economics_config_rejects_paid_content_authority_poison_fields() {
    for poison in [
        "\nreceipt_truth = true\n",
        "\nbalance_truth = true\n",
        "\npaid_unlock_authority = true\n",
        "\nledger_mutation = true\n",
        "\nwallet_mutation = true\n",
        "\nbridge_txid = \"fake\"\n",
        "\nstaking_position_id = \"fake\"\n",
        "\nliquidity_pool_id = \"fake\"\n",
        "\nexternal_settlement_id = \"fake\"\n",
    ] {
        let bad = format!("{CHECKED_IN_POLICY}{poison}");

        assert!(
            load_internal_roc_economics_toml(bad.as_bytes()).is_err(),
            "economics config must reject authority poison field: {poison:?}"
        );
    }
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "internal-roc-beta-paid-content-policy",
                "when": {
                    "tenant": "*",
                    "method": "GET",
                    "region": "*",
                    "require_tags_all": tags
                },
                "action": "allow",
                "reason": "declarative paid-content classification only"
            }
        ]
    })
    .to_string()
    .into_bytes()
}

fn policy_with_obligation_kind(kind: &str) -> Vec<u8> {
    json!({
        "version": 1,
        "rules": [
            {
                "id": "internal-roc-beta-authority-obligation",
                "when": {
                    "tenant": "*",
                    "method": "GET",
                    "region": "*"
                },
                "action": "allow",
                "obligations": [
                    {
                        "kind": kind,
                        "params": {
                            "source": "client_supplied"
                        }
                    }
                ],
                "reason": "obligation instruction only"
            }
        ]
    })
    .to_string()
    .into_bytes()
}

fn assert_decision_debug_has_no_authority_fields(debug: &str) {
    let lower = debug.to_ascii_lowercase();

    for token in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "receipt_proof",
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
        "mint_authority",
        "operation_id",
        "idempotency_key",
        "account_sequence",
        "hold_id",
        "payout_execution",
    ] {
        assert!(
            !lower.contains(token),
            "policy decision/debug shape must not carry authority token `{token}`:\n{debug}"
        );
    }
}

fn assert_economics_json_has_no_authority_keys(value: &Value) {
    const FORBIDDEN_KEYS: &[&str] = &[
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_execution_truth",
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "balance_minor",
        "wallet_balance",
        "ledger_balance",
        "unlock_granted",
        "finality",
        "finalized",
        "settlement_status",
        "state_root",
        "checkpoint_root",
        "checkpoint_hash",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
    ];

    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !FORBIDDEN_KEYS.iter().any(|forbidden| key == forbidden),
                    "economics config must not expose authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_economics_json_has_no_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_economics_json_has_no_authority_keys(nested);
            }
        }
        _ => {}
    }
}
