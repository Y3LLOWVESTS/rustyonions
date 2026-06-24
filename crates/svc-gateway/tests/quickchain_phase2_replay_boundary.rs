//! RO:WHAT — Phase 2 Round 1 read-only verifier artifact boundary tests for svc-gateway.
//! RO:WHY — Public gateway may route/display replay metadata but must not become verifier/finality/settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, route source, Cargo.toml.
//! RO:INVARIANTS — gateway is public/proxy/admission only; backend wallet/ledger truth remains required for paid access.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks replay/proof/verifier/quorum/committee authority smuggling through gateway.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase2_replay_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

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
        "{label} must contain required Phase 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 2 authority marker: {needle}"
    );
}

#[test]
fn docs_name_phase2_round1_gateway_read_only_replay_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 1 verifier artifact / read-only replication",
        "svc-gateway may expose read-only proof/replay artifact routes if needed",
        "gateway replay metadata is display and routing context only",
        "gateway is not verifier truth",
        "gateway is not replay truth",
        "gateway is not quorum truth",
        "gateway is not committee truth",
        "gateway does not sign verifier attestations",
        "gateway does not decide fork choice",
        "gateway does not claim finality",
        "gateway cannot unlock paid content from replay artifacts alone",
        "paid unlock still requires backend wallet/ledger truth",
        "quickchain_phase2_replay_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn verifier_replay_headers_are_transport_metadata_not_gateway_authority() {
    for raw in [
        "x-ron-quickchain-replay-bundle",
        "x-ron-quickchain-replay-result",
        "x-ron-proof-artifact",
        "x-ron-proof-root",
        "x-ron-replay-result",
        "x-ron-replay-root",
        "x-ron-verifier-result",
        "x-ron-verifier-attestation",
        "x-ron-committee-attestation",
        "x-ron-quorum",
        "x-ron-root-proof",
        "x-ron-checkpoint-proof",
        "x-ron-finality",
        "x-ron-finalized",
        "x-ron-anchored",
    ] {
        let name = HeaderName::from_static(raw);

        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must not forward Phase 2 authority-shaped header: {raw}"
        );
        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must not forward Phase 2 authority-shaped header: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response copy must not endorse Phase 2 authority-shaped header: {raw}"
        );
    }
}

#[test]
fn gateway_routes_do_not_expose_verifier_committee_or_finality_authority_paths() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/quickchain",
        ".route(\"/replay/commit",
        ".route(\"/replay/finalize",
        ".route(\"/verifier",
        ".route(\"/committee",
        ".route(\"/quorum",
        ".route(\"/fork-choice",
        ".route(\"/checkpoint",
        ".route(\"/validator",
        ".route(\"/bridge",
        ".route(\"/anchor",
        ".route(\"/staking",
        ".route(\"/liquidity",
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
                &format!("gateway route source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_does_not_gain_direct_ledger_wallet_or_external_settlement_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "anchor-lang",
        "spl-token",
        "solana-client",
        "solana-sdk",
        "ethers",
        "web3",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-gateway Cargo.toml");
    }
}

#[test]
fn gateway_source_does_not_construct_phase2_authority_status() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    let forbidden_compact_markers = [
        "finalized:true",
        "\"finalized\":true",
        "anchored:true",
        "\"anchored\":true",
        "epoch_included:true",
        "\"epoch_included\":true",
        "quorum_reached:true",
        "\"quorum_reached\":true",
        "committee_signed:true",
        "\"committee_signed\":true",
        "fork_choice_winner:true",
        "\"fork_choice_winner\":true",
        "settlement_finality:true",
        "\"settlement_finality\":true",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = source
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();

        for forbidden in forbidden_compact_markers {
            assert_not_contains(
                &compact,
                forbidden,
                &format!("gateway source {}", path.display()),
            );
        }
    }
}
