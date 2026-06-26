#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 2 dispute/challenge/appeal/freeze boundary tests for svc-index.
//! RO:WHY — Index may point to dispute/evidence artifacts, but pointers remain lookup truth only, never dispute, appeal, freeze, slash, finality, settlement, or payment authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, route/source scanner, Cargo manifest.
//! RO:INVARIANTS — pointers are references; b3 proves bytes only; names/crab navigation are not authority; no paid unlock or validator consequence from index metadata.
//! RO:METRICS — none; docs/source boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents index/pointer metadata from becoming Phase 4 Round 2 dispute/challenge/slash authority.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase4_bond_dispute_boundary.

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
        "{label} must contain required Phase 4 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 4 Round 2 authority marker: {needle}"
    );
}

#[test]
fn docs_name_phase4_round2_index_dispute_pointer_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 2 bond dispute/challenge pointer boundary",
        "phase 4 round 2 is simulation only",
        "svc-index may point to dispute/challenge/appeal/freeze artifacts only as references",
        "index pointer is not dispute truth",
        "index pointer is not challenge-window truth",
        "index pointer is not appeal authority",
        "index pointer is not freeze authority",
        "index pointer is not irreversible slash authority",
        "index pointer is not slash simulation authority",
        "dispute artifact cid proves bytes only",
        "challenge evidence cid proves bytes only",
        "appeal artifact cid proves bytes only",
        "freeze status label is display metadata only",
        "b3 proves bytes, not dispute resolution truth",
        "names are pointers, not dispute authority",
        "manifest lookup is not challenge evidence validation",
        "provider lookup is not appeal authority",
        "index cache cannot unlock paid content or trigger dispute consequences",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "no live irreversible slash through svc-index",
        "quickchain_phase4_bond_dispute_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn index_route_surface_does_not_expose_dispute_challenge_appeal_freeze_or_slash_routes() {
    let mut files = Vec::new();

    collect_rust_files(&crate_root().join("src/http/routes"), &mut files);
    files.push(crate_root().join("src/router.rs"));

    let forbidden_route_fragments = [
        ".route(\"/quickchain",
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
        ".route(\"/external-settlement",
        ".route(\"/bridge",
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
fn index_source_does_not_construct_phase4_round2_dispute_challenge_or_freeze_authority() {
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
        "dispute_authority:true",
        "\"dispute_authority\":true",
        "dispute_truth:true",
        "\"dispute_truth\":true",
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
        "index_proves_dispute",
        "pointer_proves_dispute",
        "manifest_proves_dispute",
        "lookup_proves_dispute",
        "index_grants_appeal",
        "index_grants_freeze",
        "dispute_from_index(",
        "challenge_from_index(",
        "appeal_from_index(",
        "freeze_from_index(",
        "unlock_from_dispute",
        "unlock_from_challenge",
        "unlock_from_appeal",
        "unlock_from_freeze",
        "paid_from_dispute",
        "receipt_from_dispute",
        "balance_from_dispute",
        "finality_from_dispute",
        "settlement_from_dispute",
        "execute_dispute(",
        "resolve_dispute(",
        "accept_challenge_evidence(",
        "open_challenge_window(",
        "submit_appeal(",
        "grant_appeal(",
        "freeze_bond(",
        "thaw_bond(",
        "capture_disputed_bond(",
        "slash_disputed_bond(",
        "execute_irreversible_slash(",
        "commit_irreversible_slash(",
        "slash_without_governance(",
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
fn index_manifest_does_not_add_phase4_round2_runtime_economy_or_settlement_dependencies() {
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
fn index_allows_dispute_reference_language_without_promoting_it_to_authority() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for allowed_reference_phrase in [
        "svc-index may point to dispute/challenge/appeal/freeze artifacts only as references",
        "dispute artifact cid proves bytes only",
        "challenge evidence cid proves bytes only",
        "appeal artifact cid proves bytes only",
    ] {
        assert_contains(
            &doc,
            allowed_reference_phrase,
            "svc-index docs should allow dispute reference-only artifact language",
        );
    }

    for forbidden_authority_phrase in [
        "index pointer proves dispute",
        "index pointer proves challenge",
        "index pointer grants appeal",
        "index pointer grants freeze",
        "index pointer unlocks paid content from dispute",
        "index pointer finalizes dispute settlement",
        "index pointer executes irreversible slash",
    ] {
        assert_not_contains(
            &doc,
            forbidden_authority_phrase,
            "svc-index docs must not promote dispute pointer authority",
        );
    }
}

#[test]
fn preflight_runner_names_phase4_round2_index_dispute_boundary_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    for required in [
        "quickchain_phase4_bond_dispute_boundary",
        "find \"$TEST_DIR\"",
        "-name 'quickchain*.rs'",
        "test -p \"$PKG\" --test \"$test_name\"",
    ] {
        assert_contains(&script, required, "svc-index dev-quickchain-preflight.sh");
    }
}
