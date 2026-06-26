#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 4 Round 1 bond/slash/stake/liquidity boundary tests for svc-index.
//! RO:WHY — Index may point to bond/evidence artifacts, but pointers remain lookup truth only, never bond/slash/finality/settlement/payment authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, route/source scanner, Cargo manifest.
//! RO:INVARIANTS — pointers are references; b3 proves bytes only; names/crab navigation are not authority; no paid unlock from index.
//! RO:METRICS — none; docs/source boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents index/pointer metadata from becoming Phase 4 economic or validator consequence authority.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase4_bond_boundary.

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
fn docs_name_phase4_round1_index_bond_pointer_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 1 bond artifact pointer boundary",
        "svc-index may point to bond/evidence/report artifacts only as references",
        "index pointer is not bond truth",
        "index pointer is not slash truth",
        "index pointer is not staking authority",
        "index pointer is not liquidity authority",
        "index pointer is not settlement finality",
        "index pointer is not validator economy authority",
        "bond artifact cid proves bytes only",
        "slash evidence cid proves bytes only",
        "b3 proves bytes, not bond lifecycle truth",
        "names are pointers, not bond authority",
        "manifest lookup is not slash authority",
        "provider lookup is not staking authority",
        "index cache cannot unlock paid content or trigger validator consequences",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "quickchain_phase4_bond_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn index_route_surface_does_not_expose_bond_slash_stake_liquidity_or_settlement_authority() {
    let mut files = Vec::new();

    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);
    files.push(crate_root().join("src/router.rs"));

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

    for path in files {
        if !path.exists() {
            continue;
        }

        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read source {}: {err}", path.display())),
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
fn index_source_does_not_construct_phase4_bond_slash_stake_or_liquidity_authority() {
    let mut files = Vec::new();

    for subdir in [
        "src/http",
        "src/pipeline",
        "src/store",
        "src/cache",
        "src/state",
        "src/dht",
    ] {
        let root = crate_root().join(subdir);
        if root.exists() {
            collect_rust_files(&root, &mut files);
        }
    }

    files.push(crate_root().join("src/types.rs"));
    files.push(crate_root().join("src/lib.rs"));

    let forbidden_compact_markers = [
        "bond_authority:true",
        "\"bond_authority\":true",
        "bond_truth:true",
        "\"bond_truth\":true",
        "bond_lifecycle_authority:true",
        "\"bond_lifecycle_authority\":true",
        "slash_authority:true",
        "\"slash_authority\":true",
        "slash_truth:true",
        "\"slash_truth\":true",
        "staking_authority:true",
        "\"staking_authority\":true",
        "liquidity_authority:true",
        "\"liquidity_authority\":true",
        "index_proves_bond(",
        "index_proves_slash(",
        "index_grants_stake(",
        "index_grants_liquidity(",
        "bond_from_index(",
        "slash_from_index(",
        "stake_from_index(",
        "liquidity_from_index(",
        "unlock_from_index(",
        "paid_from_index(",
        "finality_from_index(",
        "settle_from_index(",
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
        if !path.exists() {
            continue;
        }

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
fn index_manifest_does_not_add_phase4_runtime_economy_or_settlement_dependencies() {
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
fn index_allows_reference_language_without_promoting_it_to_authority() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for allowed_reference_phrase in [
        "svc-index may point to bond/evidence/report artifacts only as references",
        "bond artifact cid proves bytes only",
        "slash evidence cid proves bytes only",
    ] {
        assert_contains(
            &doc,
            allowed_reference_phrase,
            "svc-index docs should allow reference-only artifact language",
        );
    }

    for forbidden_authority_phrase in [
        "index pointer proves bond",
        "index pointer proves slash",
        "index pointer grants stake",
        "index pointer grants liquidity",
        "index pointer unlocks paid content",
        "index pointer finalizes settlement",
    ] {
        assert_not_contains(
            &doc,
            forbidden_authority_phrase,
            "svc-index docs must not promote pointer authority",
        );
    }
}
