#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback policy boundary tests for ron-policy.
//! RO:WHY — Policy may express declarative DA/archive/challenge fallback readiness,
//! but decisions/obligations/config are never DA proof, archive proof, paid unlock,
//! pruning, wallet, ledger, finality, settlement, outside-DA, or outside-chain authority.
//! RO:INVARIANTS — policy gates only; no policy-created DA truth, pruning truth, balance truth, or paid unlock.
//! RO:TEST — cargo test -p ron-policy --test quickchain_phase5_da_fallback_policy_boundary.

use ron_policy::{
    ctx::clock::SystemClock, engine::eval::DecisionEffect, load_json, Context, Evaluator,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE5_R2_DA_AUTHORITY_TAGS: &[&str] = &[
    "da_fallback_truth",
    "da-fallback-authority",
    "data.availability.truth",
    "data/availability/authority",
    "archive_restore_authority",
    "archive-proof-truth",
    "carrier.proof.authority",
    "missing_data_challenge_truth",
    "missing-data-challenge-authority",
    "retention_window_authority",
    "pruning_truth",
    "pruning_authority",
    "pruning.approved",
    "prune_now",
    "archive_paid_unlock",
    "da-paid-unlock",
    "outside_da_truth",
    "outside_chain_da_truth",
    "da_settlement_truth",
    "da_finality_truth",
];

const PHASE5_R2_ALLOWED_DISPLAY_TAGS: &[&str] = &[
    "da-fallback-display",
    "data-availability-evidence-display",
    "archive-restore-display",
    "missing-data-challenge-display",
    "retention-window-display",
    "pruning-blocked-display",
    "backend-derived-da-status",
];

const PHASE5_R2_DA_AUTHORITY_OBLIGATION_KINDS: &[&str] = &[
    "grant-da-fallback-authority",
    "accept-da-truth",
    "accept-archive-proof-as-payment",
    "unlock-from-da-fallback",
    "unlock-from-archive-restore",
    "approve-pruning",
    "prune-history",
    "settle-from-da",
    "finalize-from-da",
    "mutate-from-da",
    "grant-outside-da-truth",
    "mark-da-final",
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

fn collect_rust_files(path: &Path, out: &mut Vec<PathBuf>) {
    if !path.exists() {
        return;
    }

    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path.to_path_buf());
        }
        return;
    }

    let entries =
        fs::read_dir(path).unwrap_or_else(|err| panic!("read dir {}: {err}", path.display()));

    for entry in entries {
        let child = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", path.display()))
            .path();

        collect_rust_files(&child, out);
    }
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 5 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 5 Round 2 authority marker: {needle}"
    );
}

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8> {
    json!({
        "version": 1,
        "defaults": { "default_action": "deny" },
        "rules": [
            {
                "id": "phase5-da-fallback-display-policy",
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
                "reason": "declarative DA fallback display policy only"
            }
        ]
    })
    .to_string()
    .into_bytes()
}

fn policy_with_obligation_param(param_key: &str) -> Vec<u8> {
    json!({
        "version": 1,
        "rules": [
            {
                "id": "phase5-da-fallback-obligation-param",
                "when": {
                    "tenant": "*",
                    "method": "GET",
                    "region": "*"
                },
                "action": "allow",
                "obligations": [
                    {
                        "kind": "add-header",
                        "params": {
                            param_key: "forbidden"
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

fn policy_with_obligation_kind(kind: &str) -> Vec<u8> {
    json!({
        "version": 1,
        "rules": [
            {
                "id": "phase5-da-fallback-obligation-kind",
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
fn docs_name_phase5_round2_policy_da_fallback_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 5 round 2 da/archive/challenge fallback policy boundary",
        "ron-policy may express declarative da/archive/challenge fallback visibility or readiness policy only",
        "ron-policy may express declarative retention-window and pruning-blocker policy only",
        "policy allow is not da proof",
        "policy allow is not archive restore proof",
        "policy allow is not missing-data challenge proof",
        "policy obligation is not da evidence acceptance",
        "policy obligation is not archive restore authority",
        "policy obligation is not pruning authority",
        "policy decision cannot mutate wallet or ledger from da/archive/challenge evidence",
        "policy decision cannot unlock paid content from da/archive/challenge evidence",
        "policy decision cannot become balance truth, receipt truth, finality truth, settlement truth, pruning authority, outside-da truth, or outside-chain truth",
        "no pruning can be approved or triggered through ron-policy",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase5_da_fallback_policy_boundary",
    ] {
        assert_contains(&doc, required, "ron-policy quickchain-preflight.md");
    }
}

#[test]
fn ordinary_da_fallback_display_policy_remains_allowed_but_non_authoritative() {
    let policy = load_json(&policy_with_required_tags(PHASE5_R2_ALLOWED_DISPLAY_TAGS))
        .expect("ordinary DA fallback display/gating tags should stay valid");
    let evaluator = Evaluator::new(&policy).expect("policy should validate");

    let mut builder = Context::builder().tenant("t").method("GET").region("US");
    for tag in PHASE5_R2_ALLOWED_DISPLAY_TAGS {
        builder = builder.tag(*tag);
    }

    let decision = evaluator
        .evaluate(&builder.build(&SystemClock))
        .expect("policy should evaluate");

    assert_eq!(decision.effect, DecisionEffect::Allow);
    assert_eq!(
        decision.reason.as_deref(),
        Some("declarative DA fallback display policy only")
    );

    let debug = format!("{decision:?}").to_ascii_lowercase();
    for forbidden in [
        "da_fallback_authority",
        "da_fallback_truth",
        "data_availability_truth",
        "archive_restore_authority",
        "archive_proof_truth",
        "missing_data_challenge_truth",
        "pruning_authority",
        "pruning_approved",
        "prune_now",
        "da_paid_unlock",
        "archive_paid_unlock",
        "outside_da_truth",
        "outside_chain_roc_truth",
        "settlement_status",
        "balance_minor",
        "receipt_hash",
        "unlock_granted",
    ] {
        assert_not_contains(&debug, forbidden, "ron-policy decision/debug shape");
    }
}

#[test]
fn phase5_da_fallback_authority_condition_tags_reject() {
    for tag in PHASE5_R2_DA_AUTHORITY_TAGS {
        let err = load_json(&policy_with_required_tags(&[*tag]))
            .expect_err("Phase 5 Round 2 DA authority-shaped tag must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "tag {tag} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase5_da_fallback_authority_obligation_params_reject() {
    for key in PHASE5_R2_DA_AUTHORITY_TAGS {
        let err = load_json(&policy_with_obligation_param(key))
            .expect_err("Phase 5 Round 2 DA authority-shaped obligation param must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation param {key} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn phase5_da_fallback_authority_obligation_kinds_reject() {
    for kind in PHASE5_R2_DA_AUTHORITY_OBLIGATION_KINDS {
        let err = load_json(&policy_with_obligation_kind(kind))
            .expect_err("Phase 5 Round 2 DA authority-shaped obligation kind must reject");

        assert!(
            err.to_string().contains("economic authority"),
            "obligation kind {kind} should reject as economic authority, got: {err}"
        );
    }
}

#[test]
fn parser_and_economics_validator_contain_phase5_da_fallback_authority_shapes() {
    let parser = normalized(&read_rel("src/parse/validate.rs"));
    let economics = normalized(&read_rel("src/economics/validate.rs"));

    for required in [
        "\"dafallbacktruth\"",
        "\"dafallbackauthority\"",
        "\"dataavailabilitytruth\"",
        "\"dataavailabilityauthority\"",
        "\"archiverestoreauthority\"",
        "\"archiveprooftruth\"",
        "\"carrierproofauthority\"",
        "\"missingdatachallengetruth\"",
        "\"missingdatachallengeauthority\"",
        "\"retentionwindowauthority\"",
        "\"pruningtruth\"",
        "\"pruningauthority\"",
        "\"pruningapproved\"",
        "\"prunenow\"",
        "\"archivepaidunlock\"",
        "\"dapaidunlock\"",
        "\"outsidedatruth\"",
        "\"outsidechaindatruth\"",
        "\"dasettlementtruth\"",
        "\"dafinalitytruth\"",
    ] {
        assert_contains(&parser, required, "ron-policy parser validation table");
        assert_contains(
            &economics,
            required,
            "ron-policy economics validation table",
        );
    }

    for required in [
        "\"grantdafallbackauthority\"",
        "\"acceptdatruth\"",
        "\"acceptarchiveproofaspayment\"",
        "\"unlockfromdafallback\"",
        "\"unlockfromarchiverestore\"",
        "\"approvepruning\"",
        "\"prunehistory\"",
        "\"settlefromda\"",
        "\"finalizefromda\"",
        "\"mutatefromda\"",
        "\"grantoutsidedatruth\"",
        "\"markdafinal\"",
    ] {
        assert_contains(
            &parser,
            required,
            "ron-policy parser forbidden obligation kind table",
        );
    }
}

#[test]
fn production_source_does_not_construct_phase5_da_fallback_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "da_fallback_authority:true",
        "\"da_fallback_authority\":true",
        "da_fallback_truth:true",
        "\"da_fallback_truth\":true",
        "data_availability_truth:true",
        "\"data_availability_truth\":true",
        "archive_restore_authority:true",
        "\"archive_restore_authority\":true",
        "missing_data_challenge_truth:true",
        "\"missing_data_challenge_truth\":true",
        "pruning_authority:true",
        "\"pruning_authority\":true",
        "pruning_approved:true",
        "\"pruning_approved\":true",
        "outside_da_truth:true",
        "\"outside_da_truth\":true",
        "outside_chain_da_truth:true",
        "\"outside_chain_da_truth\":true",
        "policy_decision_grants_da_truth",
        "policy_decision_grants_archive_restore",
        "policy_decision_grants_missing_data_challenge",
        "policy_decision_grants_pruning",
        "policy_allow_unlocks_da_material",
        "policy_allow_unlocks_archive_material",
        "unlock_from_da_fallback",
        "unlock_from_archive_restore",
        "paid_from_da",
        "receipt_from_da",
        "balance_from_da",
        "finality_from_da",
        "settlement_from_da",
        "approve_pruning(",
        "prune_history(",
        "trigger_pruning(",
        "accept_da_truth(",
        "accept_archive_proof_as_payment(",
        "mutate_from_da(",
        "settle_from_da(",
        "finalize_from_da(",
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
fn preflight_runner_dynamically_discovers_phase5_da_fallback_policy_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "ron-policy dev-quickchain-preflight.sh");
    }
}
