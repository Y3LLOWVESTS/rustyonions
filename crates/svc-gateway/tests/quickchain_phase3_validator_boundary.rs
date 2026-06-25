#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 3 Round 1 passport-gated validator boundary tests for svc-gateway.
//! RO:WHY — Gateway may route backend-derived validator/readiness labels later, but must not become validator identity, passport registry, capability, wallet, ledger, finality, staking, slashing, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, headers::proxy, route/admission/layer source.
//! RO:INVARIANTS — gateway remains public boundary / route admission / proxy / fail-closed enforcement only.
//! RO:METRICS — none; source/docs/header boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks validator/passport/registry/capability authority smuggling and paid-unlock authority creep.
//! RO:TEST — cargo test -p svc-gateway --test quickchain_phase3_validator_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use http::HeaderName;
use svc_gateway::headers::proxy;

const PHASE3_VALIDATOR_PASSPORT_AUTHORITY_HEADERS: &[&str] = &[
    "x-ron-validator",
    "x-ron-validator-set",
    "x-ron-validator-signature",
    "x-ron-validator-passport",
    "x-ron-validator-passport-subject",
    "x-ron-validator-capability",
    "x-ron-validator-capability-id",
    "x-ron-validator-registry-entry",
    "x-ron-validator-membership-proof",
    "x-ron-validator-authorization",
    "x-ron-validator-authz-result",
    "x-ron-passport-validator",
    "x-ron-passport-validator-admission",
    "x-ron-passport-validator-capability",
    "x-ron-passport-validator-revocation",
    "x-ron-registry-validator",
    "x-ron-registry-validator-set",
    "x-ron-registry-validator-membership",
    "x-ron-capability-validator",
    "x-ron-capability-validator-scope",
    "x-ron-attestation-identity",
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
fn docs_name_phase3_round1_gateway_validator_passport_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 1 validator/passport boundary",
        "svc-gateway may route backend-derived validator set/readiness labels if future backend routes expose them",
        "gateway validator status labels are display and routing context only",
        "svc-gateway is public boundary / route admission / proxy / fail-closed enforcement only",
        "svc-gateway is not validator identity authority",
        "svc-gateway is not passport registry authority",
        "svc-gateway is not validator capability authority",
        "svc-gateway is not validator-set authority",
        "wallet/ledger truth remains backend-owned",
        "accepted wallet receipts can unlock paid content",
        "validator/passport material cannot unlock paid content by itself",
        "validator/passport material cannot mint, transfer, burn, hold, capture, release, or issue receipts",
        "validator/passport material cannot replace wallet/ledger truth",
        "gateway rejects phase 3 validator/passport authority header smuggling",
        "quickchain_phase3_validator_boundary",
    ] {
        assert_contains(&doc, required, "svc-gateway quickchain-preflight.md");
    }
}

#[test]
fn gateway_filters_phase3_validator_passport_registry_and_capability_authority_headers() {
    for raw in PHASE3_VALIDATOR_PASSPORT_AUTHORITY_HEADERS {
        let name = HeaderName::from_bytes(raw.as_bytes())
            .unwrap_or_else(|err| panic!("valid header name {raw}: {err}"));

        assert!(
            !proxy::should_forward_passthrough_header(&name),
            "gateway passthrough proxy must not forward Phase 3 authority header: {raw}"
        );
        assert!(
            !proxy::should_forward_product_header(&name),
            "gateway product proxy must not forward Phase 3 authority header: {raw}"
        );
        assert!(
            !proxy::should_copy_response_header(&name),
            "gateway response copy must not endorse Phase 3 authority header: {raw}"
        );
    }
}

#[test]
fn gateway_still_allows_non_authority_product_context_headers() {
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
fn gateway_routes_do_not_expose_validator_passport_registry_or_finality_authority_paths() {
    let mut route_files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut route_files);

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
                &format!("gateway route source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_source_does_not_implement_validator_admission_or_paid_unlock_authority() {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src/routes"), &mut files);
    collect_rust_files(&crate_root().join("src/admission"), &mut files);
    collect_rust_files(&crate_root().join("src/layers"), &mut files);
    collect_rust_files(&crate_root().join("src/headers"), &mut files);

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
                &format!("gateway runtime source {}", path.display()),
            );
        }
    }
}

#[test]
fn gateway_source_does_not_import_ledger_or_build_validator_economy_runtime() {
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
                &format!("gateway source {}", path.display()),
            );
        }
    }
}
