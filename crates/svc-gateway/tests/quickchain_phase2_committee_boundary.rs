#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 2 Round 2 committee-readiness boundary tests for svc-gateway.
//! RO:WHY — Gateway may route/display backend-derived status later, but must not become committee, quorum, fork-choice, finality, settlement, wallet, ledger, staking, or bridge authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, route source, Cargo.toml.
//! RO:INVARIANTS — gateway is public/proxy/admission only; wallet/ledger truth remains required for paid access.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks committee/quorum/finality header smuggling and validator-economy authority creep.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase2_committee_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

const COMMITTEE_AUTHORITY_HEADERS: &[&str] = &[
    "x-quickchain-committee-member",
    "x-quickchain-committee-signature",
    "x-quickchain-signed-verification-attestation",
    "x-quickchain-quorum-certificate",
    "x-quickchain-quorum-reached",
    "x-quickchain-fork-choice",
    "x-quickchain-finality",
    "x-quickchain-settlement-finality",
    "x-qc-quorum-certificate",
    "x-ron-quickchain-committee-member",
    "x-ron-quickchain-quorum-reached",
    "x-ron-replay-result",
    "x-ron-replay-root",
    "x-ron-verifier-result",
    "x-ron-verifier-attestation",
    "x-ron-committee-attestation",
    "x-ron-committee-signature",
    "x-ron-committee-member",
    "x-ron-quorum",
    "x-ron-quorum-certificate",
    "x-ron-quorum-reached",
    "x-ron-validator-signature",
    "x-ron-validator-set",
    "x-ron-finality",
    "x-ron-finalized",
    "x-ron-anchored",
    "x-ron-settlement-finality",
    "x-ron-external-settlement",
    "x-ron-bridge-settled",
    "x-ron-staking",
    "x-ron-liquidity",
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
fn docs_name_phase2_round2_gateway_committee_readiness_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 2 committee readiness boundary",
        "svc-gateway may route backend-derived verifier/committee status labels if future backend routes expose them",
        "gateway committee status labels are display and routing context only",
        "svc-gateway is not a committee member",
        "svc-gateway does not produce signed verification attestations",
        "svc-gateway does not decide quorum",
        "svc-gateway cannot claim fork choice",
        "svc-gateway cannot claim finality",
        "svc-gateway cannot claim settlement truth",
        "svc-gateway cannot create validator rewards",
        "paid unlock remains wallet/ledger-derived",
        "gateway route labels are display/status only",
        "cache/header/client claims cannot unlock paid content alone",
        "gateway rejects quickchain committee/quorum/finality header smuggling",
        "quickchain_phase2_committee_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_filters_committee_quorum_finality_and_external_quickchain_headers() {
    for raw in COMMITTEE_AUTHORITY_HEADERS {
        let name = HeaderName::from_bytes(raw.as_bytes())
            .unwrap_or_else(|err| panic!("valid header name {raw}: {err}"));

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must not forward Phase 2 Round 2 authority header: {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must not forward Phase 2 Round 2 authority header: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response copy must not endorse Phase 2 Round 2 authority header: {raw}"
        );
    }
}

#[test]
fn gateway_still_allows_backend_product_context_without_promoting_it_to_finality() {
    for raw in [
        "authorization",
        "accept",
        "content-type",
        "x-correlation-id",
        "x-request-id",
        "idempotency-key",
        "x-ron-wallet-account",
        "x-ron-passport",
        "x-ron-wallet-txid",
        "x-ron-wallet-receipt-hash",
        "x-ron-paid-op",
        "x-ron-paid-asset",
    ] {
        let name = HeaderName::from_bytes(raw.as_bytes())
            .unwrap_or_else(|err| panic!("valid product header name {raw}: {err}"));
        assert!(
            proxy::should_forward_product_header(&name),
            "gateway product proxy should still forward non-authority product context header: {raw}"
        );
    }
}

#[test]
fn gateway_routes_do_not_expose_committee_quorum_validator_or_bridge_authority_paths() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

    let forbidden_route_fragments = [
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
fn gateway_does_not_gain_direct_ledger_wallet_validator_or_external_settlement_dependencies() {
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
        "libp2p",
        "tendermint",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-gateway Cargo.toml");
    }
}

#[test]
fn gateway_source_does_not_construct_committee_or_finality_status_truth() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/layers"), &mut files);

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
                &format!("gateway runtime source {}", path.display()),
            );
        }
    }
}
