#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 3 selected external-posture policy boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative visibility/readiness rules for selected external posture,
//! but decisions/obligations/config are never proof, payment, finality, settlement,
//! bridge, public-market, exchange, ROX/Solana, or outside-program authority.
//! RO:INVARIANTS — policy gates only; no policy-created external posture truth or paid unlock.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase5_external_posture_policy_boundary.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE5_R3_EXTERNAL_POSTURE_AUTHORITY_TAGS: &[&str] = &[
    "external_posture_truth",
    "external-posture-authority",
    "external.posture.proof.truth",
    "external/posture/finality",
    "external_posture_settlement",
    "external-posture-paid-unlock",
    "external_posture_reward_truth",
    "external_posture_balance_truth",
    "external_posture_receipt_truth",
    "external_posture_root_authority",
    "external_posture_pruning_authority",
    "selected_external_posture_truth",
    "selected-external-posture-authority",
    "outside_program_truth",
    "outside-program-authority",
    "outside.program.execution.truth",
    "public_chain_truth",
    "public-chain-authority",
    "public_chain_settlement_truth",
    "public_market_truth",
    "public-market-authority",
    "exchange_facing_authority",
    "exchange-settlement-truth",
    "rox_truth",
    "rox-authority",
    "rox_runtime_authority",
    "rox-settlement-truth",
    "solana_truth",
    "solana-authority",
    "solana_runtime_authority",
    "solana-settlement-truth",
];

const PHASE5_R3_ALLOWED_DISPLAY_TAGS: &[&str] = &[
    "external-posture-display",
    "external-posture-evidence-display",
    "external-posture-status-display",
    "outside-program-display",
    "public-chain-reference-display",
    "public-market-reference-display",
    "rox-reference-display",
    "solana-reference-display",
    "backend-derived-external-posture-status",
];

const PHASE5_R3_EXTERNAL_POSTURE_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "grant-external-posture-authority",
    "accept-external-posture-truth",
    "accept-external-posture-proof-as-payment",
    "unlock-from-external-posture",
    "settle-from-external-posture",
    "finalize-from-external-posture",
    "mutate-from-external-posture",
    "grant-outside-program-authority",
    "execute-outside-program",
    "accept-outside-program-truth",
    "grant-public-chain-authority",
    "settle-from-public-chain",
    "grant-public-market-authority",
    "grant-exchange-facing-authority",
    "settle-from-exchange",
    "grant-rox-runtime-authority",
    "settle-from-rox",
    "grant-solana-runtime-authority",
    "settle-from-solana",
];

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

fn compact_without_comments(text: &str) -> String {
    strip_line_comments(text)
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        if root.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(root.to_path_buf());
        }
        return;
    }

    if !root.exists() {
        return;
    }

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

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 5 Round 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 5 Round 3 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": 1,
        "defaults": {
            "default_action": "deny",
            "max_body_bytes": 1048576
        },
        "rules": [{
            "id": "phase5-r3-required-tags",
            "when": {
                "method": "GET",
                "require_tags_all": tags
            },
            "action": "allow",
            "reason": "display-only external posture policy"
        }]
    }))
    .expect("policy json should serialize")
}

fn policy_with_obligation_param(param_key: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": 1,
        "defaults": {
            "default_action": "deny",
            "max_body_bytes": 1048576
        },
        "rules": [{
            "id": "phase5-r3-obligation-param",
            "when": { "method": "GET" },
            "action": "allow",
            "obligations": [{
                "kind": "display_external_posture_status",
                "params": {
                    param_key: "true"
                }
            }],
            "reason": "display-only external posture policy"
        }]
    }))
    .expect("policy json should serialize")
}

fn policy_with_obligation_kind(kind: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": 1,
        "defaults": {
            "default_action": "deny",
            "max_body_bytes": 1048576
        },
        "rules": [{
            "id": "phase5-r3-obligation-kind",
            "when": { "method": "GET" },
            "action": "allow",
            "obligations": [{
                "kind": kind,
                "params": {
                    "mode": "display_only"
                }
            }],
            "reason": "display-only external posture policy"
        }]
    }))
    .expect("policy json should serialize")
}

#[test]
fn docs_name_phase5_round3_policy_external_posture_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 3 selected external posture policy boundary",
        "ron-policy may express declarative visibility/readiness policy for selected external posture evidence",
        "policy allow is not external posture proof",
        "policy allow is not outside-program execution proof",
        "policy allow is not public-chain settlement proof",
        "policy allow is not rox runtime proof",
        "policy allow is not solana runtime proof",
        "policy obligation is not external settlement authority",
        "policy decision cannot mutate wallet or ledger from selected external posture evidence",
        "policy decision cannot unlock paid content from selected external posture evidence",
        "quickchain_phase5_external_posture_policy_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn declarative_external_posture_display_policy_is_allowed_but_not_authority() {
    let bundle = load_json(&policy_with_required_tags(PHASE5_R3_ALLOWED_DISPLAY_TAGS))
        .expect("display-only external posture policy tags should parse");

    let evaluator = Evaluator::new(&bundle).expect("display policy should validate");
    let mut builder = Context::builder()
        .tenant("creator-site")
        .method("GET")
        .region("US");

    for tag in PHASE5_R3_ALLOWED_DISPLAY_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("display-only decision should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("display-only external posture policy")
    );
}

#[test]
fn phase5_external_posture_authority_condition_tags_reject() {
    for tag in PHASE5_R3_EXTERNAL_POSTURE_AUTHORITY_TAGS {
        let err = load_json(&policy_with_required_tags(&[*tag]))
            .expect_err("Phase 5 external posture authority-shaped tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase5_external_posture_authority_obligation_params_reject() {
    for key in PHASE5_R3_EXTERNAL_POSTURE_AUTHORITY_TAGS {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("Phase 5 external posture authority-shaped obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase5_external_posture_authority_obligation_kinds_reject() {
    for kind in PHASE5_R3_EXTERNAL_POSTURE_AUTHORITY_OBLIGATION_KINDS {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("Phase 5 external posture authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase5_external_posture_authority_shapes() {
    let parser = normalized(&read_rel("src/parse/validate.rs"));
    let economics = normalized(&read_rel("src/economics/validate.rs"));

    for required in [
        "\"externalposturetruth\"",
        "\"externalpostureauthority\"",
        "\"externalpostureprooftruth\"",
        "\"externalposturefinality\"",
        "\"externalposturesettlement\"",
        "\"externalposturepaidunlock\"",
        "\"externalposturerewardtruth\"",
        "\"externalposturebalancetruth\"",
        "\"externalposturereceipttruth\"",
        "\"externalposturerootauthority\"",
        "\"externalposturepruningauthority\"",
        "\"selectedexternalposturetruth\"",
        "\"selectedexternalpostureauthority\"",
        "\"outsideprogramtruth\"",
        "\"outsideprogramauthority\"",
        "\"outsideprogramexecutiontruth\"",
        "\"publicchaintruth\"",
        "\"publicchainauthority\"",
        "\"publicchainsettlementtruth\"",
        "\"publicmarkettruth\"",
        "\"publicmarketauthority\"",
        "\"exchangefacingauthority\"",
        "\"exchangesettlementtruth\"",
        "\"roxtruth\"",
        "\"roxauthority\"",
        "\"roxruntimeauthority\"",
        "\"roxsettlementtruth\"",
        "\"solanatruth\"",
        "\"solanaauthority\"",
        "\"solanaruntimeauthority\"",
        "\"solanasettlementtruth\"",
    ] {
        assert_contains(&parser, required, "ron-policy parser validation table");
        assert_contains(
            &economics,
            required,
            "ron-policy economics validation table",
        );
    }

    for required in [
        "\"grantexternalpostureauthority\"",
        "\"acceptexternalposturetruth\"",
        "\"acceptexternalpostureproofaspayment\"",
        "\"unlockfromexternalposture\"",
        "\"settlefromexternalposture\"",
        "\"finalizefromexternalposture\"",
        "\"mutatefromexternalposture\"",
        "\"grantoutsideprogramauthority\"",
        "\"executeoutsideprogram\"",
        "\"acceptoutsideprogramtruth\"",
        "\"grantpublicchainauthority\"",
        "\"settlefrompublicchain\"",
        "\"grantpublicmarketauthority\"",
        "\"grantexchangefacingauthority\"",
        "\"settlefromexchange\"",
        "\"grantroxruntimeauthority\"",
        "\"settlefromrox\"",
        "\"grantsolanaruntimeauthority\"",
        "\"settlefromsolana\"",
    ] {
        assert_contains(
            &parser,
            required,
            "ron-policy parser forbidden obligation kind table",
        );
    }
}

#[test]
fn production_source_does_not_construct_phase5_external_posture_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "external_posture_truth:true",
        "\"external_posture_truth\":true",
        "external_posture_authority:true",
        "\"external_posture_authority\":true",
        "external_posture_paid_unlock:true",
        "\"external_posture_paid_unlock\":true",
        "external_posture_balance_truth:true",
        "\"external_posture_balance_truth\":true",
        "external_posture_receipt_truth:true",
        "\"external_posture_receipt_truth\":true",
        "external_posture_reward_truth:true",
        "\"external_posture_reward_truth\":true",
        "external_posture_finality:true",
        "\"external_posture_finality\":true",
        "external_posture_settlement:true",
        "\"external_posture_settlement\":true",
        "external_posture_root_authority:true",
        "\"external_posture_root_authority\":true",
        "external_posture_pruning_authority:true",
        "\"external_posture_pruning_authority\":true",
        "policy_external_posture_truth:true",
        "\"policy_external_posture_truth\":true",
        "policy_external_posture_finality:true",
        "\"policy_external_posture_finality\":true",
        "policy_external_posture_settlement:true",
        "\"policy_external_posture_settlement\":true",
        "outside_program_authority:true",
        "\"outside_program_authority\":true",
        "public_market_authority:true",
        "\"public_market_authority\":true",
        "exchange_facing_authority:true",
        "\"exchange_facing_authority\":true",
        "rox_runtime_authority:true",
        "\"rox_runtime_authority\":true",
        "solana_runtime_authority:true",
        "\"solana_runtime_authority\":true",
        "grant_external_posture_authority(",
        "commit_external_posture_truth(",
        "accept_external_posture_proof(",
        "accept_external_posture_evidence(",
        "unlock_from_external_posture(",
        "settle_from_external_posture(",
        "finalize_from_external_posture(",
        "mutate_from_external_posture(",
        "grant_outside_program_authority(",
        "execute_outside_program(",
        "grant_public_chain_authority(",
        "grant_public_market_authority(",
        "grant_exchange_facing_authority(",
        "grant_rox_runtime_authority(",
        "grant_solana_runtime_authority(",
        "solana_settlement(",
        "rox_settlement(",
        "bridge_settlement(",
        "external_settlement(",
        "svc_wallet::",
        "ron_ledger::",
        "solana_sdk",
        "solana_client",
        "anchor_lang",
        "spl_token",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("ron-policy source {}", path.display()),
            );
        }
    }
}

#[test]
fn policy_manifest_does_not_add_phase5_external_posture_runtime_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "ron-accounting",
        "svc-rewarder",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
        "alloy",
        "solana-sdk",
        "solana-client",
    ] {
        assert_not_contains(&cargo, forbidden, "ron-policy Cargo.toml");
    }
}

#[test]
fn preflight_runner_dynamically_discovers_phase5_external_posture_policy_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "ron-policy dev-quickchain-preflight.sh");
    }
}
