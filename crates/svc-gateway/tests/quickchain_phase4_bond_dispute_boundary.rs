#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 2 bond dispute/challenge simulation boundary tests for svc-gateway.
//! RO:WHY — Gateway may proxy/display backend-derived dispute metadata, but must not become dispute, appeal, freeze, irreversible slash, wallet, ledger, paid-unlock, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, route/admission/header source.
//! RO:INVARIANTS — dispute metadata is non-authoritative; challenge windows are explicit; no live irreversible slash; paid unlock remains backend wallet/ledger receipt derived.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 4 Round 2 dispute/challenge/appeal/freeze authority smuggling through public gateway routes and headers.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase4_bond_dispute_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

const PHASE4_ROUND2_DISPUTE_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-bond-dispute",
    "x-ron-bond-dispute-state",
    "x-ron-dispute",
    "x-ron-dispute-window",
    "x-ron-challenge",
    "x-ron-challenge-window",
    "x-ron-appeal",
    "x-ron-appeal-window",
    "x-ron-freeze",
    "x-ron-frozen-bond",
    "x-ron-disputed-bond",
    "x-ron-irreversible-slash",
    "x-ron-slash-appeal",
    "x-ron-slash-challenge",
    "x-ron-slash-simulation",
    "x-quickchain-bond-dispute",
    "x-quickchain-challenge-window",
    "x-qc-bond-dispute",
    "x-qc-slash-simulation",
    "x-ron-quickchain-bond-dispute",
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
        "{label} must contain required Phase 4 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 4 Round 2 authority marker: {needle}"
    );
}

fn header(raw: &str) -> HeaderName {
    raw.parse::<HeaderName>()
        .unwrap_or_else(|err| panic!("parse header {raw}: {err}"))
}

#[test]
fn docs_name_phase4_round2_gateway_dispute_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 2 slashing/challenge simulation boundary",
        "svc-gateway may route backend-derived disputed-bond status labels only as non-authoritative metadata",
        "svc-gateway is not dispute truth",
        "svc-gateway is not challenge-window truth",
        "svc-gateway is not appeal authority",
        "svc-gateway is not freeze authority",
        "svc-gateway is not irreversible slash authority",
        "dispute status cannot unlock paid content",
        "challenge evidence cannot mutate ledger truth through gateway",
        "appeal/freeze state cannot mutate wallet or ledger truth through gateway",
        "no live irreversible slash through svc-gateway",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "svc-gateway rejects phase 4 dispute/challenge/appeal/freeze/slash-simulation authority header smuggling",
        "quickchain_phase4_bond_dispute_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_filters_phase4_round2_dispute_challenge_appeal_and_freeze_authority_headers() {
    for raw in PHASE4_ROUND2_DISPUTE_AUTHORITY_HEADERS {
        let name = header(raw);

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "svc-gateway must not forward Phase 4 Round 2 authority passthrough header: {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "svc-gateway must not forward Phase 4 Round 2 authority product header: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "svc-gateway must not copy Phase 4 Round 2 authority response header: {raw}"
        );
    }
}

#[test]
fn gateway_header_filter_names_phase4_round2_dispute_families() {
    let source = normalized(&read_rel("src/headers/proxy.rs"));

    for required in [
        "\"x-ron-bond-dispute\"",
        "\"x-ron-bond-dispute-state\"",
        "\"x-ron-dispute\"",
        "\"x-ron-dispute-window\"",
        "\"x-ron-challenge-window\"",
        "\"x-ron-appeal-window\"",
        "\"x-ron-freeze\"",
        "\"x-ron-frozen-bond\"",
        "\"x-ron-disputed-bond\"",
        "\"x-ron-irreversible-slash\"",
        "\"x-ron-slash-simulation\"",
        "raw.starts_with(\"x-ron-dispute-\")",
        "raw.starts_with(\"x-ron-disputed-\")",
        "raw.starts_with(\"x-ron-challenge-\")",
        "raw.starts_with(\"x-ron-appeal-\")",
        "raw.starts_with(\"x-ron-freeze-\")",
        "raw.starts_with(\"x-ron-frozen-\")",
        "raw.starts_with(\"x-ron-irreversible-slash\")",
        "raw.starts_with(\"x-ron-slash-simulation\")",
    ] {
        assert_contains(&source, required, "svc-gateway header proxy policy");
    }
}

#[test]
fn gateway_route_surface_does_not_expose_phase4_round2_dispute_or_challenge_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/bond-dispute",
        ".route(\"/dispute",
        ".route(\"/disputes",
        ".route(\"/challenge",
        ".route(\"/challenges",
        ".route(\"/appeal",
        ".route(\"/appeals",
        ".route(\"/freeze",
        ".route(\"/frozen-bond",
        ".route(\"/irreversible-slash",
        ".route(\"/slash-simulation",
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
fn gateway_source_does_not_construct_phase4_round2_dispute_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/headers"), &mut files);

    let forbidden_compact_markers = [
        "dispute_authority:true",
        "\"dispute_authority\":true",
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
        "execute_dispute(",
        "open_challenge_window(",
        "submit_appeal(",
        "freeze_bond(",
        "capture_disputed_bond(",
        "slash_disputed_bond(",
        "execute_irreversible_slash(",
        "commit_irreversible_slash(",
        "apply_irreversible_slash(",
        "mutate_disputed_bond(",
        "resolve_slash_live(",
        "slash_without_governance(",
        "bond_dispute_wallet_mutation",
        "bond_dispute_ledger_mutation",
        "paid_unlock_from_dispute",
        "unlock_from_challenge",
        "unlock_from_appeal",
        "unlock_from_freeze",
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
fn preflight_runner_names_phase4_round2_dispute_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "quickchain_phase4_bond_dispute_boundary",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-gateway dev-quickchain-preflight.sh");
    }
}
