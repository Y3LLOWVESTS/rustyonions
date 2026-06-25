#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative lifecycle gating, but decisions/obligations are not lifecycle, governance, paid-unlock, wallet, ledger, finality, bridge, staking, slashing, or settlement authority.
//! RO:INTERACTS — parse::validate, economics::validate, schema/policybundle.schema.json, docs/quickchain-preflight.md.
//! RO:INVARIANTS — policy gates only; policy does not rotate/revoke validators, accept evidence, update governance params, unlock paid content, or mutate wallet/ledger state.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase3_validator_lifecycle_boundary.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE3_LIFECYCLE_AUTHORITY_TAGS: &[&str] = &[
    "validator_downtime",
    "validator-degraded",
    "validator.equivocation",
    "validator/equivocation/evidence",
    "validator_double_attestation",
    "validator split brain",
    "validator_lifecycle_decision",
    "lifecycle_decision",
    "replay_challenge",
    "replay_challenge_evidence",
    "governance_parameter_update",
    "governance_approval",
];

const PHASE3_LIFECYCLE_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "mark-validator-downtime",
    "mark-validator-degraded",
    "submit-validator-equivocation-evidence",
    "submit-double-attestation-evidence",
    "submit-split-brain-evidence",
    "submit-replay-challenge",
    "submit-replay-challenge-evidence",
    "commit-governance-parameter-update",
    "grant-governance-approval",
    "grant-validator-lifecycle-decision",
    "unlock-from-validator-lifecycle",
    "settle-from-replay-challenge",
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

fn production_source_text() -> String {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    let mut source = String::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        source.push_str(&strip_line_comments(&content));
        source.push('\n');
    }

    source.to_ascii_lowercase()
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 3 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 3 Round 2 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "rules": [
            {
                "id": "phase3-lifecycle-tags",
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
                "id": "phase3-lifecycle-param",
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
                "id": "phase3-lifecycle-kind",
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
fn docs_name_phase3_round2_policy_validator_lifecycle_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 2 validator lifecycle boundary",
        "ron-policy may express declarative lifecycle eligibility/gating rules only",
        "ron-policy is not validator lifecycle authority",
        "ron-policy is not validator rotation authority",
        "ron-policy is not validator revocation authority",
        "ron-policy is not validator downtime authority",
        "ron-policy is not validator degraded-status authority",
        "ron-policy is not validator equivocation authority",
        "ron-policy is not replay challenge authority",
        "ron-policy is not governance parameter-update authority",
        "policy decisions, reasons, obligations, tags, economics config, feature flags, and explanations cannot admit validators, revoke validators, rotate validators, mark validators down, accept equivocation evidence, accept replay challenge evidence, commit governance parameter updates, unlock paid content, mutate balances, issue receipts, prove finality, prove settlement, or replace wallet/ledger truth",
        "validator lifecycle policy is declarative gating only",
        "validator lifecycle/evidence/governance material cannot mint, transfer, burn, hold, capture, release, issue receipts, mutate balances, prove finality, prove settlement, or unlock paid content",
        "ron-policy must reject validator lifecycle/evidence/governance authority smuggling through condition tags, obligation kinds, obligation params, economics identifiers, schema, and source boundaries",
        "quickchain_phase3_validator_lifecycle_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_lifecycle_display_classification_tags_remain_allowed() {
    let bundle = policy_with_required_tags(&[
        "validator-downtime-display",
        "validator-degraded-status-display",
        "validator-lifecycle-readiness-display",
        "replay-challenge-display",
        "governance-review",
    ]);

    let policy = load_json(&bundle).expect("ordinary lifecycle display tags stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should evaluate");

    let ctx = Context::builder()
        .method("GET")
        .tag("validator-downtime-display")
        .tag("validator-degraded-status-display")
        .tag("validator-lifecycle-readiness-display")
        .tag("replay-challenge-display")
        .tag("governance-review")
        .build(&SystemClock);

    let decision = evaluator
        .evaluate(&ctx)
        .expect("display classification policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "validator_lifecycle_authority",
        "validator_lifecycle_decision",
        "governance_parameter_update",
        "replay_challenge_evidence",
        "unlock_granted",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
    ] {
        assert_not_contains(&debug, forbidden, "policy decision debug");
    }
}

#[test]
fn lifecycle_authority_condition_tags_reject() {
    for tag in PHASE3_LIFECYCLE_AUTHORITY_TAGS {
        let bundle = policy_with_required_tags(&[tag]);
        let err = load_json(&bundle).expect_err("authority-shaped lifecycle tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should be rejected as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn lifecycle_authority_obligation_param_keys_reject() {
    for key in PHASE3_LIFECYCLE_AUTHORITY_TAGS {
        let bundle = policy_with_obligation_param(key);
        let err =
            load_json(&bundle).expect_err("authority-shaped lifecycle obligation param rejects");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should be rejected as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn lifecycle_authority_obligation_kinds_reject() {
    for kind in PHASE3_LIFECYCLE_AUTHORITY_OBLIGATION_KINDS {
        let bundle = policy_with_obligation_kind(kind);
        let err = load_json(&bundle).expect_err("validator lifecycle obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should be rejected as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase3_round2_exact_authority_shapes() {
    let parser = read_rel("src/parse/validate.rs");
    let economics = read_rel("src/economics/validate.rs");

    for required in [
        "\"validatordowntime\"",
        "\"validatordegraded\"",
        "\"validatorequivocation\"",
        "\"validatorequivocationevidence\"",
        "\"validatordoubleattestation\"",
        "\"validatorsplitbrain\"",
        "\"validatorlifecycledecision\"",
        "\"lifecycledecision\"",
        "\"replaychallenge\"",
        "\"replaychallengeevidence\"",
        "\"governanceparameterupdate\"",
        "\"governanceapproval\"",
    ] {
        assert_contains(&parser, required, "ron-policy parse validator");
        assert_contains(&economics, required, "ron-policy economics validator");
    }

    for required in [
        "\"markvalidatordowntime\"",
        "\"markvalidatordegraded\"",
        "\"submitvalidatorequivocationevidence\"",
        "\"submitdoubleattestationevidence\"",
        "\"submitsplitbrainevidence\"",
        "\"submitreplaychallenge\"",
        "\"submitreplaychallengeevidence\"",
        "\"commitgovernanceparameterupdate\"",
        "\"grantgovernanceapproval\"",
        "\"grantvalidatorlifecycledecision\"",
        "\"unlockfromvalidatorlifecycle\"",
        "\"settlefromreplaychallenge\"",
    ] {
        assert_contains(
            &parser,
            required,
            "ron-policy parse validator forbidden kind list",
        );
    }
}

#[test]
fn production_source_does_not_construct_lifecycle_governance_paid_unlock_or_settlement_runtime() {
    let source = production_source_text();

    for forbidden in [
        "svc_wallet::",
        "ron_ledger::",
        "svc_passport::",
        "svc_registry::",
        "ron_auth::",
        "ron_proto::quickchain",
        "quickchain::",
        "solana_client",
        "spl_token",
        "anchor_lang",
        "grant_validator_lifecycle_authority(",
        "grant_lifecycle_decision(",
        "commit_validator_rotation(",
        "commit_validator_revocation(",
        "mark_validator_downtime(",
        "mark_validator_degraded(",
        "accept_validator_equivocation(",
        "accept_replay_challenge(",
        "commit_governance_parameter_update(",
        "policy_decision_proves_lifecycle",
        "policy_decision_proves_governance",
        "policy_decision_proves_replay_challenge",
        "unlock_from_validator_lifecycle",
        "unlock_from_replay_challenge",
        "unlock_from_governance_approval",
        "receipt_from_validator_lifecycle",
        "balance_from_validator_lifecycle",
        "finality_from_validator_lifecycle",
        "settlement_from_validator_lifecycle",
        "mutate_balance(",
        "create_receipt(",
        "grant_paid_access(",
        "commit_checkpoint(",
        "anchor_checkpoint(",
        "bridge_settlement(",
        "slash_validator(",
        "stake_validator(",
    ] {
        assert_not_contains(&source, forbidden, "ron-policy production source");
    }
}
