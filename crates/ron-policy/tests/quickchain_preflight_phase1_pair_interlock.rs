#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — QC-1A pair-interlock tests for ron-policy.
//! RO:WHY — Keeps declarative policy from becoming index, wallet, ledger, paid-unlock, root, finality, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, scripts/dev-quickchain-preflight.sh, policy loaders/evaluator/source boundary.
//! RO:INVARIANTS — policy decisions/obligations/explanations/economics config are not payment proof, receipt truth, balances, roots, or finality.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — blocks authority creep through policy tags, obligations, config, source shortcuts, or feature flags.
//! RO:TEST — cargo test -p ron-policy --test quickchain_preflight_phase1_pair_interlock.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::json;
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

fn assert_contains_all(haystack: &str, label: &str, phrases: &[&str]) {
    for phrase in phrases {
        assert!(
            haystack.contains(phrase),
            "{label} must contain QC-1A interlock phrase `{phrase}`"
        );
    }
}

fn assert_contains_none(haystack: &str, label: &str, snippets: &[&str]) {
    let normalized = haystack.to_ascii_lowercase();

    for snippet in snippets {
        let needle = snippet.to_ascii_lowercase();
        assert!(
            !normalized.contains(&needle),
            "{label} must not contain forbidden QC-1A authority snippet `{snippet}`"
        );
    }
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
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

fn strip_comment_only_lines(input: &str) -> String {
    let mut out = String::new();

    for line in input.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("//")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("///")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

fn production_source_text() -> String {
    let src_root = crate_root().join("src");
    let mut files = Vec::new();
    collect_rust_files(&src_root, &mut files);
    files.sort();

    let mut combined = String::new();

    for path in files {
        let rel = path
            .strip_prefix(crate_root())
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        let stripped = strip_comment_only_lines(&raw);

        combined.push_str("\n// FILE: ");
        combined.push_str(&rel);
        combined.push('\n');
        combined.push_str(&stripped);
    }

    combined
}

#[test]
fn docs_lock_policy_index_pair_roles_without_authority_transfer() {
    let docs = read_rel("docs/quickchain-preflight.md");

    assert_contains_all(
        &docs,
        "ron-policy quickchain-preflight.md",
        &[
            "ron-policy is declarative policy infrastructure",
            "policy decision is not economic truth",
            "policy allow is not paid proof",
            "policy obligation is not receipt proof",
            "policy explanation is not finality proof",
            "economics policy config is not ledger mutation",
            "feature flag is not settlement authority",
            "Policy must not manufacture paid proof",
            "Policy must not manufacture receipt proof",
            "Policy must not manufacture finality proof",
            "Policy must not manufacture balance proof",
            "root-producing code",
            "checkpoint-producing code",
            "validator code",
            "settlement code",
            "wallet mutation",
            "ledger mutation",
            "paid unlock finality",
        ],
    );
}

#[test]
fn manifest_does_not_add_index_wallet_ledger_or_chain_runtime_dependencies() {
    let cargo = read_rel("Cargo.toml");

    assert_contains_none(
        &cargo,
        "ron-policy Cargo.toml",
        &[
            "svc-index",
            "svc-wallet",
            "ron-ledger",
            "ron-accounting",
            "svc-rewarder",
            "ron_proto::quickchain",
            "quickchain-runtime",
            "quickchain-validator",
            "quickchain-consensus",
            "solana",
            "anchor-lang",
            "spl-token",
            "ethers",
            "web3",
        ],
    );
}

#[test]
fn policy_allow_decision_does_not_unlock_paid_content_without_backend_truth() {
    let bundle = load_json(
        br#"{
          "version": 1,
          "defaults": { "default_action": "deny" },
          "rules": [
            {
              "id": "allow-after-index-and-policy-context",
              "when": {
                "method": "GET",
                "require_tags_all": ["index-pointer-found", "policy-context-checked"]
              },
              "action": "allow",
              "reason": "declarative allow only",
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
    .expect("safe declarative policy must load");

    let evaluator = Evaluator::new(&bundle).expect("safe declarative policy must validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("index-pointer-found")
        .tag("policy-context-checked")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_eq!(decision.reason.as_deref(), Some("declarative allow only"));
    assert_eq!(decision.obligations.items.len(), 1);
    assert_eq!(
        decision.obligations.items[0].kind,
        "require-backend-wallet-ledger-proof"
    );

    assert_debug_has_no_authority_fields(&format!("{decision:?}"));
}

#[test]
fn policy_rejects_authority_shaped_tags_from_index_cache_or_client_context() {
    for tag in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "receipt_proof",
        "balance_minor",
        "wallet_balance",
        "ledger_balance",
        "unlock_granted",
        "paid_proof",
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
        let bundle = policy_with_required_tag(tag);
        let err = load_json(&bundle).expect_err("authority-shaped condition tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag:?} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn policy_allows_ordinary_pair_classification_tags_only() {
    let bundle = load_json(&policy_with_required_tags(&[
        "index-pointer-found",
        "policy-context-checked",
        "asset:image",
        "tenant-beta",
    ]))
    .expect("ordinary classification tags must remain valid");

    let evaluator = Evaluator::new(&bundle).expect("ordinary classification policy must validate");
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("index-pointer-found")
        .tag("policy-context-checked")
        .tag("asset:image")
        .tag("tenant-beta")
        .build(&SystemClock);

    let decision = evaluator.evaluate(&ctx).expect("policy should evaluate");

    assert!(matches!(decision.effect, DecisionEffect::Allow));
    assert_debug_has_no_authority_fields(&format!("{decision:?}"));
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
                "id": "pair-classification",
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

fn assert_debug_has_no_authority_fields(debug: &str) {
    let lower_debug = debug.to_ascii_lowercase();

    for token in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "receipt_proof",
        "balance_minor",
        "wallet_balance",
        "ledger_balance",
        "unlock_granted",
        "paid_proof",
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
    ] {
        assert!(
            !lower_debug.contains(token),
            "policy decision/debug shape must not carry economic authority field token: {token}\n{debug}"
        );
    }
}

#[test]
fn runtime_source_does_not_define_policy_or_index_as_paid_authority() {
    let source = production_source_text();

    assert_contains_none(
        &source,
        "ron-policy runtime source",
        &[
            "svc_index::",
            "svc_wallet::",
            "ron_ledger::",
            "ron_proto::quickchain",
            "quickchain::",
            "unlock_from_policy",
            "unlock_from_index",
            "unlock_from_policy_allow",
            "unlock_from_obligation",
            "unlock_from_feature_flag",
            "unlock_from_economics_config",
            "unlock_from_policy_tag",
            "paid_from_policy",
            "paid_from_index",
            "receipt_from_policy",
            "receipt_from_index",
            "balance_from_policy",
            "balance_from_index",
            "finality_from_policy",
            "finality_from_index",
            "checkpoint_from_policy",
            "checkpoint_from_index",
            "root_from_policy",
            "root_from_index",
            "policy_allow_proves_payment",
            "policy_obligation_proves_receipt",
            "policy_explanation_proves_finality",
            "feature_flag_proves_settlement",
            "economics_config_mutates_ledger",
            "create_receipt(",
            "put_receipt(",
            "insert_receipt(",
            "accept_receipt(",
            "commit_receipt(",
            "mutate_balance(",
            "set_balance(",
            "credit_account(",
            "debit_account(",
            "unlock_paid_content(",
            "produce_root(",
            "produce_checkpoint(",
            "sign_checkpoint(",
            "anchor_checkpoint(",
            "bridge_settlement(",
        ],
    );
}

#[test]
fn schema_keeps_authority_shaped_obligations_forbidden() {
    let schema = read_rel("schema/policybundle.schema.json");
    let normalized_schema = normalize_schema_text(&schema);

    assert_contains_all(
        &normalized_schema,
        "normalized ron-policy policybundle.schema.json",
        &[
            "unlock",
            "paid",
            "proof",
            "receipt",
            "balance",
            "finality",
            "state",
            "root",
            "checkpoint",
            "validator",
            "bridge",
            "operation",
            "idempotency",
            "hold",
        ],
    );
}

fn normalize_schema_text(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

#[test]
fn dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    assert_contains_all(
        &script,
        "ron-policy dev-quickchain-preflight.sh",
        &[
            "find \"$TEST_DIR\"",
            "-name 'quickchain*.rs'",
            "basename \"$test_file\" .rs",
            "test -p \"$PKG\" --test \"$test_name\"",
        ],
    );
}
