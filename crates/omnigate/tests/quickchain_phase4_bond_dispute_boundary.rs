#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 4 Round 2 bond dispute/challenge simulation boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate/display backend-derived dispute metadata, but must not become dispute, appeal, freeze, irreversible slash, wallet, ledger, paid-unlock, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, v1 header policy, v1 route/product hydration source.
//! RO:INVARIANTS — dispute metadata is display-only; challenge windows are explicit; no live irreversible slash; paid unlock remains backend wallet/ledger receipt derived.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks Phase 4 Round 2 dispute/challenge/appeal/freeze authority smuggling into hydration or access decisions.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase4_bond_dispute_boundary.

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
fn docs_name_phase4_round2_omnigate_dispute_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 4 round 2 slashing/challenge simulation boundary",
        "omnigate may hydrate backend-derived disputed-bond status labels only as non-authoritative metadata",
        "omnigate is not dispute truth",
        "omnigate is not challenge-window truth",
        "omnigate is not appeal authority",
        "omnigate is not freeze authority",
        "omnigate is not irreversible slash authority",
        "dispute status cannot unlock paid content",
        "challenge evidence cannot mutate ledger truth through omnigate",
        "appeal/freeze state cannot mutate wallet or ledger truth through omnigate",
        "no live irreversible slash through omnigate",
        "accepted wallet/ledger receipts remain the only paid unlock authority",
        "omnigate rejects phase 4 dispute/challenge/appeal/freeze/slash-simulation authority header smuggling",
        "quickchain_phase4_bond_dispute_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_header_policy_names_phase4_round2_dispute_families() {
    let source = normalized(&read_rel("src/routes/v1/header_policy.rs"));

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
        assert_contains(&source, required, "omnigate header policy");
    }
}

#[test]
fn omnigate_route_surface_does_not_expose_phase4_round2_dispute_or_challenge_routes() {
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
                &format!("omnigate route source {}", path.display()),
            );
        }
    }
}

#[test]
fn omnigate_source_does_not_construct_phase4_round2_dispute_runtime_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);
    collect_rust_files(&crate_root().join("src/middleware"), &mut files);

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
        "hydrate_as_dispute_truth",
        "hydrate_as_slash_truth",
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
fn existing_paid_and_access_routes_do_not_unlock_from_dispute_or_appeal_evidence() {
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
            "unlock_from_dispute",
            "unlock_from_challenge",
            "unlock_from_appeal",
            "unlock_from_freeze",
            "paid_by_dispute_state",
            "receipt_from_dispute_state",
            "balance_from_dispute_state",
            "finality_from_dispute_state",
            "cache_unlock_from_dispute",
            "policy_unlock_from_dispute",
            "dispute_paid_unlock_authority",
        ] {
            assert_not_contains(&source, forbidden, rel);
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
        assert_contains(&script, required, "omnigate dev-quickchain-preflight.sh");
    }
}
