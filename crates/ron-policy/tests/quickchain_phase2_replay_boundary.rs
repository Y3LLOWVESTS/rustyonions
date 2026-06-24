#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 2 Round 1 read-only verifier access policy boundary tests for ron-policy.
//! RO:WHY — ron-policy may declare verifier artifact access policy but must not create verifier truth, finality, settlement, or paid unlock authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, load_json, Evaluator, obligations.
//! RO:INVARIANTS — policy decisions/obligations are declarative instructions only.
//! RO:METRICS — none; docs/source/decision boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents replay/proof/verifier policy context from becoming committee/quorum/finality authority.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase2_replay_boundary.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| panic!("read {}: {err}", full.display()))
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn production_source_text() -> String {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    let mut out = String::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        out.push_str(&strip_line_comments(&source));
        out.push('\n');
    }
    out.to_ascii_lowercase()
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, label: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{label} must not contain forbidden Phase 2 authority marker: {forbidden}"
    );
}

fn policy_with_required_tag(tag: &str) -> Vec<u8> {
    policy_with_required_tags(&[tag])
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase2-classification",
                "when": {
                    "method": "GET",
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
                "id": "phase2-param",
                "when": { "method": "GET" },
                "action": "allow",
                "obligations": [
                    {
                        "kind": "require-backend-replay-artifact-reference",
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

fn assert_debug_has_no_phase2_authority_fields(debug: &str) {
    let rendered = debug.to_ascii_lowercase();

    for forbidden in [
        "replay_result",
        "replay_root",
        "verifier_result",
        "verifier_attestation",
        "verifier_signature",
        "committee_attestation",
        "committee_signature",
        "committee_vote",
        "quorum_reached",
        "fork_choice",
        "fork_choice_winner",
        "finalized",
        "settlement_status",
        "settlement_finality",
        "checkpoint_hash",
        "validator_signature",
        "bridge_proof",
        "unlock_granted",
        "paid_unlock",
    ] {
        assert_not_contains(&rendered, forbidden, "ron-policy decision/debug shape");
    }
}

#[test]
fn docs_name_phase2_round1_policy_verifier_access_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 1 verifier artifact / read-only replication",
        "ron-policy may declare verifier access policy only",
        "policy-created verifier truth is forbidden",
        "policy-created replay truth is forbidden",
        "policy-created quorum truth is forbidden",
        "policy-created committee truth is forbidden",
        "policy-created fork choice is forbidden",
        "policy-created finality is forbidden",
        "policy allow is not verifier proof",
        "policy obligation is not verifier attestation",
        "policy obligation is not quorum",
        "policy obligation cannot unlock paid content from replay artifacts alone",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase2_replay_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn policy_can_declare_read_only_replay_artifact_access_without_creating_truth() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "allow-read-only-replay-artifact-view",
              "when": {
                "method": "GET",
                "require_tags_all": [
                  "artifact-pointer-found",
                  "read-only-replay-artifact",
                  "verifier-access-policy-checked"
                ]
              },
              "action": "allow",
              "reason": "declarative verifier access policy only",
              "obligations": [
                {
                  "kind": "require-backend-replay-artifact-reference",
                  "params": {
                    "artifact_source": "backend_replay_bundle_store",
                    "consumer": "gateway_or_omnigate"
                  }
                },
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
    .expect("read-only replay artifact policy must remain declarative and valid");

    let evaluator = Evaluator::new(&bundle).expect("policy should validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("artifact-pointer-found")
        .tag("read-only-replay-artifact")
        .tag("verifier-access-policy-checked")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative verifier access policy only")
    );
    assert_eq!(decision.obligations.items.len(), 2);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-backend-replay-artifact-reference"
    );
    assert_eq!(
        decision.obligations.items[1].kind,
        "require-backend-wallet-ledger-proof"
    );

    assert_debug_has_no_phase2_authority_fields(&format!("{decision:?}"));
}

#[test]
fn ordinary_phase2_classification_tags_are_allowed_but_existing_authority_tags_still_reject() {
    load_json(&policy_with_required_tags(&[
        "artifact-pointer-found",
        "read-only-replay-artifact",
        "verifier-access-policy-checked",
        "tenant-beta",
    ]))
    .expect("ordinary Phase 2 classification tags should remain allowed");

    for tag in [
        "receipt_hash",
        "receipt_proof",
        "paid_proof",
        "unlock_granted",
        "finality",
        "finalized",
        "settlement_status",
        "state_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_proof",
    ] {
        let err =
            load_json(&policy_with_required_tag(tag)).expect_err("authority-shaped tag rejects");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn replay_artifact_policy_params_cannot_smuggle_existing_authority_fields() {
    for param_key in [
        "receipt_hash",
        "receipt_proof",
        "paid_proof",
        "unlock_granted",
        "finality",
        "finalized",
        "settlement_status",
        "state_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_proof",
    ] {
        let err = load_json(&policy_with_obligation_param(param_key))
            .expect_err("authority-shaped obligation param key rejects");

        assert!(
            err.to_string().contains("economic authority"),
            "param key {param_key:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn production_source_does_not_construct_phase2_committee_quorum_or_finality_authority() {
    let source = production_source_text();

    for forbidden in [
        "svc_wallet::",
        "ron_ledger::",
        "ron_proto::quickchain",
        "quickchain::",
        "replay_result:",
        "\"replay_result\"",
        "verifier_result:",
        "\"verifier_result\"",
        "verifier_attestation:",
        "\"verifier_attestation\"",
        "committee_attestation:",
        "\"committee_attestation\"",
        "committee_vote:",
        "\"committee_vote\"",
        "quorum_reached:",
        "\"quorum_reached\"",
        "fork_choice_winner:",
        "\"fork_choice_winner\"",
        "settlement_finality:",
        "\"settlement_finality\"",
        "policy_decision_proves_verifier_truth",
        "policy_decision_proves_finality",
        "policy_obligation_is_attestation",
        "policy_obligation_is_quorum",
        "policy_allow_unlocks_replay_artifact",
        "sign_attestation(",
        "decide_quorum(",
        "decide_fork_choice(",
        "mark_finalized(",
        "set_settlement_status(",
        "commit_checkpoint(",
        "anchor_checkpoint(",
        "bridge_settlement(",
    ] {
        assert_not_contains(&source, forbidden, "ron-policy production source");
    }
}
