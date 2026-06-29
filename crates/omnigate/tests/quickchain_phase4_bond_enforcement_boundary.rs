#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 3 controlled bond-enforcement boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate/display backend-derived enforcement metadata, but must not become reserve-slash, release-slash-reserve, capture-slash-reserve, wallet, ledger, paid-unlock, bridge, settlement, staking, or liquidity authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, v1 header policy, v1 route/product hydration source.
//! RO:INVARIANTS — enforcement metadata is display-only; only svc-wallet/ron-ledger own bond mutation truth.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 4 Round 3 controlled-enforcement authority smuggling into hydration or access decisions.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase4_bond_enforcement_boundary.

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

#[test]
fn docs_name_phase4_round3_omnigate_bond_enforcement_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 3 controlled internal bond enforcement boundary",
        "omnigate may hydrate backend-derived bond-enforcement status labels only as non-authoritative metadata",
        "omnigate is not reserve-slash authority",
        "omnigate is not release-slash-reserve authority",
        "omnigate is not capture-slash-reserve authority",
        "omnigate is not bond enforcement truth",
        "bond enforcement status cannot unlock paid content",
        "reserve slash material cannot mutate ledger truth through omnigate",
        "release/capture slash reserve material cannot mutate wallet or ledger truth through omnigate",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "omnigate rejects phase 4 controlled bond-enforcement authority header smuggling",
        "quickchain_phase4_bond_enforcement_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_header_policy_names_phase4_round3_bond_enforcement_families() {
    let source = normalized(&read_rel("src/routes/v1/header_policy.rs"));

    for required in [
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
        assert_contains(&source, required, "omnigate header policy");
    }
}

#[test]
fn omnigate_route_surface_does_not_expose_phase4_round3_bond_enforcement_routes() {
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
                &format!("omnigate route source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_source_does_not_construct_phase4_round3_bond_enforcement_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);
    collect_rust_files(&crate_root().join("src/middleware"), &mut files);

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
        "hydrate_as_bond_enforcement_truth",
        "hydrate_as_reserve_slash_truth",
    ];

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        let compact = compact_without_comments(&source);

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
fn existing_paid_and_access_routes_do_not_unlock_from_bond_enforcement_material() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/streams.rs",
        "src/routes/v1/paid.rs",
        "src/routes/v1/wallet.rs",
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
        assert_contains(&script, required, "omnigate dev-quickchain-preflight.sh");
    }
}
