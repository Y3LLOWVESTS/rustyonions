#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 1 Round 2 downstream-confirmation tests for ron-policy.
//! RO:WHY — Confirms policy can gate access while never creating QuickChain proof, finality, spend authority, paid proof, wallet truth, or ledger truth.
//! RO:INTERACTS — docs/quickchain-preflight.md, parse::validate, Evaluator, policy obligations.
//! RO:INVARIANTS — policy decisions and obligations remain declarative instructions only.
//! RO:METRICS — none; docs/source/DTO boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents proof/root/finality vocabulary from becoming policy-created authority.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase1_round2_confirmation.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE1_AUTHORITY_TAGS: &[&str] = &[
    "accepted_receipt",
    "epoch_included",
    "epoch_included_receipt",
    "receipt_inclusion_proof",
    "account_root",
    "account_state_proof",
    "state_root",
    "state_proof",
    "hold_root",
    "hold_proof",
    "epoch_root",
    "root_hash",
    "root_proof",
    "proof_hash",
    "merkle_proof",
    "checkpoint_root",
    "checkpoint_hash",
    "checkpoint_proof",
    "validator_signature",
    "validator_proof",
    "validator_set",
    "quorum_signature",
    "anchor_proof",
    "anchored",
    "anchored_receipt",
    "external_anchor",
    "spend_authority",
    "settlement_status",
];

const PHASE1_AUTHORITY_KINDS: &[&str] = &[
    "grant-spend-authority",
    "unlock-paid-content",
    "grant-paid-access",
    "mark-paid-unlocked",
    "prove-payment-finality",
    "verify-finality",
    "mark-finalized",
    "mark-epoch-included",
    "mark-anchored",
    "produce-proof",
    "produce-root-proof",
    "produce-merkle-proof",
    "produce-inclusion-proof",
    "produce-checkpoint-proof",
    "verify-state-proof",
    "verify-account-proof",
    "verify-receipt-proof",
    "verify-inclusion-proof",
    "verify-merkle-proof",
    "write-checkpoint",
    "commit-checkpoint",
    "finalize-checkpoint",
    "set-settlement-status",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = crate_dir().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//")
                || trimmed.starts_with("//!")
                || trimmed.starts_with("///")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rs_sources(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            collect_rs_sources(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn production_source_text() -> String {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_sources(&src_root, &mut files);
    files.sort();

    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context} must preserve required marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, context: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{context} must not contain forbidden authority marker: {forbidden}"
    );
}

#[test]
fn docs_name_phase1_round2_policy_downstream_confirmation_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 1 round 2 downstream confirmation",
        "policy can gate access but not create proof/finality",
        "policy cannot turn a proof into spend authority",
        "policy decision is not quickchain proof",
        "policy allow is not epoch_included",
        "policy allow is not finalized",
        "policy allow is not anchored",
        "policy obligation is not receipt inclusion proof",
        "policy obligation is not account proof",
        "policy obligation is not spend authority",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase1_round2_confirmation",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn policy_can_gate_on_ordinary_artifact_context_without_creating_authority() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "gate-after-index-and-policy-context",
              "when": {
                "method": "GET",
                "require_tags_all": [
                  "index-pointer-found",
                  "policy-context-checked",
                  "proof-artifact-reference-present"
                ]
              },
              "action": "allow",
              "reason": "policy gate only",
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
    .expect("ordinary artifact-reference context must remain declarative and valid");

    let evaluator = Evaluator::new(&bundle).expect("policy should validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("index-pointer-found")
        .tag("policy-context-checked")
        .tag("proof-artifact-reference-present")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_eq!(decision.reason.as_deref(), Some("policy gate only"));
    assert_eq!(decision.obligations.items.len(), 1);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-backend-wallet-ledger-proof"
    );

    assert_debug_has_no_phase1_authority_fields(&format!("{decision:?}"));
}

#[test]
fn phase1_root_proof_and_finality_condition_tags_reject() {
    for tag in PHASE1_AUTHORITY_TAGS {
        let bundle = policy_with_tags(vec![tag]);

        let err = load_json(&bundle).expect_err("authority-shaped condition tag must reject");

        assert!(
            err.to_string().contains("looks like economic authority"),
            "tag {tag:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase1_root_proof_and_finality_tags_reject_across_separator_styles() {
    for tag in [
        "receipt/inclusion/proof",
        "account.state.proof",
        "state-root",
        "root hash",
        "checkpoint/proof",
        "validator.set",
        "anchor proof",
        "spend authority",
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
fn phase1_root_proof_and_finality_obligation_param_keys_reject() {
    for key in PHASE1_AUTHORITY_TAGS {
        let bundle = policy_with_obligation_param(key);

        let err =
            load_json(&bundle).expect_err("authority-shaped obligation param key must reject");

        assert!(
            err.to_string().contains("looks like economic authority"),
            "param key {key:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase1_authority_shaped_obligation_kinds_reject() {
    for kind in PHASE1_AUTHORITY_KINDS {
        let bundle = policy_with_obligation_kind(kind);

        let err = load_json(&bundle).expect_err("authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("looks like economic authority"),
            "obligation kind {kind:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn production_source_remains_declarative_no_root_proof_or_spend_authority() {
    let source = strip_line_comments(&production_source_text()).to_ascii_lowercase();

    for forbidden in [
        "svc_wallet::",
        "ron_ledger::",
        "ron_proto::quickchain",
        "quickchain::",
        "produce_root(",
        "produce_proof(",
        "verify_finality(",
        "mark_finalized(",
        "mark_epoch_included(",
        "mark_anchored(",
        "grant_spend_authority(",
        "grant_paid_access(",
        "unlock_paid_content(",
        "set_settlement_status(",
        "commit_checkpoint(",
        "anchor_checkpoint(",
        "bridge_settlement(",
    ] {
        assert_not_contains(&source, forbidden, "ron-policy production source");
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

fn policy_with_obligation_kind(kind: &str) -> Vec<u8> {
    json!({
        "version": 1,
        "rules": [
            {
                "id": "obligation-kind",
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
                            "source": "forbidden"
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

fn assert_debug_has_no_phase1_authority_fields(debug: &str) {
    let normalized_debug = debug.to_ascii_lowercase();

    for token in [
        "accepted_receipt",
        "epoch_included",
        "receipt_inclusion_proof",
        "account_root",
        "account_state_proof",
        "state_root",
        "state_proof",
        "hold_root",
        "hold_proof",
        "epoch_root",
        "root_hash",
        "root_proof",
        "proof_hash",
        "merkle_proof",
        "checkpoint_root",
        "checkpoint_hash",
        "checkpoint_proof",
        "validator_signature",
        "validator_set",
        "quorum_signature",
        "anchor_proof",
        "anchored_receipt",
        "external_anchor",
        "spend_authority",
        "settlement_status",
    ] {
        assert!(
            !normalized_debug.contains(token),
            "policy decision/debug shape must not carry Phase 1 authority field token: {token}\n{debug}"
        );
    }
}
