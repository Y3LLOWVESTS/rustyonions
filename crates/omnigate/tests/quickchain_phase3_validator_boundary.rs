#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 3 Round 1 passport-gated validator boundary tests for omnigate.
//! RO:WHY — Omnigate may hydrate backend-derived validator/readiness labels later, but must not become validator identity, passport registry, capability, wallet, ledger, finality, staking, slashing, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, routes/v1/header_policy.rs, v1 route/product hydration source.
//! RO:INVARIANTS — omnigate remains product hydration / access composition / backend-derived display surface only.
//! RO:METRICS — none; source/docs/header-policy boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks validator/passport/registry/capability authority smuggling and paid-unlock authority creep.
//! RO:TEST — cargo test -p omnigate --test quickchain_phase3_validator_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

const PHASE3_HEADER_POLICY_MARKERS: &[&str] = &[
    "\"x-ron-validator\"",
    "\"x-ron-validator-set\"",
    "\"x-ron-validator-signature\"",
    "\"x-ron-validator-passport\"",
    "\"x-ron-validator-capability\"",
    "\"x-ron-validator-registry-entry\"",
    "\"x-ron-validator-membership-proof\"",
    "\"x-ron-validator-authorization\"",
    "\"x-ron-validator-authz-result\"",
    "\"x-ron-passport-validator\"",
    "\"x-ron-passport-validator-admission\"",
    "\"x-ron-passport-validator-capability\"",
    "\"x-ron-registry-validator\"",
    "\"x-ron-registry-validator-set\"",
    "\"x-ron-capability-validator\"",
    "\"x-ron-capability-validator-scope\"",
    "\"x-ron-attestation-identity\"",
    "raw.starts_with(\"x-ron-validator-\")",
    "raw.starts_with(\"x-ron-passport-validator\")",
    "raw.starts_with(\"x-ron-registry-validator\")",
    "raw.starts_with(\"x-ron-capability-validator\")",
    "raw.starts_with(\"x-ron-attestation-identity\")",
    "raw.starts_with(\"x-ron-validator-authz\")",
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
        "{label} must contain required Phase 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 3 authority marker: {needle}"
    );
}

#[test]
fn docs_name_phase3_round1_omnigate_validator_passport_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 1 validator/passport boundary",
        "omnigate may hydrate backend-derived validator set/readiness labels if future backend routes expose them",
        "omnigate validator status labels are display and hydration context only",
        "omnigate is product hydration / access composition / backend-derived display surface only",
        "omnigate is not validator identity authority",
        "omnigate is not passport registry authority",
        "omnigate is not validator capability authority",
        "omnigate is not validator-set authority",
        "wallet/ledger truth remains backend-owned",
        "accepted wallet receipts can unlock paid content",
        "validator/passport material cannot unlock paid content by itself",
        "validator/passport material cannot mint, transfer, burn, hold, capture, release, or issue receipts",
        "validator/passport material cannot replace wallet/ledger truth",
        "omnigate rejects phase 3 validator/passport authority header smuggling",
        "quickchain_phase3_validator_boundary",
    ] {
        assert_contains(&doc, required, "omnigate quickchain-preflight.md");
    }
}

#[test]
fn omnigate_header_policy_blocks_phase3_validator_passport_registry_and_capability_authority() {
    let policy = read_rel("src/routes/v1/header_policy.rs");

    for required in PHASE3_HEADER_POLICY_MARKERS {
        assert_contains(
            &policy,
            required,
            "omnigate v1 header_policy.rs Phase 3 validator/passport denylist",
        );
    }

    for allowed_context in [
        "x-ron-wallet-account",
        "x-ron-passport",
        "x-ron-wallet-txid",
        "x-ron-wallet-receipt-hash",
        "x-ron-paid-op",
        "x-ron-paid-asset",
    ] {
        assert_contains(
            &policy,
            allowed_context,
            "omnigate v1 header_policy.rs product context allowlist docs",
        );
    }
}

#[test]
fn omnigate_routes_do_not_expose_validator_passport_registry_or_finality_authority_paths() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes/v1"), &mut route_files);

    let forbidden_route_fragments = [
        ".route(\"/quickchain",
        ".route(\"/validator",
        ".route(\"/validators",
        ".route(\"/passport/validator",
        ".route(\"/registry/validator",
        ".route(\"/capability/validator",
        ".route(\"/validator-set",
        ".route(\"/validator-admission",
        ".route(\"/validator-revocation",
        ".route(\"/validator-rotation",
        ".route(\"/finality",
        ".route(\"/staking",
        ".route(\"/slashing",
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
fn omnigate_source_does_not_implement_validator_admission_or_paid_unlock_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes/v1"), &mut files);
    collect_rust_files(&crate_root().join("src/hydration"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/middleware"), &mut files);

    let forbidden_compact_markers = [
        "validator_identity_authority:true",
        "\"validator_identity_authority\":true",
        "passport_registry_authority:true",
        "\"passport_registry_authority\":true",
        "validator_capability_authority:true",
        "\"validator_capability_authority\":true",
        "validator_set_authority:true",
        "\"validator_set_authority\":true",
        "validator_finality:true",
        "\"validator_finality\":true",
        "validator_paid_unlock:true",
        "\"validator_paid_unlock\":true",
        "unlock_from_validator_passport",
        "unlock_from_validator_capability",
        "unlock_from_validator_set",
        "unlock_from_passport_registry",
        "paid_by_validator_attestation",
        "paid_by_validator_capability",
        "mint_from_validator_passport",
        "issue_from_validator_passport",
        "admit_validator(",
        "revoke_validator(",
        "rotate_validator(",
        "slash_validator(",
        "stake_validator(",
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
                &format!("omnigate runtime source {}", path.display()),
            );
        }
    }
}

#[test]
fn existing_paid_routes_do_not_unlock_from_validator_or_passport_material() {
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
            "unlock_from_validator_passport",
            "unlock_from_validator_capability",
            "unlock_from_validator_set",
            "unlock_from_passport_registry",
            "paid_by_validator_attestation",
            "paid_by_validator_capability",
            "cache_unlock_authority",
            "cache_only_unlock",
            "validator_receipt_truth",
            "validator_balance_truth",
            "validator_finality_truth",
        ] {
            assert_not_contains(&source, forbidden, rel);
        }
    }
}

#[test]
fn omnigate_source_does_not_import_ledger_or_build_validator_economy_runtime() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    for path in files {
        let source = normalized(&strip_line_comments(
            &fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read source {}: {err}", path.display())),
        ));

        for forbidden in [
            "use ron_ledger",
            "ron_ledger::",
            "ledger_commit",
            "mutate_ledger",
            "mutate_balance",
            "validator_bond",
            "validator_stake",
            "validator_slash",
            "solana_client",
            "spl_token",
            "anchor_lang",
        ] {
            assert_not_contains(
                &source,
                forbidden,
                &format!("omnigate source {}", path.display()),
            );
        }
    }
}
