#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 4 Round 1 bond/slash/stake/liquidity boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate/display backend-derived bond metadata, but must not become bond, slash, staking, liquidity, wallet, ledger, paid-unlock, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, v1 header policy, v1 route/product hydration source.
//! RO:INVARIANTS — bond metadata is display-only; no live slashing; no public staking market; paid unlock remains backend wallet/ledger receipt derived.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 4 economic-authority smuggling into hydration or access decisions.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase4_bond_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

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

#[test]
fn docs_name_phase4_round1_omnigate_bond_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 1 bonded validator model boundary",
        "omnigate may hydrate backend-derived bond status labels only as non-authoritative metadata",
        "omnigate is not bond truth",
        "omnigate is not slash truth",
        "omnigate is not slashing authority",
        "omnigate is not staking market authority",
        "omnigate is not liquidity authority",
        "bond status cannot unlock paid content",
        "slash evidence cannot mutate ledger truth through omnigate",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "omnigate rejects phase 4 bond/slash/stake/liquidity authority header smuggling",
        "quickchain_phase4_bond_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_header_policy_names_phase4_bond_and_slash_families() {
    let source = normalized(&read_rel("src/routes/v1/header_policy.rs"));

    for required in [
        "\"x-ron-bond\"",
        "\"x-ron-bond-account\"",
        "\"x-ron-bond-intent\"",
        "\"x-ron-bond-lifecycle\"",
        "\"x-ron-bond-lifecycle-decision\"",
        "\"x-ron-bond-authority\"",
        "\"x-ron-validator-bond\"",
        "\"x-ron-bonded-stake\"",
        "\"x-ron-slash\"",
        "\"x-ron-slashing\"",
        "\"x-ron-slash-evidence\"",
        "\"x-ron-slash-decision\"",
        "\"x-ron-stake\"",
        "\"x-ron-staking\"",
        "\"x-ron-liquidity\"",
        "raw.starts_with(\"x-ron-bond-\")",
        "raw.starts_with(\"x-ron-slash-\")",
        "raw.starts_with(\"x-ron-stake-\")",
    ] {
        assert_contains(&source, required, "omnigate header policy");
    }
}

#[test]
fn omnigate_route_surface_does_not_expose_phase4_bond_slash_stake_or_liquidity_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/quickchain",
        ".route(\"/bond",
        ".route(\"/bonds",
        ".route(\"/validator-bond",
        ".route(\"/slash",
        ".route(\"/slashing",
        ".route(\"/stake",
        ".route(\"/staking",
        ".route(\"/liquidity",
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
fn omnigate_source_does_not_construct_phase4_bond_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);

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
                &format!("omnigate source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_runtime_source_does_not_import_phase4_settlement_or_external_chain_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    for path in files {
        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read source {}: {err}", path.display())),
        ));

        for forbidden in [
            "ron_ledger::",
            "quickchain::bond",
            "quickchain::slash",
            "anchor_lang",
            "spl_token",
            "solana_sdk",
            "solana_client",
            "ethers::",
            "web3::",
        ] {
            assert_not_contains(
                &source,
                forbidden,
                &format!("omnigate runtime source {}", path.display()),
            );
        }
    }
}
