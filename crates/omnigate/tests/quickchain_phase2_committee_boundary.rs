#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 2 Round 2 committee-readiness boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate/display backend-derived status later, but must not become committee, quorum, fork-choice, finality, settlement, wallet, ledger, staking, or bridge authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, routes/v1/header_policy.rs, v1 route/product hydration source, Cargo.toml.
//! RO:INVARIANTS — hydration is backend-derived display only; wallet/ledger truth remains required for paid access.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks committee/quorum/finality header smuggling and validator-economy authority creep.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase2_committee_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

const COMMITTEE_AUTHORITY_HEADER_MARKERS: &[&str] = &[
    "\"x-ron-replay-result\"",
    "\"x-ron-replay-root\"",
    "\"x-ron-verifier-result\"",
    "\"x-ron-verifier-attestation\"",
    "\"x-ron-committee-attestation\"",
    "\"x-ron-committee-signature\"",
    "\"x-ron-committee-member\"",
    "\"x-ron-quorum\"",
    "\"x-ron-quorum-certificate\"",
    "\"x-ron-quorum-reached\"",
    "\"x-ron-validator-signature\"",
    "\"x-ron-validator-set\"",
    "\"x-ron-finality\"",
    "\"x-ron-finalized\"",
    "\"x-ron-anchored\"",
    "\"x-ron-external-settlement\"",
    "raw.starts_with(\"x-quickchain-\")",
    "raw.starts_with(\"x-qc-\")",
    "raw.starts_with(\"x-ron-replay-\")",
    "raw.starts_with(\"x-ron-verifier-\")",
    "raw.starts_with(\"x-ron-committee-\")",
    "raw.starts_with(\"x-ron-quorum-\")",
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
        "{label} must contain required Phase 2 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 2 Round 2 authority marker: {needle}"
    );
}

#[test]
fn docs_name_phase2_round2_omnigate_committee_readiness_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 2 committee readiness boundary",
        "omnigate may hydrate backend-derived verifier/committee status labels if future backend routes expose them",
        "omnigate committee status labels are display and hydration context only",
        "omnigate is not a committee member",
        "omnigate does not produce signed verification attestations",
        "omnigate does not decide quorum",
        "omnigate cannot claim fork choice",
        "omnigate cannot claim finality",
        "omnigate cannot claim settlement truth",
        "omnigate quote/access coordination is not settlement truth",
        "hydration is backend-derived display only",
        "paid unlock remains wallet/ledger-derived",
        "cache/header/client claims cannot unlock paid content alone",
        "omnigate rejects quickchain committee/quorum/finality header smuggling",
        "quickchain_phase2_committee_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_header_policy_blocks_committee_quorum_finality_and_external_quickchain_headers() {
    let policy = read_rel("src/routes/v1/header_policy.rs");

    for required in COMMITTEE_AUTHORITY_HEADER_MARKERS {
        assert_contains(
            &policy,
            required,
            "omnigate v1 header_policy.rs committee authority denylist",
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
fn omnigate_routes_do_not_expose_committee_quorum_validator_or_bridge_authority_paths() {
    let routes = normalized(&strip_line_comments(&read_rel("src/routes/v1/mod.rs")));

    for forbidden in [
        ".route(\"/quickchain",
        ".route(\"/committee",
        ".route(\"/quorum",
        ".route(\"/fork-choice",
        ".route(\"/finality",
        ".route(\"/validator",
        ".route(\"/checkpoint",
        ".route(\"/bridge",
        ".route(\"/anchor",
        ".route(\"/staking",
        ".route(\"/slashing",
        ".route(\"/liquidity",
        ".route(\"/solana",
        ".route(\"/rox",
    ] {
        assert_not_contains(&routes, forbidden, "omnigate v1 route registry");
    }
}

#[test]
fn omnigate_does_not_gain_direct_ledger_validator_or_external_settlement_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "anchor-lang",
        "spl-token",
        "solana-client",
        "solana-sdk",
        "ethers",
        "web3",
        "libp2p",
        "tendermint",
    ] {
        assert_not_contains(&cargo, forbidden, "omnigate Cargo.toml");
    }
}

#[test]
fn omnigate_hydration_and_paid_routes_do_not_construct_committee_or_finality_status_truth() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes/v1"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);

    let forbidden_compact_markers = [
        "quorum_reached:true",
        "\"quorum_reached\":true",
        "committee_signed:true",
        "\"committee_signed\":true",
        "committee_member:true",
        "\"committee_member\":true",
        "fork_choice_winner:true",
        "\"fork_choice_winner\":true",
        "settlement_finality:true",
        "\"settlement_finality\":true",
        "bridge_finality:true",
        "\"bridge_finality\":true",
        "stake_weight:",
        "\"stake_weight\":",
        "slash_evidence:",
        "\"slash_evidence\":",
        "validator_reward:",
        "\"validator_reward\":",
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
fn existing_paid_routes_remain_backend_wallet_ledger_derived_not_committee_unlocks() {
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
            "unlock_from_committee_attestation",
            "unlock_from_quorum_certificate",
            "unlock_from_quorum",
            "unlock_from_finality",
            "unlock_from_fork_choice",
            "unlock_from_validator_signature",
            "paid_by_committee_attestation",
            "paid_by_quorum_certificate",
            "paid_by_finality_header",
            "cache_unlock_authority",
            "cache_only_unlock",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}
