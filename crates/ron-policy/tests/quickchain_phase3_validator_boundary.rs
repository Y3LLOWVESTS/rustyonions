#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 3 Round 1 validator/passport boundary tests for ron-policy.
//! RO:WHY — Policy may express deny-by-default validator eligibility, but policy decisions are not validator identity, passport registry, capability issuance, validator-set truth, paid unlocks, finality, or settlement.
//! RO:INTERACTS — parse::validate, schema/policybundle.schema.json, docs/quickchain-preflight.md.
//! RO:INVARIANTS — policy gates; policy does not admit/revoke/rotate validators or mutate wallet/ledger state.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase3_validator_boundary.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE3_AUTHORITY_TAGS: &[&str] = &[
    "validator_passport",
    "validator-passport",
    "validator.passport",
    "validator/passport",
    "validator capability",
    "validator_capability",
    "validator_registry_entry",
    "validator_membership_proof",
    "validator_authorization",
    "validator_authz_result",
    "passport_validator",
    "passport_validator_admission",
    "passport_validator_capability",
    "registry_validator",
    "registry_validator_set",
    "capability_validator",
    "capability_validator_scope",
    "attestation_identity",
    "validator_admission",
    "validator_revocation",
    "validator_rotation",
    "validator_identity_authority",
    "validator_set_authority",
    "validator_paid_unlock",
];

const PHASE3_FORBIDDEN_OBLIGATION_KINDS: &[&str] = &[
    "admit-validator",
    "revoke-validator",
    "rotate-validator",
    "authorize-validator",
    "register-validator",
    "deregister-validator",
    "set-validator-set",
    "update-validator-set",
    "commit-validator-set",
    "grant-validator-capability",
    "grant-validator-admission",
    "sign-validator-attestation",
    "verify-validator-attestation",
    "unlock-from-validator-passport",
    "unlock-from-validator-capability",
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
        "{label} must contain required Phase 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, label: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{label} must not contain forbidden Phase 3 authority marker: {forbidden}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase3-validator-readiness-display-policy",
                "when": {
                    "method": "GET",
                    "require_tags_all": tags
                },
                "action": "allow",
                "reason": "declarative validator readiness display policy only",
                "obligations": [
                    {
                        "kind": "add-header",
                        "params": {
                            "x-ron-policy-note": "display-only"
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
                "id": "phase3-obligation-param",
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
                "id": "phase3-obligation-kind",
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
fn docs_name_phase3_round1_policy_validator_passport_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 1 validator/passport boundary",
        "ron-policy may express validator eligibility policy",
        "validator eligibility policy is declarative gating only",
        "ron-policy is not validator identity authority",
        "ron-policy is not passport registry authority",
        "ron-policy is not validator capability authority",
        "ron-policy is not validator-set authority",
        "ron-policy cannot admit validators by itself",
        "ron-policy cannot revoke validators by itself",
        "ron-policy cannot rotate validators by itself",
        "ron-policy cannot unlock paid content from validator/passport material",
        "ron-policy cannot replace wallet/ledger truth",
        "quickchain_phase3_validator_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_validator_display_classification_tags_remain_allowed() {
    let bundle = policy_with_required_tags(&[
        "validator-readiness-display",
        "passport-required",
        "registry-checked",
        "governance-review",
    ]);

    let policy =
        load_json(&bundle).expect("ordinary display/gating classification tags stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should evaluate");

    let ctx = Context::builder()
        .method("GET")
        .tag("validator-readiness-display")
        .tag("passport-required")
        .tag("registry-checked")
        .tag("governance-review")
        .build(&SystemClock);

    let decision = evaluator
        .evaluate(&ctx)
        .expect("display classification policy should evaluate");
    assert_eq!(decision.effect, DecisionEffect::Allow);

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "validator_identity_authority",
        "passport_registry_authority",
        "validator_capability_authority",
        "validator_set_authority",
        "validator_paid_unlock",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
    ] {
        assert_not_contains(&debug, forbidden, "policy decision debug");
    }
}

#[test]
fn phase3_validator_passport_authority_condition_tags_reject() {
    for tag in PHASE3_AUTHORITY_TAGS {
        let bundle = policy_with_required_tags(&[tag]);
        let err = load_json(&bundle).expect_err("authority-shaped Phase 3 tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should be rejected as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn phase3_validator_passport_authority_obligation_param_keys_reject() {
    for key in PHASE3_AUTHORITY_TAGS {
        let bundle = policy_with_obligation_param(key);
        let err =
            load_json(&bundle).expect_err("authority-shaped Phase 3 obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should be rejected as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn phase3_validator_lifecycle_obligation_kinds_reject() {
    for kind in PHASE3_FORBIDDEN_OBLIGATION_KINDS {
        let bundle = policy_with_obligation_kind(kind);
        let err = load_json(&bundle).expect_err("validator lifecycle obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should be rejected as authority-shaped, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase3_exact_authority_shapes() {
    let parser = read_rel("src/parse/validate.rs");
    let economics = read_rel("src/economics/validate.rs");

    for required in [
        "\"validatorpassport\"",
        "\"validatorcapability\"",
        "\"validatorregistryentry\"",
        "\"validatormembershipproof\"",
        "\"validatorauthorization\"",
        "\"validatorauthzresult\"",
        "\"passportvalidator\"",
        "\"passportvalidatoradmission\"",
        "\"passportvalidatorcapability\"",
        "\"registryvalidator\"",
        "\"registryvalidatorset\"",
        "\"capabilityvalidator\"",
        "\"capabilityvalidatorscope\"",
        "\"attestationidentity\"",
    ] {
        assert_contains(&parser, required, "ron-policy parse validator");
        assert_contains(&economics, required, "ron-policy economics validator");
    }
}

#[test]
fn production_source_does_not_import_wallet_ledger_passport_registry_or_external_runtime_authority()
{
    let source = production_source_text();

    for forbidden in [
        "use ron_ledger",
        "ron_ledger::",
        "use svc_wallet",
        "svc_wallet::",
        "use svc_passport",
        "svc_passport::",
        "use svc_registry",
        "svc_registry::",
        "use ron_auth",
        "ron_auth::",
        "solana_client",
        "spl_token",
        "anchor_lang",
        "mutate_balance(",
        "create_receipt(",
        "grant_paid_access(",
        "admit_validator(",
        "revoke_validator(",
        "rotate_validator(",
        "register_validator(",
        "deregister_validator(",
        "slash_validator(",
        "stake_validator(",
    ] {
        assert_not_contains(&source, forbidden, "ron-policy production source");
    }
}
