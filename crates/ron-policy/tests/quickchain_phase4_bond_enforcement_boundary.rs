#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 3 controlled bond-enforcement boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative slash/capture/release gates, but decisions/obligations/config are never reserve-slash, release-slash-reserve, capture-slash-reserve, wallet, ledger, finality, bridge, settlement, staking, or liquidity authority.
//! RO:INTERACTS — parse::validate, economics::validate, docs/quickchain-preflight.md.
//! RO:INVARIANTS — policy gates only; no policy-created balance mutation, bond enforcement truth, slash reserve authority, or paid unlock.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase4_bond_enforcement_boundary.

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

const PHASE4_R3_AUTHORITY_TAGS: &[&str] = &[
    "bond_enforcement",
    "bond-enforcement-decision",
    "bond.enforcement.authority",
    "bond/enforcement/operation",
    "validator_bond_enforcement",
    "reserve_slash",
    "release_slash_reserve",
    "capture_slash_reserve",
    "slash_reserve",
    "slash_reserved",
    "slash-reserve-release",
    "slash-reserve-capture",
    "bond_reserve",
    "bond_capture",
    "bond_release",
    "controlled_slash",
    "controlled-slash-release",
    "controlled-slash-capture",
    "live_bond_enforcement",
    "automatic_bond_enforcement",
];

const PHASE4_R3_ALLOWED_DISPLAY_TAGS: &[&str] = &[
    "bond-enforcement-display",
    "bond-enforcement-policy",
    "slash-reserve-display",
    "controlled-slash-status-display",
    "reserve-slash-review",
    "capture-release-review",
];

const PHASE4_R3_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "grant-bond-enforcement-authority",
    "commit-bond-enforcement",
    "execute-bond-enforcement",
    "reserve-slash",
    "release-slash-reserve",
    "capture-slash-reserve",
    "commit-slash-reserve",
    "grant-slash-reserve-authority",
    "capture-controlled-slash",
    "release-controlled-slash",
    "settle-bond-enforcement",
    "unlock-from-bond-enforcement",
    "settle-from-slash-reserve",
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
        "{label} must contain required Phase 4 Round 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 4 Round 3 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase4-bond-enforcement-display-policy",
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
                "reason": "declarative bond enforcement display policy only"
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
                "id": "phase4-round3-obligation-param",
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
                "id": "phase4-round3-obligation-kind",
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
fn docs_name_phase4_round3_policy_bond_enforcement_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 3 controlled bond enforcement policy boundary",
        "ron-policy may express declarative bond enforcement eligibility policy only",
        "ron-policy may express declarative slash reserve policy only",
        "ron-policy may express declarative capture/release gate policy only",
        "ron-policy is not bond enforcement truth",
        "ron-policy is not reserve-slash authority",
        "ron-policy is not release-slash-reserve authority",
        "ron-policy is not capture-slash-reserve authority",
        "policy allow is not bond enforcement proof",
        "policy obligation is not slash reserve evidence acceptance",
        "policy decision cannot mutate wallet or ledger",
        "policy decision cannot unlock paid content from bond enforcement material",
        "policy config cannot create slash reserves",
        "policy config cannot capture or release slash reserves",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase4_bond_enforcement_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_bond_enforcement_display_policy_remains_allowed_but_non_authoritative() {
    let policy = load_json(&policy_with_required_tags(PHASE4_R3_ALLOWED_DISPLAY_TAGS))
        .expect("ordinary Round 3 display/gating tags should stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should validate");

    let mut builder = Context::builder().tenant("t").method("GET").region("US");
    for tag in PHASE4_R3_ALLOWED_DISPLAY_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative bond enforcement display policy only")
    );

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "bond_enforcement_authority",
        "bond_enforcement_truth",
        "reserve_slash_authority",
        "release_slash_reserve_authority",
        "capture_slash_reserve_authority",
        "slash_reserve_truth",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
        "unlock_granted",
    ] {
        assert_not_contains(&debug, forbidden, "ron-policy decision/debug shape");
    }
}

#[test]
fn phase4_round3_bond_enforcement_authority_condition_tags_reject() {
    for tag in PHASE4_R3_AUTHORITY_TAGS {
        let err = load_json(&policy_with_required_tags(&[*tag]))
            .expect_err("Phase 4 Round 3 authority-shaped tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase4_round3_bond_enforcement_authority_obligation_params_reject() {
    for key in PHASE4_R3_AUTHORITY_TAGS {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("Phase 4 Round 3 authority-shaped obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase4_round3_bond_enforcement_authority_obligation_kinds_reject() {
    for kind in PHASE4_R3_AUTHORITY_OBLIGATION_KINDS {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("Phase 4 Round 3 authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase4_round3_exact_authority_shapes() {
    let parser = normalized(&read_rel("src/parse/validate.rs"));
    let economics = normalized(&read_rel("src/economics/validate.rs"));

    for required in [
        "\"bondenforcement\"",
        "\"bondenforcementdecision\"",
        "\"bondenforcementauthority\"",
        "\"bondenforcementoperation\"",
        "\"validatorbondenforcement\"",
        "\"reserveslash\"",
        "\"releaseslashreserve\"",
        "\"captureslashreserve\"",
        "\"slashreserve\"",
        "\"slashreserved\"",
        "\"slashreserverelease\"",
        "\"slashreservecapture\"",
        "\"bondreserve\"",
        "\"bondcapture\"",
        "\"bondrelease\"",
        "\"controlledslash\"",
        "\"controlledslashrelease\"",
        "\"controlledslashcapture\"",
        "\"livebondenforcement\"",
        "\"automaticbondenforcement\"",
    ] {
        assert_contains(&parser, required, "ron-policy parser validation table");
        assert_contains(
            &economics,
            required,
            "ron-policy economics validation table",
        );
    }

    for required in [
        "\"grantbondenforcementauthority\"",
        "\"commitbondenforcement\"",
        "\"executebondenforcement\"",
        "\"reserveslash\"",
        "\"releaseslashreserve\"",
        "\"captureslashreserve\"",
        "\"commitslashreserve\"",
        "\"grantslashreserveauthority\"",
        "\"capturecontrolledslash\"",
        "\"releasecontrolledslash\"",
        "\"settlebondenforcement\"",
        "\"unlockfrombondenforcement\"",
        "\"settlefromslashreserve\"",
    ] {
        assert_contains(
            &parser,
            required,
            "ron-policy parser forbidden obligation kind table",
        );
    }
}

#[test]
fn economics_config_rejects_phase4_round3_authority_shaped_aliases() {
    let base = load_checked_in_economics();

    for forbidden_alias in PHASE4_R3_AUTHORITY_TAGS {
        let mut policy = base.clone();
        policy
            .accounts
            .insert((*forbidden_alias).to_owned(), "acct:forbidden".to_owned());

        let err = validate_economics_policy(&policy)
            .expect_err("authority-shaped economics account alias must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "economics alias {forbidden_alias} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn production_source_does_not_construct_phase4_round3_runtime_or_mutation_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "bond_enforcement_authority:true",
        "\"bond_enforcement_authority\":true",
        "bond_enforcement_truth:true",
        "\"bond_enforcement_truth\":true",
        "reserve_slash_authority:true",
        "\"reserve_slash_authority\":true",
        "release_slash_reserve_authority:true",
        "\"release_slash_reserve_authority\":true",
        "capture_slash_reserve_authority:true",
        "\"capture_slash_reserve_authority\":true",
        "slash_reserve_truth:true",
        "\"slash_reserve_truth\":true",
        "policy_decision_proves_bond_enforcement",
        "policy_decision_grants_reserve_slash",
        "policy_decision_grants_release_slash_reserve",
        "policy_decision_grants_capture_slash_reserve",
        "policy_obligation_is_bond_enforcement",
        "policy_obligation_is_slash_reserve",
        "policy_allow_unlocks_bond_enforcement_material",
        "unlock_from_bond_enforcement",
        "unlock_from_slash_reserve",
        "unlock_from_reserve_slash",
        "unlock_from_capture_slash_reserve",
        "receipt_from_bond_enforcement",
        "balance_from_bond_enforcement",
        "finality_from_bond_enforcement",
        "settlement_from_bond_enforcement",
        "execute_bond_enforcement(",
        "resolve_bond_enforcement(",
        "reserve_slash(",
        "release_slash_reserve(",
        "capture_slash_reserve(",
        "commit_slash_reserve(",
        "execute_controlled_slash(",
        "slash_without_governance(",
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

#[test]
fn preflight_runner_names_phase4_round3_policy_bond_enforcement_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "quickchain_phase4_bond_enforcement_boundary",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "ron-policy dev-quickchain-preflight.sh");
    }
}
