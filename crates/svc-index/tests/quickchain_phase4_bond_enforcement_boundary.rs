#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 3 controlled bond-enforcement boundary tests for svc-index.
//! RO:WHY — Index may point to bond-enforcement and slash-reserve artifacts, but pointers remain lookup truth only, never reserve-slash, release-slash-reserve, capture-slash-reserve, finality, settlement, or payment authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, route/source scanner, Cargo manifest.
//! RO:INVARIANTS — pointers are references; b3 proves bytes only; names/crab navigation are not authority; no paid unlock or validator consequence from index metadata.
//! RO:METRICS — none; docs/source boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents index/pointer metadata from becoming Phase 4 Round 3 controlled bond-enforcement authority.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase4_bond_enforcement_boundary.

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
    if !root.exists() {
        return;
    }

    if root.is_file() {
        if root.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(root.to_path_buf());
        }
        return;
    }

    if !root.is_dir() {
        return;
    }

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
fn docs_name_phase4_round3_index_bond_enforcement_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 3 controlled bond enforcement index boundary",
        "svc-index may point to bond-enforcement/slash-reserve artifacts only as references",
        "svc-index is not bond enforcement truth",
        "svc-index is not reserve-slash authority",
        "svc-index is not release-slash-reserve authority",
        "svc-index is not capture-slash-reserve authority",
        "index pointer cannot reserve slash material",
        "index pointer cannot release slash reserve",
        "index pointer cannot capture slash reserve",
        "index pointer cannot unlock paid content from bond enforcement",
        "bond enforcement artifact cid proves bytes only",
        "slash reserve artifact cid proves bytes only",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase4_bond_enforcement_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn index_route_surface_does_not_expose_phase4_round3_bond_enforcement_routes() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/http/routes"), &mut route_files);
    collect_rust_files(&crate_root().join("src/router.rs"), &mut route_files);

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
                &format!("svc-index route source {}", path.display()),
            );
        }
    }
}

#[test]
fn index_source_does_not_construct_phase4_round3_bond_enforcement_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    collect_rust_files(&crate_root().join("examples"), &mut files);

    let forbidden_compact_markers = [
        "bond_enforcement_authority:true",
        "\"bond_enforcement_authority\":true",
        "bond_enforcement_truth:true",
        "\"bond_enforcement_truth\":true",
        "reserve_slash_authority:true",
        "\"reserve_slash_authority\":true",
        "release_slash_reserve_authority:true",
        "\"release_slash_reserve_authority\":true",
        "capture_slash_reserve_authority:true",
        "\"capture_slash_reserve_authority\":true",
        "index_proves_bond_enforcement(",
        "pointer_proves_bond_enforcement(",
        "manifest_proves_bond_enforcement(",
        "lookup_proves_bond_enforcement(",
        "index_grants_reserve_slash(",
        "index_grants_release_slash_reserve(",
        "index_grants_capture_slash_reserve(",
        "bond_enforcement_from_index(",
        "reserve_slash_from_index(",
        "release_slash_reserve_from_index(",
        "capture_slash_reserve_from_index(",
        "unlock_from_bond_enforcement(",
        "unlock_from_slash_reserve(",
        "paid_from_bond_enforcement(",
        "receipt_from_bond_enforcement(",
        "balance_from_bond_enforcement(",
        "finality_from_bond_enforcement(",
        "settle_from_bond_enforcement(",
        "execute_bond_enforcement(",
        "apply_bond_enforcement(",
        "commit_bond_enforcement(",
        "reserve_slash(",
        "release_slash_reserve(",
        "capture_slash_reserve(",
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
                &format!("svc-index source {}", path.display()),
            );
        }
    }
}

#[test]
fn index_manifest_does_not_add_phase4_round3_runtime_economy_or_settlement_dependencies() {
    let cargo = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "ron-accounting",
        "svc-rewarder",
        "ron-policy",
        "anchor-lang",
        "spl-token",
        "ethers",
        "web3",
        "solana-sdk",
        "solana-client",
    ] {
        assert_not_contains(&cargo, forbidden, "svc-index Cargo.toml");
    }
}

#[test]
fn index_allows_bond_enforcement_reference_language_without_promoting_it_to_authority() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for allowed_reference_phrase in [
        "svc-index may point to bond-enforcement/slash-reserve artifacts only as references",
        "bond enforcement artifact cid proves bytes only",
        "slash reserve artifact cid proves bytes only",
        "release/capture reserve artifact cid proves bytes only",
    ] {
        assert_contains(
            &doc,
            allowed_reference_phrase,
            "svc-index docs should allow Round 3 reference-only artifact language",
        );
    }

    for forbidden_authority_phrase in [
        "index pointer proves bond enforcement",
        "index pointer grants reserve slash",
        "index pointer grants release slash reserve",
        "index pointer grants capture slash reserve",
        "index pointer unlocks paid content from bond enforcement",
        "index pointer finalizes bond enforcement settlement",
        "index pointer executes controlled slash",
    ] {
        assert_not_contains(
            &doc,
            forbidden_authority_phrase,
            "svc-index docs must not promote Round 3 pointer authority",
        );
    }
}

#[test]
fn preflight_runner_names_phase4_round3_index_bond_enforcement_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "quickchain_phase4_bond_enforcement_boundary",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-index dev-quickchain-preflight.sh");
    }
}
