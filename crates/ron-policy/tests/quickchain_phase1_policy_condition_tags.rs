//! RO:WHAT — Phase 1 policy condition-tag non-authority tests for ron-policy QuickChain boundaries.
//! RO:WHY — Prevent caller-provided tags from becoming fake paid proof, receipt truth, balance truth, finality, roots, checkpoints, or validator authority.
//! RO:INTERACTS — `ron_policy::load_json`, `parse::validate`, policy schema.
//! RO:INVARIANTS — policy tags may classify requests, but cannot prove payment, finality, receipt inclusion, root truth, or ledger state.
//! RO:TEST — `cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags`.

use ron_policy::load_json;
use serde_json::{json, Value};

const FORBIDDEN_AUTHORITY_TAGS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
    "account_proof",
    "inclusion_proof",
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
];

const FORBIDDEN_OBLIGATION_PARAM_KEYS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
    "account_proof",
    "inclusion_proof",
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
];

#[test]
fn ordinary_classification_tags_remain_allowed() {
    let bundle = policy_with_tags(vec![
        "asset:image",
        "paid-storage-request",
        "tenant-beta",
        "creator-content",
    ]);

    load_json(&bundle).expect("ordinary classification tags must remain valid");
}

#[test]
fn authority_shaped_condition_tags_reject() {
    for tag in FORBIDDEN_AUTHORITY_TAGS {
        let bundle = policy_with_tags(vec![tag]);

        let err = load_json(&bundle).expect_err("authority-shaped condition tag must reject");

        assert!(
            err.to_string().contains("looks like economic authority"),
            "tag {tag:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn authority_shaped_condition_tags_reject_across_separator_styles() {
    for tag in [
        "paid-proof",
        "receipt.hash",
        "checkpoint/root",
        "validator signature",
    ] {
        let bundle = policy_with_tags(vec![tag]);

        let err = load_json(&bundle).expect_err("authority-shaped condition tag must reject");

        assert!(
            err.to_string().contains("looks like economic authority"),
            "tag {tag:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn authority_shaped_obligation_param_keys_reject() {
    for key in FORBIDDEN_OBLIGATION_PARAM_KEYS {
        let bundle = policy_with_obligation_param(key);

        let err =
            load_json(&bundle).expect_err("authority-shaped obligation param key must reject");

        assert!(
            err.to_string().contains("looks like economic authority"),
            "param key {key:?} should reject as economic authority, got: {err}"
        );
    }
}

fn policy_with_tags(tags: Vec<&str>) -> Vec<u8> {
    json!({
        "version": 1,
        "rules": [
            {
                "id": "tag-classification",
                "when": {
                    "tenant": "*",
                    "method": "GET",
                    "region": "*",
                    "require_tags_all": tags
                },
                "action": "allow",
                "reason": "classification only"
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
                "id": "obligation-param",
                "when": {
                    "tenant": "*",
                    "method": "GET",
                    "region": "*"
                },
                "action": "allow",
                "obligations": [
                    {
                        "kind": "add-header",
                        "params": params
                    }
                ],
                "reason": "obligation instruction only"
            }
        ]
    })
    .to_string()
    .into_bytes()
}
