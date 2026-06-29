#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 3 controlled bond-enforcement boundary tests for svc-gateway.
//! RO:WHY — Gateway may proxy backend-derived enforcement status, but must not become reserve-slash, release-slash-reserve, capture-slash-reserve, wallet, ledger, paid-unlock, bridge, settlement, staking, or liquidity authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, route/admission/header source.
//! RO:INVARIANTS — enforcement metadata is non-authoritative; only svc-wallet/ron-ledger own bond mutation truth.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 4 Round 3 controlled-enforcement authority smuggling through public gateway routes and headers.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase4_bond_enforcement_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

const PHASE4_ROUND3_BOND_ENFORCEMENT_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-bond-enforcement",
    "x-ron-bond-enforcement-decision",
    "x-ron-bond-enforcement-authority",
    "x-ron-bond-enforcement-operation",
    "x-ron-validator-bond-enforcement",
    "x-ron-reserve-slash",
    "x-ron-release-slash-reserve",
    "x-ron-capture-slash-reserve",
    "x-ron-slash-reserve",
    "x-ron-slash-reserved",
    "x-ron-slash-reserve-release",
    "x-ron-slash-reserve-capture",
    "x-ron-bond-reserve",
    "x-ron-bond-capture",
    "x-ron-bond-release",
    "x-ron-controlled-slash",
    "x-ron-controlled-slash-release",
    "x-ron-controlled-slash-capture",
    "x-quickchain-bond-enforcement",
    "x-quickchain-reserve-slash",
    "x-quickchain-capture-slash-reserve",
    "x-qc-bond-enforcement",
    "x-qc-reserve-slash",
    "x-ron-quickchain-bond-enforcement",
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

fn compact_without_comments(text: &str) -> String {
    strip_line_comments(text)
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
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
        "{label} must contain required Phase 4 Round 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 4 Round 3 authority marker: {needle}"
    );
}

fn header(raw: &str) -> HeaderName {
    raw.parse::<HeaderName>()
        .unwrap_or_else(|err| panic!("parse header {raw}: {err}"))
}

#[test]
fn docs_name_phase4_round3_gateway_bond_enforcement_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 3 controlled internal bond enforcement boundary",
        "svc-gateway may route backend-derived bond-enforcement status labels only as non-authoritative metadata",
        "svc-gateway is not reserve-slash authority",
        "svc-gateway is not release-slash-reserve authority",
        "svc-gateway is not capture-slash-reserve authority",
        "svc-gateway is not bond enforcement truth",
        "bond enforcement status cannot unlock paid content",
        "reserve slash material cannot mutate ledger truth through gateway",
        "release/capture slash reserve material cannot mutate wallet or ledger truth through gateway",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "svc-gateway rejects phase 4 controlled bond-enforcement authority header smuggling",
        "quickchain_phase4_bond_enforcement_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_filters_phase4_round3_bond_enforcement_authority_headers() {
    for raw in PHASE4_ROUND3_BOND_ENFORCEMENT_AUTHORITY_HEADERS {
        let name = header(raw);

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must reject Phase 4 Round 3 authority header {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must reject Phase 4 Round 3 authority header {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response proxy must reject Phase 4 Round 3 authority header {raw}"
        );
    }
}

#[test]
fn gateway_header_policy_names_phase4_round3_bond_enforcement_families() {
    let source = normalized(&read_rel("src/headers/proxy.rs"));

    for required in [
        "is_phase4_round3_bond_enforcement_authority_header",
        "\"x-ron-bond-enforcement\"",
        "\"x-ron-bond-enforcement-decision\"",
        "\"x-ron-bond-enforcement-authority\"",
        "\"x-ron-validator-bond-enforcement\"",
        "\"x-ron-reserve-slash\"",
        "\"x-ron-release-slash-reserve\"",
        "\"x-ron-capture-slash-reserve\"",
        "\"x-ron-slash-reserve\"",
        "\"x-ron-slash-reserved\"",
        "\"x-ron-controlled-slash\"",
        "raw.starts_with(\"x-ron-reserve-slash-\")",
        "raw.starts_with(\"x-ron-release-slash-reserve\")",
        "raw.starts_with(\"x-ron-capture-slash-reserve\")",
        "raw.starts_with(\"x-ron-slash-reserve-\")",
        "raw.starts_with(\"x-ron-controlled-slash-\")",
    ] {
        assert_contains(&source, required, "svc-gateway header policy");
    }
}

#[test]
fn gateway_route_surface_does_not_expose_phase4_round3_bond_enforcement_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/bond-enforcement",
        ".route(\"/reserve-slash",
        ".route(\"/release-slash-reserve",
        ".route(\"/capture-slash-reserve",
        ".route(\"/slash-reserve",
        ".route(\"/controlled-slash",
        ".route(\"/validator-bond-enforcement",
        ".route(\"/public-staking",
        ".route(\"/staking-market",
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
fn gateway_source_does_not_construct_phase4_round3_bond_enforcement_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/layers"), &mut files);

    let forbidden_compact_markers = [
        "bond_enforcement_authority:true",
        "\"bond_enforcement_authority\":true",
        "reserve_slash_authority:true",
        "\"reserve_slash_authority\":true",
        "release_slash_reserve_authority:true",
        "\"release_slash_reserve_authority\":true",
        "capture_slash_reserve_authority:true",
        "\"capture_slash_reserve_authority\":true",
        "execute_bond_enforcement(",
        "commit_bond_enforcement(",
        "apply_bond_enforcement(",
        "reserve_slash(",
        "release_slash_reserve(",
        "capture_slash_reserve(",
        "execute_reserve_slash(",
        "execute_release_slash_reserve(",
        "execute_capture_slash_reserve(",
        "bond_enforcement_wallet_mutation",
        "bond_enforcement_ledger_mutation",
        "paid_unlock_from_bond_enforcement",
        "unlock_from_slash_reserve",
        "unlock_from_reserve_slash",
        "unlock_from_capture_slash_reserve",
        "receipt_from_bond_enforcement",
        "balance_from_bond_enforcement",
        "finality_from_bond_enforcement",
        "gateway_bond_enforcement_truth",
        "gateway_reserve_slash_truth",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

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
fn existing_paid_and_product_routes_do_not_unlock_from_bond_enforcement_material() {
    for rel in [
        "src/routes/paid_storage.rs",
        "src/routes/product.rs",
        "src/routes/app.rs",
        "src/routes/objects.rs",
        "src/routes/objects_range.rs",
    ] {
        let path = crate_root().join(rel);
        if !path.exists() {
            continue;
        }

        let source = normalized(&strip_line_comments(&read_rel(rel)));

        for forbidden in [
            "unlock_from_bond_enforcement",
            "unlock_from_reserve_slash",
            "unlock_from_slash_reserve",
            "unlock_from_capture_slash_reserve",
            "paid_by_bond_enforcement",
            "receipt_from_bond_enforcement",
            "balance_from_bond_enforcement",
            "finality_from_bond_enforcement",
            "cache_unlock_from_bond_enforcement",
            "policy_unlock_from_bond_enforcement",
            "bond_enforcement_paid_unlock_authority",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn preflight_runner_names_phase4_round3_bond_enforcement_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "quickchain_phase4_bond_enforcement_boundary",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-gateway dev-quickchain-preflight.sh");
    }
}
