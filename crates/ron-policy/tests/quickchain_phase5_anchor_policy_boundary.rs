#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 1 anchor-policy boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative anchor display/acceptance gates,
//! but policy decisions/obligations/config are never anchor proof, wallet, ledger, paid-unlock,
//! finality, settlement, bridge, external-chain, staking, liquidity, or exchange authority.
//! RO:INTERACTS — parse::validate, economics::validate, docs/quickchain-preflight.md.
//! RO:INVARIANTS — policy gates only; no policy-created anchor truth, settlement truth, balance truth, or paid unlock.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase5_anchor_policy_boundary.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE5_ANCHOR_AUTHORITY_TAGS: &[&str] = &[
    "anchor_truth",
    "anchor-authority",
    "anchor.finality",
    "anchor/settlement",
    "anchor_payment_truth",
    "anchor-paid-unlock",
    "anchor.proof.truth",
    "anchor/external/truth",
    "external_anchor_truth",
    "external-anchor-authority",
    "external_chain_roc_truth",
    "external-chain-balance",
    "external_settlement_truth",
    "anchor_checkpoint_truth",
    "checkpoint_commitment_truth",
    "anchored_balance",
    "anchored_receipt",
    "anchored_finality",
    "anchored_settlement",
    "anchor_verification_authority",
    "dry_run_settlement",
    "dry_run_finality",
];

const PHASE5_ALLOWED_DISPLAY_TAGS: &[&str] = &[
    "anchor-dry-run-display",
    "anchor-evidence-display",
    "checkpoint-commitment-display",
    "anchor-verification-report-display",
    "backend-derived-anchor-status",
    "external-anchor-non-authority",
];

const PHASE5_ANCHOR_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "grant-anchor-authority",
    "commit-anchor-truth",
    "accept-anchor-proof",
    "accept-anchor-evidence",
    "unlock-from-anchor",
    "settle-from-anchor",
    "finalize-from-anchor",
    "mutate-from-anchor",
    "bridge-from-anchor",
    "settle-external-anchor",
    "mark-anchored-final",
    "grant-external-settlement-authority",
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
        "{label} must contain required Phase 5 Round 1 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 5 Round 1 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase5-anchor-display-policy",
                "when": {
                    "tenant": "*",
                    "method": "GET",
                    "region": "*",
                    "require_tags_all": tags
                },
                "action": "allow",
                "obligations": [
                    {
                        "kind": "require-backend-wallet-ledger-proof",
                        "params": {
                            "source": "backend"
                        }
                    }
                ],
                "reason": "declarative anchor dry-run display policy only"
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
                "id": "phase5-anchor-obligation-param",
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
                "id": "phase5-anchor-obligation-kind",
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

#[test]
fn docs_name_phase5_round1_policy_anchor_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 1 anchor policy boundary",
        "ron-policy may express declarative anchor display or acceptance policy only",
        "policy decisions do not mutate wallet or ledger from anchor evidence",
        "policy allow is not anchor proof",
        "policy obligation is not external settlement evidence acceptance",
        "policy decision cannot unlock paid content from anchor evidence",
        "policy decision cannot become balance truth, receipt truth, finality truth, or settlement truth",
        "outside chains cannot become roc truth through ron-policy",
        "quickchain_phase5_anchor_policy_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_anchor_display_policy_remains_allowed_but_non_authoritative() {
    let policy = load_json(&policy_with_required_tags(PHASE5_ALLOWED_DISPLAY_TAGS))
        .expect("ordinary anchor dry-run display/gating tags should stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should validate");

    let mut builder = Context::builder().tenant("t").method("GET").region("US");
    for tag in PHASE5_ALLOWED_DISPLAY_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative anchor dry-run display policy only")
    );

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "anchor_authority",
        "anchor_truth",
        "anchor_proof_truth",
        "anchor_settlement_truth",
        "anchor_finality_truth",
        "anchor_payment_truth",
        "external_chain_roc_truth",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
        "unlock_granted",
        "bridge_settlement",
    ] {
        assert_not_contains(&debug, forbidden, "ron-policy decision/debug shape");
    }
}

#[test]
fn phase5_anchor_authority_condition_tags_reject() {
    for tag in PHASE5_ANCHOR_AUTHORITY_TAGS {
        let err = load_json(&policy_with_required_tags(&[*tag]))
            .expect_err("Phase 5 anchor authority-shaped tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase5_anchor_authority_obligation_params_reject() {
    for key in PHASE5_ANCHOR_AUTHORITY_TAGS {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("Phase 5 anchor authority-shaped obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase5_anchor_authority_obligation_kinds_reject() {
    for kind in PHASE5_ANCHOR_AUTHORITY_OBLIGATION_KINDS {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("Phase 5 anchor authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase5_anchor_authority_shapes() {
    let parser = normalized(&read_rel("src/parse/validate.rs"));
    let economics = normalized(&read_rel("src/economics/validate.rs"));

    for required in [
        "\"anchortruth\"",
        "\"anchorauthority\"",
        "\"anchorfinality\"",
        "\"anchorsettlement\"",
        "\"anchorpaymenttruth\"",
        "\"anchorpaidunlock\"",
        "\"anchorprooftruth\"",
        "\"externalchainroctruth\"",
        "\"externalsettlementtruth\"",
        "\"anchorcheckpointtruth\"",
        "\"checkpointcommitmenttruth\"",
        "\"anchoredbalance\"",
        "\"anchoredreceipt\"",
        "\"anchoredfinality\"",
        "\"anchoredsettlement\"",
        "\"anchorverificationauthority\"",
        "\"dryrunsettlement\"",
        "\"dryrunfinality\"",
    ] {
        assert_contains(&parser, required, "ron-policy parser validation table");
        assert_contains(
            &economics,
            required,
            "ron-policy economics validation table",
        );
    }

    for required in [
        "\"grantanchorauthority\"",
        "\"commitanchortruth\"",
        "\"acceptanchorproof\"",
        "\"acceptanchorevidence\"",
        "\"unlockfromanchor\"",
        "\"settlefromanchor\"",
        "\"finalizefromanchor\"",
        "\"mutatefromanchor\"",
        "\"bridgefromanchor\"",
        "\"settleexternalanchor\"",
        "\"markanchoredfinal\"",
        "\"grantexternalsettlementauthority\"",
    ] {
        assert_contains(
            &parser,
            required,
            "ron-policy parser forbidden obligation kind table",
        );
    }
}

#[test]
fn production_source_does_not_construct_phase5_anchor_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "anchor_authority:true",
        "\"anchor_authority\":true",
        "anchor_truth:true",
        "\"anchor_truth\":true",
        "anchor_finality_truth:true",
        "\"anchor_finality_truth\":true",
        "anchor_settlement_truth:true",
        "\"anchor_settlement_truth\":true",
        "anchor_payment_truth:true",
        "\"anchor_payment_truth\":true",
        "anchor_paid_unlock_authority:true",
        "\"anchor_paid_unlock_authority\":true",
        "policy_anchor_truth:true",
        "\"policy_anchor_truth\":true",
        "policy_anchor_finality:true",
        "\"policy_anchor_finality\":true",
        "policy_anchor_settlement:true",
        "\"policy_anchor_settlement\":true",
        "grant_anchor_authority(",
        "commit_anchor_truth(",
        "accept_anchor_proof(",
        "accept_anchor_evidence(",
        "unlock_from_anchor(",
        "settle_from_anchor(",
        "finalize_from_anchor(",
        "mutate_from_anchor(",
        "bridge_from_anchor(",
        "grant_external_settlement_authority(",
        "solana_settlement(",
        "rox_settlement(",
        "bridge_settlement(",
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
fn preflight_runner_dynamically_discovers_phase5_anchor_policy_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "ron-policy dev-quickchain-preflight.sh");
    }
}
