#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 4 Round 1 bond/slash/stake/liquidity boundary tests for svc-gateway.
//! RO:WHY — Gateway may proxy/display backend-derived bond metadata, but must not become bond, slash, staking, liquidity, wallet, ledger, paid-unlock, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, route/admission/product source.
//! RO:INVARIANTS — bond metadata is non-authoritative; no live slashing; no public staking market; paid unlock remains backend wallet/ledger receipt derived.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 4 economic-authority smuggling through public gateway routes and headers.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase4_bond_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

const PHASE4_BOND_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-bond",
    "x-ron-bond-account",
    "x-ron-bond-intent",
    "x-ron-bond-lifecycle",
    "x-ron-bond-lifecycle-decision",
    "x-ron-bond-authority",
    "x-ron-validator-bond",
    "x-ron-bonded-stake",
    "x-ron-slash",
    "x-ron-slashing",
    "x-ron-slash-evidence",
    "x-ron-slash-decision",
    "x-ron-stake",
    "x-ron-staking",
    "x-ron-liquidity",
    "x-quickchain-bond",
    "x-quickchain-slash-evidence",
    "x-qc-bond-lifecycle-decision",
    "x-qc-slash-decision",
    "x-ron-quickchain-bond-account",
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

fn header(raw: &str) -> HeaderName {
    raw.parse::<HeaderName>()
        .unwrap_or_else(|err| panic!("parse header {raw}: {err}"))
}

#[test]
fn docs_name_phase4_round1_gateway_bond_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 1 bonded validator model boundary",
        "svc-gateway may route backend-derived bond status labels only as non-authoritative metadata",
        "svc-gateway is not bond truth",
        "svc-gateway is not slash truth",
        "svc-gateway is not slashing authority",
        "svc-gateway is not staking market authority",
        "svc-gateway is not liquidity authority",
        "bond status cannot unlock paid content",
        "slash evidence cannot mutate ledger truth through gateway",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "svc-gateway rejects phase 4 bond/slash/stake/liquidity authority header smuggling",
        "quickchain_phase4_bond_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_filters_phase4_bond_slash_stake_and_liquidity_authority_headers() {
    for raw in PHASE4_BOND_AUTHORITY_HEADERS {
        let name = header(raw);

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "svc-gateway must not forward Phase 4 authority passthrough header: {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "svc-gateway must not forward Phase 4 authority product header: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "svc-gateway must not copy Phase 4 authority response header: {raw}"
        );
    }
}

#[test]
fn gateway_keeps_current_backend_derived_product_context_headers_but_not_phase4_authority() {
    for raw in [
        "authorization",
        "accept",
        "content-type",
        "x-correlation-id",
        "x-request-id",
        "idempotency-key",
        "x-ron-wallet-account",
        "x-ron-wallet-receipt-hash",
        "x-ron-paid-op",
        "x-ron-paid-asset",
    ] {
        let name = header(raw);
        assert!(
            proxy::should_forward_product_header(&name),
            "svc-gateway should still forward backend-derived product context header: {raw}"
        );
    }

    for raw in [
        "x-ron-bond-account",
        "x-ron-slash-evidence",
        "x-ron-staking",
        "x-ron-liquidity",
    ] {
        let name = header(raw);
        assert!(
            !proxy::should_forward_product_header(&name),
            "svc-gateway must reject Phase 4 authority-shaped product header: {raw}"
        );
    }
}

#[test]
fn gateway_header_filter_names_phase4_bond_and_slash_families() {
    let source = normalized(&read_rel("src/headers/proxy.rs"));

    for required in [
        "\"x-ron-bond\"",
        "\"x-ron-bond-account\"",
        "\"x-ron-bond-intent\"",
        "\"x-ron-slash-evidence\"",
        "\"x-ron-slash-decision\"",
        "\"x-ron-stake\"",
        "\"x-ron-staking\"",
        "\"x-ron-liquidity\"",
        "raw.starts_with(\"x-ron-bond-\")",
        "raw.starts_with(\"x-ron-slash-\")",
        "raw.starts_with(\"x-ron-stake-\")",
    ] {
        assert_contains(&source, required, "svc-gateway header proxy policy");
    }
}

#[test]
fn gateway_route_surface_does_not_expose_phase4_bond_slash_stake_or_liquidity_routes() {
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
                &format!("svc-gateway route source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_source_does_not_construct_phase4_bond_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/headers"), &mut files);

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
                &format!("svc-gateway source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_manifest_does_not_add_phase4_runtime_settlement_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
        "solana-sdk",
        "solana-client",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-gateway Cargo.toml");
    }
}
