//! RO:WHAT — Phase 2 Round 1 read-only verifier artifact boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate/display backend-derived replay metadata but must not become verifier/finality authority.
//! RO:INTERACTS — docs/quickchain-preflight.md and v1 route/product hydration source.
//! RO:INVARIANTS — hydration is backend-derived; replay/proof artifacts are display-only unless future gates authorize more.
//! RO:METRICS — none; source/docs boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks replay/proof/verifier/quorum/committee authority creep in product hydration.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase2_replay_boundary.

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
fn docs_name_phase2_round1_omnigate_read_only_replay_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 2 round 1 verifier artifact / read-only replication",
        "omnigate may expose read-only proof/replay artifact views if needed",
        "omnigate replay metadata is display and hydration context only",
        "omnigate is not verifier truth",
        "omnigate is not replay truth",
        "omnigate is not quorum truth",
        "omnigate is not committee truth",
        "omnigate does not sign verifier attestations",
        "omnigate does not decide fork choice",
        "omnigate does not claim finality",
        "omnigate cannot unlock paid content from replay artifacts alone",
        "paid unlock still requires backend wallet/ledger truth",
        "quickchain_phase2_replay_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_routes_do_not_expose_verifier_committee_or_finality_authority_paths() {
    let routes = normalized(&strip_line_comments(&read_rel("src/routes/v1/mod.rs")));

    for forbidden in [
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
    ] {
        assert_not_contains(&routes, forbidden, "omnigate v1 route registry");
    }
}

#[test]
fn omnigate_source_does_not_construct_phase2_authority_results() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes/v1"), &mut files);

    let forbidden_compact_markers = [
        "verifier_result:",
        "\"verifier_result\":",
        "replay_result:",
        "\"replay_result\":",
        "committee_attestation:",
        "\"committee_attestation\":",
        "quorum_reached:true",
        "\"quorum_reached\":true",
        "fork_choice_winner:true",
        "\"fork_choice_winner\":true",
        "finalized:true",
        "\"finalized\":true",
        "anchored:true",
        "\"anchored\":true",
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
                &format!("omnigate route source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_does_not_gain_direct_ledger_or_external_settlement_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "anchor-lang",
        "spl-token",
        "solana-client",
        "solana-sdk",
        "ethers",
        "web3",
    ] {
        assert_not_contains(&cargo, forbidden, "omnigate Cargo.toml");
    }
}

#[test]
fn existing_paid_routes_remain_backend_wallet_ledger_derived_not_replay_artifact_unlocks() {
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
            "unlock_from_replay_artifact",
            "unlock_from_proof_artifact",
            "unlock_from_verifier_result",
            "unlock_from_committee_attestation",
            "paid_by_replay_result",
            "paid_by_proof_result",
            "paid_by_quorum",
            "paid_by_finality_header",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}
