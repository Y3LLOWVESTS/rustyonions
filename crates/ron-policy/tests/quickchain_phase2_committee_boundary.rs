#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

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
        "{label} must contain required Phase 2 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, label: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{label} must not contain forbidden Phase 2 Round 2 authority marker: {forbidden}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase2-r2-committee-readiness-display-policy",
                "when": {
                    "method": "GET",
                    "require_tags_all": tags
                },
                "action": "allow",
                "reason": "declarative committee readiness display policy only",
                "obligations": [
                    {
                        "kind": "require-backend-verifier-status-reference",
                        "params": {
                            "source": "backend_replay_verifier",
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
                "id": "phase2-r2-param",
                "when": { "method": "GET" },
                "action": "allow",
                "obligations": [
                    {
                        "kind": "require-backend-verifier-status-reference",
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

fn assert_debug_has_no_phase2_r2_authority_fields(debug: &str) {
    let rendered = debug.to_ascii_lowercase();

    for forbidden in [
        "verifier_attestation",
        "verifier_signature",
        "committee_attestation",
        "committee_signature",
        "committee_vote",
        "quorum_reached",
        "quorum_certificate",
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
fn docs_name_phase2_round2_policy_committee_readiness_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 2 committee readiness boundary",
        "ron-policy may declare verifier access policy only",
        "ron-policy may declare committee-readiness display policy only",
        "ron-policy may declare quorum-readiness display policy only",
        "policy-created verifier truth is forbidden",
        "policy-created committee truth is forbidden",
        "policy-created quorum truth is forbidden",
        "policy-created fork choice is forbidden",
        "policy-created finality is forbidden",
        "policy-created settlement is forbidden",
        "policy allow is not verifier proof",
        "policy allow is not committee attestation",
        "policy allow is not quorum",
        "policy obligation is not verifier attestation",
        "policy obligation is not committee attestation",
        "policy obligation is not quorum certificate",
        "policy decision is not signed verification attestation",
        "policy config is not validator membership",
        "ron-policy does not produce signed verification attestations",
        "ron-policy does not decide committee membership",
        "ron-policy does not decide quorum",
        "ron-policy does not decide fork choice",
        "ron-policy does not decide finality",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase2_committee_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn policy_can_gate_committee_readiness_display_without_creating_truth() {
    let bundle = load_json(&policy_with_required_tags(&[
        "verifier-artifact-reference-present",
        "committee-readiness-reference-present",
        "policy-context-checked",
    ]))
    .expect("ordinary committee-readiness display policy must remain declarative and valid");

    let evaluator = Evaluator::new(&bundle).expect("policy should validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("verifier-artifact-reference-present")
        .tag("committee-readiness-reference-present")
        .tag("policy-context-checked")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative committee readiness display policy only")
    );
    assert_eq!(decision.obligations.items.len(), 2);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-backend-verifier-status-reference"
    );
    assert_eq!(
        decision.obligations.items[1].kind,
        "require-backend-wallet-ledger-proof"
    );

    assert_debug_has_no_phase2_r2_authority_fields(&format!("{decision:?}"));
}

#[test]
fn committee_readiness_policy_params_cannot_smuggle_existing_authority_fields() {
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
        "operation_id",
        "idempotency_key",
        "account_sequence",
        "hold_id",
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
fn production_source_does_not_construct_committee_quorum_or_finality_authority() {
    let source = production_source_text();

    for forbidden in [
        "svc_wallet::",
        "ron_ledger::",
        "ron_proto::quickchain",
        "quickchain::",
        "verifier_attestation:",
        "\"verifier_attestation\"",
        "verifier_signature:",
        "\"verifier_signature\"",
        "committee_attestation:",
        "\"committee_attestation\"",
        "committee_signature:",
        "\"committee_signature\"",
        "committee_member:",
        "\"committee_member\"",
        "committee_vote:",
        "\"committee_vote\"",
        "quorum_reached:",
        "\"quorum_reached\"",
        "quorum_certificate:",
        "\"quorum_certificate\"",
        "fork_choice:",
        "\"fork_choice\"",
        "fork_choice_winner:",
        "\"fork_choice_winner\"",
        "settlement_finality:",
        "\"settlement_finality\"",
        "policy_decision_proves_committee_truth",
        "policy_decision_proves_quorum_truth",
        "policy_decision_proves_finality",
        "policy_obligation_is_attestation",
        "policy_obligation_is_quorum",
        "policy_config_is_validator_membership",
        "policy_allow_unlocks_committee_artifact",
        "produce_attestation(",
        "sign_attestation(",
        "decide_committee(",
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

#[test]
fn manifest_does_not_add_index_wallet_ledger_validator_or_external_settlement_dependencies() {
    let cargo = read_rel("Cargo.toml").to_ascii_lowercase();

    for forbidden in [
        "svc-index",
        "svc-wallet",
        "ron-ledger",
        "ron-accounting",
        "svc-rewarder",
        "ron-proto",
        "quickchain-runtime",
        "quickchain-validator",
        "quickchain-consensus",
        "tendermint",
        "cometbft",
        "solana-client",
        "solana-sdk",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
    ] {
        assert_not_contains(&cargo, forbidden, "ron-policy Cargo.toml");
    }
}

#[test]
fn dynamic_preflight_will_pick_up_the_phase2_committee_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "basename \"$test_file\" .rs",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "ron-policy dev-quickchain-preflight.sh");
    }
}
