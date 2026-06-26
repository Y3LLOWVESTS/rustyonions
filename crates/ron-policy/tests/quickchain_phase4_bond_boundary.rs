#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 1 bond/slash/stake/liquidity boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative bond eligibility and slash-simulation gates, but decisions/obligations/config are never bond, slash, staking, liquidity, wallet, ledger, finality, bridge, or settlement authority.
//! RO:INTERACTS — parse::validate, economics::validate, docs/quickchain-preflight.md.
//! RO:INVARIANTS — policy gates only; no policy-created balance mutation, bond lifecycle truth, slash execution, public staking market, liquidity, or paid unlock.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase4_bond_boundary.

use ron_policy::{
    ctx::clock::SystemClock,
    economics::{load_economics_toml_str, validate_economics_policy},
    engine::eval::DecisionEffect,
    load_json, Context, Evaluator,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const CHECKED_IN_POLICY: &str = include_str!("../../../configs/roc-economics.toml");

const PHASE4_AUTHORITY_TAGS: &[&str] = &[
    "bond_authority",
    "bond-truth",
    "bond.account.truth",
    "bond/lifecycle/authority",
    "bond_lifecycle_decision",
    "validator_bond",
    "validator-bond-authority",
    "bonded_stake",
    "slash_authority",
    "slash-truth",
    "slash_decision",
    "slash.evidence.authority",
    "slashing_authority",
    "stake_authority",
    "staking_authority",
    "staking-market-authority",
    "public_staking_market",
    "liquidity_authority",
    "liquidity.pool.authority",
    "live_slash",
    "automatic_slash",
];

const PHASE4_ALLOWED_DISPLAY_TAGS: &[&str] = &[
    "bond-readiness-display",
    "bond-eligibility-policy",
    "slash-evidence-display",
    "slash-simulation-display",
    "staking-disabled-display",
    "liquidity-disabled-display",
];

const PHASE4_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "grant-bond-authority",
    "commit-bond-lifecycle",
    "grant-bond-lifecycle-decision",
    "mark-validator-bonded",
    "capture-validator-bond",
    "release-validator-bond",
    "slash-validator",
    "execute-slashing",
    "commit-slash-decision",
    "open-staking-market",
    "grant-staking-authority",
    "create-liquidity-pool",
    "grant-liquidity-authority",
    "settle-bond",
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

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 4 Round 1 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 4 Round 1 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase4-bond-display-policy",
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
                "reason": "declarative bond/slash display policy only"
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
                "id": "phase4-obligation-param",
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
                "id": "phase4-obligation-kind",
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

fn load_checked_in_economics() -> ron_policy::economics::EconomicsPolicy {
    load_economics_toml_str(CHECKED_IN_POLICY).expect("checked-in economics config should load")
}

#[test]
fn docs_name_phase4_round1_policy_bond_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 1 bonded validator policy boundary",
        "ron-policy may express declarative bond eligibility policy",
        "ron-policy may express declarative slash simulation policy only",
        "ron-policy is not bond truth",
        "ron-policy is not slash truth",
        "ron-policy is not slashing authority",
        "ron-policy is not staking market authority",
        "ron-policy is not liquidity authority",
        "policy allow is not bond lifecycle proof",
        "policy obligation is not slash evidence acceptance",
        "policy decision cannot mutate wallet or ledger",
        "policy decision cannot unlock paid content from bond or slash material",
        "policy config cannot create bond balances",
        "policy config cannot create slash authority",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase4_bond_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_bond_and_slash_display_policy_remains_allowed_but_non_authoritative() {
    let policy = load_json(&policy_with_required_tags(PHASE4_ALLOWED_DISPLAY_TAGS))
        .expect("ordinary display/gating tags should stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should validate");

    let mut builder = Context::builder().tenant("t").method("GET").region("US");
    for tag in PHASE4_ALLOWED_DISPLAY_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative bond/slash display policy only")
    );

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "bond_authority",
        "bond_truth",
        "bond_lifecycle_decision",
        "slash_authority",
        "slash_truth",
        "slash_decision",
        "staking_authority",
        "liquidity_authority",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
        "unlock_granted",
    ] {
        assert_not_contains(&debug, forbidden, "ron-policy decision/debug shape");
    }
}

#[test]
fn phase4_bond_slash_stake_and_liquidity_authority_condition_tags_reject() {
    for tag in PHASE4_AUTHORITY_TAGS {
        let err = load_json(&policy_with_required_tags(&[*tag]))
            .expect_err("Phase 4 authority-shaped tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase4_bond_slash_stake_and_liquidity_authority_obligation_params_reject() {
    for key in PHASE4_AUTHORITY_TAGS {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("Phase 4 authority-shaped obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase4_bond_slash_stake_and_liquidity_authority_obligation_kinds_reject() {
    for kind in PHASE4_AUTHORITY_OBLIGATION_KINDS {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("Phase 4 authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn economics_identifiers_reject_phase4_authority_shapes_without_over_banning_display_words() {
    let checked = load_checked_in_economics();
    validate_economics_policy(&checked).expect("checked-in economics policy remains valid");

    for account_alias in [
        "bond_authority",
        "bond_lifecycle_decision",
        "slash_truth",
        "staking_authority",
        "liquidity_authority",
    ] {
        let mut policy = checked.clone();
        policy.accounts.insert(
            account_alias.to_owned(),
            format!("t:default/{account_alias}"),
        );

        let err = validate_economics_policy(&policy)
            .expect_err("authority-shaped economics account alias must reject");
        assert!(
            err.to_string().contains("economic authority"),
            "account alias {account_alias} should reject as authority-shaped, got: {err}"
        );
    }

    for role_alias in [
        "validator_bond",
        "slash_decision",
        "public_staking_market",
        "liquidity_pool_authority",
    ] {
        let mut policy = checked.clone();
        policy.roles.insert(
            role_alias.to_owned(),
            format!("dynamic {role_alias} recipient"),
        );

        let err = validate_economics_policy(&policy)
            .expect_err("authority-shaped economics role alias must reject");
        assert!(
            err.to_string().contains("economic authority"),
            "role alias {role_alias} should reject as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase4_exact_authority_shapes() {
    let parser = normalized(&read_rel("src/parse/validate.rs"));
    let economics = normalized(&read_rel("src/economics/validate.rs"));

    for required in [
        "\"bondauthority\"",
        "\"bondtruth\"",
        "\"bondlifecycledecision\"",
        "\"validatorbond\"",
        "\"slashauthority\"",
        "\"slashtruth\"",
        "\"slashdecision\"",
        "\"stakingauthority\"",
        "\"publicstakingmarket\"",
        "\"liquidityauthority\"",
        "\"liveslash\"",
        "\"automaticslash\"",
    ] {
        assert_contains(&parser, required, "ron-policy parser validation table");
        assert_contains(
            &economics,
            required,
            "ron-policy economics validation table",
        );
    }
}

#[test]
fn production_source_does_not_construct_phase4_runtime_or_mutation_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "bond_authority:true",
        "\"bond_authority\":true",
        "bond_truth:true",
        "\"bond_truth\":true",
        "slash_authority:true",
        "\"slash_authority\":true",
        "slash_truth:true",
        "\"slash_truth\":true",
        "staking_authority:true",
        "\"staking_authority\":true",
        "liquidity_authority:true",
        "\"liquidity_authority\":true",
        "execute_bond(",
        "apply_bond(",
        "commit_bond(",
        "capture_bond(",
        "release_bond(",
        "execute_slash(",
        "apply_slash(",
        "commit_slash(",
        "slash_validator(",
        "open_staking_market(",
        "create_liquidity_pool(",
        "bridge_settlement(",
        "external_settlement(",
        "mint_rox(",
        "solana_settlement(",
        "svc_wallet::",
        "ron_ledger::",
        "solana_sdk",
        "solana_client",
        "anchor_lang",
        "spl_token",
    ];

    for path in files {
        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read source {}: {err}", path.display())),
        ));
        let compact = source.split_whitespace().collect::<String>();

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("ron-policy source {}", path.display()),
            );
        }
    }
}
