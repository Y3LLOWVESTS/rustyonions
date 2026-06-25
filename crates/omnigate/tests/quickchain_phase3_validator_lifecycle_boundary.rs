#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle hardening boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate/display backend-derived lifecycle metadata, but must not become lifecycle, governance, paid-unlock, wallet, ledger, finality, staking, slashing, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, v1 header policy, v1 route/product hydration source.
//! RO:INVARIANTS — lifecycle/evidence metadata is non-authoritative; paid unlock remains backend wallet/ledger receipt derived.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks validator lifecycle/evidence/governance authority smuggling into hydration or access decisions.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase3_validator_lifecycle_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

const LIFECYCLE_HEADER_POLICY_MARKERS: &[&str] = &[
    "\"x-ron-governance-parameter-update\"",
    "\"x-ron-governance-approval\"",
    "\"x-ron-validator-lifecycle-decision\"",
    "\"x-ron-lifecycle-decision\"",
    "raw.starts_with(\"x-ron-governance-\")",
    "raw.starts_with(\"x-ron-lifecycle-\")",
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
        "{label} must contain required Phase 3 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 3 Round 2 authority marker: {needle}"
    );
}

#[test]
fn docs_name_phase3_round2_omnigate_validator_lifecycle_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 2 validator lifecycle boundary",
        "omnigate may hydrate backend-derived lifecycle status labels only as non-authoritative metadata",
        "omnigate is product hydration / access composition / backend-derived display surface only",
        "omnigate is not validator lifecycle authority",
        "omnigate is not governance parameter-update authority",
        "omnigate is not validator rotation authority",
        "omnigate is not validator revocation authority",
        "omnigate is not validator downtime authority",
        "omnigate is not validator equivocation authority",
        "omnigate is not replay challenge authority",
        "validator rotation, revocation, downtime, equivocation evidence, replay challenge evidence, and governance-gated parameter updates cannot unlock paid content",
        "lifecycle/evidence material cannot mint, transfer, burn, hold, capture, release, issue receipts, or mutate ledger truth",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "validator lifecycle data cannot replace wallet/ledger truth",
        "omnigate rejects phase 3 validator lifecycle/evidence/governance header smuggling",
        "quickchain_phase3_validator_lifecycle_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_header_policy_blocks_lifecycle_evidence_and_governance_authority() {
    let policy = read_rel("src/routes/v1/header_policy.rs");

    for required in LIFECYCLE_HEADER_POLICY_MARKERS {
        assert_contains(
            &policy,
            required,
            "omnigate v1 header_policy.rs Phase 3 Round 2 lifecycle/governance denylist",
        );
    }

    for allowed_context in [
        "x-ron-wallet-account",
        "x-ron-passport",
        "x-ron-wallet-txid",
        "x-ron-wallet-receipt-hash",
        "x-ron-paid-op",
        "x-ron-paid-asset",
    ] {
        assert_contains(
            &policy,
            allowed_context,
            "omnigate v1 header_policy.rs product context allowlist docs",
        );
    }
}

#[test]
fn omnigate_routes_do_not_expose_validator_lifecycle_mutation_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes/v1"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/quickchain",
        ".route(\"/validator-rotation",
        ".route(\"/validator-revocation",
        ".route(\"/validator-downtime",
        ".route(\"/validator-equivocation",
        ".route(\"/validator-lifecycle",
        ".route(\"/replay-challenge",
        ".route(\"/governance-parameter",
        ".route(\"/finality",
        ".route(\"/staking",
        ".route(\"/slashing",
        ".route(\"/bridge",
        ".route(\"/external-settlement",
        ".route(\"/solana",
        ".route(\"/rox",
    ];

    for path in route_files {
        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read route source {}: {err}", path.display())),
        ));

        for forbidden in forbidden_route_fragments {
            assert_not_contains(
                &source,
                forbidden,
                &format!("omnigate route source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_source_does_not_construct_validator_lifecycle_or_paid_unlock_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes/v1"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/middleware"), &mut files);

    let forbidden_compact_markers = [
        "validator_lifecycle_authority:true",
        "\"validator_lifecycle_authority\":true",
        "validator_rotation_authority:true",
        "\"validator_rotation_authority\":true",
        "validator_revocation_authority:true",
        "\"validator_revocation_authority\":true",
        "validator_downtime_authority:true",
        "\"validator_downtime_authority\":true",
        "validator_equivocation_authority:true",
        "\"validator_equivocation_authority\":true",
        "replay_challenge_authority:true",
        "\"replay_challenge_authority\":true",
        "governance_parameter_update_authority:true",
        "\"governance_parameter_update_authority\":true",
        "unlock_from_validator_lifecycle",
        "unlock_from_validator_rotation",
        "unlock_from_validator_revocation",
        "unlock_from_validator_evidence",
        "unlock_from_replay_challenge",
        "paid_by_validator_lifecycle",
        "receipt_from_validator_lifecycle",
        "balance_from_validator_lifecycle",
        "ledger_from_validator_lifecycle",
        "finality_from_validator_lifecycle",
        "settlement_from_validator_lifecycle",
        "admit_validator(",
        "revoke_validator(",
        "rotate_validator(",
        "slash_validator(",
        "stake_validator(",
        "validator_reward(",
        "bridge_settlement(",
        "external_settlement(",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = strip_line_comments(&source)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("omnigate runtime source {}", path.display()),
            );
        }
    }
}

#[test]
fn existing_paid_and_access_routes_do_not_unlock_from_lifecycle_evidence() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/streams.rs",
        "src/routes/v1/paid.rs",
        "src/routes/v1/wallet.rs",
    ] {
        let source = normalized(&strip_line_comments(&read_rel(rel)));

        for forbidden in [
            "unlock_from_validator_lifecycle",
            "unlock_from_validator_rotation",
            "unlock_from_validator_revocation",
            "unlock_from_validator_evidence",
            "unlock_from_replay_challenge",
            "paid_by_validator_lifecycle",
            "cache_unlock_authority",
            "cache_only_unlock",
            "validator_lifecycle_receipt_truth",
            "validator_lifecycle_balance_truth",
            "validator_lifecycle_finality_truth",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn omnigate_source_does_not_import_ledger_or_external_settlement_runtime() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    for path in files {
        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read source {}: {err}", path.display())),
        ));

        for forbidden in [
            "use ron_ledger",
            "ron_ledger::",
            "ledger_commit",
            "mutate_ledger",
            "mutate_balance",
            "validator_bond",
            "validator_stake",
            "validator_slash",
            "solana_client",
            "spl_token",
            "anchor_lang",
        ] {
            assert_not_contains(
                &source,
                forbidden,
                &format!("omnigate source {}", path.display()),
            );
        }
    }
}
