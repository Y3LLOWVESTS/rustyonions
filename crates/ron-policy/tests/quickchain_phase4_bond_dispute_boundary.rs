#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 2 dispute/challenge/appeal/freeze boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative dispute/challenge visibility gates, but decisions/obligations/config are never dispute truth, challenge-window truth, appeal/freeze/slash authority, wallet, ledger, finality, bridge, or settlement authority.
//! RO:INTERACTS — parse::validate, economics::validate, docs/quickchain-preflight.md.
//! RO:INVARIANTS — policy gates only; no policy-created balance mutation, dispute resolution, challenge evidence acceptance, freeze authority, irreversible slash, or paid unlock.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase4_bond_dispute_boundary.

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

const PHASE4_R2_AUTHORITY_TAGS: &[&str] = &[
    "bond_dispute",
    "bond-dispute-state",
    "dispute_truth",
    "dispute.authority",
    "dispute/window/authority",
    "challenge_window",
    "challenge-window-authority",
    "appeal_authority",
    "appeal-window-authority",
    "freeze_authority",
    "frozen_bond",
    "disputed_bond",
    "irreversible_slash",
    "slash_appeal",
    "slash_challenge",
    "slash_simulation_authority",
    "live_dispute_resolution",
    "automatic_dispute_slash",
];

const PHASE4_R2_ALLOWED_DISPLAY_TAGS: &[&str] = &[
    "bond-dispute-display",
    "challenge-window-display",
    "slash-evidence-review-display",
    "appeal-state-display",
    "freeze-status-display",
    "slash-simulation-display",
];

const PHASE4_R2_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "grant-dispute-authority",
    "commit-bond-dispute-state",
    "open-challenge-window",
    "grant-challenge-window-authority",
    "submit-slash-appeal",
    "grant-appeal-authority",
    "freeze-bond",
    "capture-disputed-bond",
    "slash-disputed-bond",
    "execute-irreversible-slash",
    "commit-irreversible-slash",
    "settle-bond-dispute",
    "unlock-from-bond-dispute",
    "settle-from-slash-challenge",
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
        "{label} must contain required Phase 4 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 4 Round 2 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase4-dispute-display-policy",
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
                "reason": "declarative dispute/challenge display policy only"
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
                "id": "phase4-dispute-obligation-param",
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
                "id": "phase4-dispute-obligation-kind",
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
fn docs_name_phase4_round2_policy_dispute_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 2 slashing challenge policy boundary",
        "phase 4 round 2 is simulation only",
        "ron-policy may express declarative dispute visibility policy",
        "ron-policy may express declarative challenge window policy only",
        "ron-policy may express declarative appeal/freeze display policy only",
        "ron-policy is not dispute truth",
        "ron-policy is not challenge-window truth",
        "ron-policy is not appeal authority",
        "ron-policy is not freeze authority",
        "ron-policy is not irreversible slash authority",
        "ron-policy is not slash simulation authority",
        "policy allow is not dispute resolution proof",
        "policy obligation is not challenge evidence acceptance",
        "policy decision cannot mutate wallet or ledger",
        "policy decision cannot unlock paid content from dispute/challenge/appeal/freeze material",
        "policy config cannot create disputed bond balances",
        "policy config cannot create freeze authority",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "no live irreversible slash through ron-policy",
        "quickchain_phase4_bond_dispute_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_dispute_and_challenge_display_policy_remains_allowed_but_non_authoritative() {
    let policy = load_json(&policy_with_required_tags(PHASE4_R2_ALLOWED_DISPLAY_TAGS))
        .expect("ordinary display/gating tags should stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should validate");

    let mut builder = Context::builder().tenant("t").method("GET").region("US");
    for tag in PHASE4_R2_ALLOWED_DISPLAY_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative dispute/challenge display policy only")
    );

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "dispute_authority",
        "dispute_truth",
        "challenge_window_authority",
        "appeal_authority",
        "freeze_authority",
        "irreversible_slash",
        "slash_simulation_authority",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
        "unlock_granted",
    ] {
        assert_not_contains(&debug, forbidden, "ron-policy decision/debug shape");
    }
}

#[test]
fn phase4_round2_dispute_challenge_appeal_and_freeze_authority_condition_tags_reject() {
    for tag in PHASE4_R2_AUTHORITY_TAGS {
        let err = load_json(&policy_with_required_tags(&[*tag]))
            .expect_err("Phase 4 Round 2 authority-shaped tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase4_round2_dispute_challenge_appeal_and_freeze_authority_obligation_params_reject() {
    for key in PHASE4_R2_AUTHORITY_TAGS {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("Phase 4 Round 2 authority-shaped obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase4_round2_dispute_challenge_appeal_and_freeze_authority_obligation_kinds_reject() {
    for kind in PHASE4_R2_AUTHORITY_OBLIGATION_KINDS {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("Phase 4 Round 2 authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase4_round2_exact_authority_shapes() {
    let parser = normalized(&read_rel("src/parse/validate.rs"));
    let economics = normalized(&read_rel("src/economics/validate.rs"));

    for required in [
        "\"bonddispute\"",
        "\"bonddisputestate\"",
        "\"disputetruth\"",
        "\"disputeauthority\"",
        "\"disputewindowauthority\"",
        "\"challengewindow\"",
        "\"challengewindowauthority\"",
        "\"appealauthority\"",
        "\"appealwindowauthority\"",
        "\"freezeauthority\"",
        "\"frozenbond\"",
        "\"disputedbond\"",
        "\"irreversibleslash\"",
        "\"slashappeal\"",
        "\"slashchallenge\"",
        "\"slashsimulationauthority\"",
        "\"livedisputeresolution\"",
        "\"automaticdisputeslash\"",
    ] {
        assert_contains(&parser, required, "ron-policy parser validation table");
        assert_contains(
            &economics,
            required,
            "ron-policy economics validation table",
        );
    }

    for required in [
        "\"grantdisputeauthority\"",
        "\"commitbonddisputestate\"",
        "\"openchallengewindow\"",
        "\"grantchallengewindowauthority\"",
        "\"submitslashappeal\"",
        "\"grantappealauthority\"",
        "\"freezebond\"",
        "\"capturedisputedbond\"",
        "\"slashdisputedbond\"",
        "\"executeirreversibleslash\"",
        "\"commitirreversibleslash\"",
        "\"settlebonddispute\"",
        "\"unlockfrombonddispute\"",
        "\"settlefromslashchallenge\"",
    ] {
        assert_contains(
            &parser,
            required,
            "ron-policy parser forbidden obligation kind table",
        );
    }
}

#[test]
fn economics_config_rejects_phase4_round2_authority_shaped_aliases() {
    let base = load_checked_in_economics();

    for forbidden_alias in PHASE4_R2_AUTHORITY_TAGS {
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
fn production_source_does_not_construct_phase4_round2_runtime_or_mutation_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "dispute_authority:true",
        "\"dispute_authority\":true",
        "dispute_truth:true",
        "\"dispute_truth\":true",
        "challenge_window_authority:true",
        "\"challenge_window_authority\":true",
        "appeal_authority:true",
        "\"appeal_authority\":true",
        "freeze_authority:true",
        "\"freeze_authority\":true",
        "irreversible_slash_authority:true",
        "\"irreversible_slash_authority\":true",
        "slash_simulation_authority:true",
        "\"slash_simulation_authority\":true",
        "policy_decision_proves_dispute",
        "policy_decision_proves_challenge",
        "policy_decision_grants_appeal",
        "policy_decision_grants_freeze",
        "policy_obligation_is_challenge_evidence",
        "policy_obligation_is_appeal_authority",
        "policy_obligation_is_freeze_authority",
        "policy_allow_unlocks_dispute_material",
        "unlock_from_bond_dispute",
        "unlock_from_slash_challenge",
        "unlock_from_appeal",
        "unlock_from_freeze",
        "receipt_from_dispute",
        "balance_from_dispute",
        "finality_from_dispute",
        "settlement_from_dispute",
        "execute_dispute(",
        "resolve_dispute(",
        "accept_challenge_evidence(",
        "open_challenge_window(",
        "grant_appeal_authority(",
        "freeze_bond(",
        "capture_disputed_bond(",
        "slash_disputed_bond(",
        "execute_irreversible_slash(",
        "commit_irreversible_slash(",
        "slash_without_governance(",
        "svc_wallet::",
        "ron_ledger::",
        "solana_sdk",
        "solana_client",
        "anchor_lang",
        "spl_token",
        "bridge_settlement(",
        "external_settlement(",
        "mint_rox(",
        "solana_settlement(",
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
fn preflight_runner_names_phase4_round2_policy_dispute_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "quickchain_phase4_bond_dispute_boundary",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "ron-policy dev-quickchain-preflight.sh");
    }
}
