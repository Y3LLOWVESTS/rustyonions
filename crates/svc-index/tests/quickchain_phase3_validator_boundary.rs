#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 3 Round 1 validator/passport boundary tests for svc-index.
//! RO:WHY — Index may point to backend-derived validator/readiness artifacts later, but pointers are not validator, passport-registry, quorum, proof, finality, payment, wallet, or ledger truth.
//! RO:INTERACTS — docs/quickchain-preflight.md, manifest pointer DTOs, route/source literals.
//! RO:INVARIANTS — index points; index does not prove, unlock, settle, admit validators, revoke validators, or mutate wallet/ledger state.
//! RO:METRICS — none; source/docs/DTO boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks validator/passport/registry/capability authority smuggling through manifest pointer DTOs.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase3_validator_boundary.

use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

use svc_index::types::{PutAssetManifestPointer, PutSiteManifestPointer};

const ASSET_MANIFEST_CID: &str =
    "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const SITE_MANIFEST_CID: &str =
    "b3:1111111111111111111111111111111111111111111111111111111111111111";

const PHASE3_VALIDATOR_AUTHORITY_FIELDS: &[&str] = &[
    "validator",
    "validators",
    "validator_set",
    "validator_signature",
    "validator_passport",
    "validator_capability",
    "validator_registry_entry",
    "validator_membership_proof",
    "validator_authorization",
    "validator_authz_result",
    "passport_validator",
    "passport_validator_admission",
    "passport_validator_capability",
    "registry_validator",
    "registry_validator_set",
    "capability_validator",
    "capability_validator_scope",
    "attestation_identity",
    "validator_admission",
    "validator_revocation",
    "validator_rotation",
    "validator_finality",
    "validator_paid_unlock",
    "staking",
    "slashing",
    "bonded_validator",
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

fn read_all_src() -> String {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);

    let mut out = String::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        out.push_str(&strip_line_comments(&source));
        out.push('\n');
    }

    out
}

fn string_literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }

        let mut literal = String::new();
        let mut escaped = false;

        for next in chars.by_ref() {
            if escaped {
                literal.push(next);
                escaped = false;
                continue;
            }

            match next {
                '\\' => escaped = true,
                '"' => break,
                other => literal.push(other),
            }
        }

        out.push(literal);
    }

    out
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 3 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, label: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{label} must not contain forbidden Phase 3 authority marker: {forbidden}"
    );
}

fn base_asset_pointer() -> Value {
    json!({
        "asset_kind": "image",
        "manifest_cid": ASSET_MANIFEST_CID,
        "owner_passport_subject": "passport:creator-alpha",
        "owner_wallet_account": "acct:creator-alpha",
        "updated_at_ms": 1
    })
}

fn base_site_pointer() -> Value {
    json!({
        "manifest_cid": SITE_MANIFEST_CID,
        "owner_passport_subject": "passport:site-owner",
        "owner_wallet_account": "acct:site-owner",
        "updated_at_ms": 1
    })
}

fn object_with_extra_field(mut value: Value, field: &str) -> Value {
    let object = value
        .as_object_mut()
        .expect("test value should be a JSON object");
    object.insert(field.to_owned(), Value::String("forbidden".to_owned()));
    value
}

#[test]
fn docs_name_phase3_round1_index_validator_passport_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 1 validator/passport boundary",
        "svc-index may point to backend-derived validator set/readiness artifacts if future backend routes expose them",
        "index validator status pointers are references only",
        "svc-index is lookup and pointer infrastructure only",
        "svc-index is not validator identity authority",
        "svc-index is not passport registry authority",
        "svc-index is not validator capability authority",
        "svc-index is not validator-set authority",
        "svc-index cannot admit validators",
        "svc-index cannot revoke validators",
        "svc-index cannot rotate validators",
        "svc-index cannot unlock paid content from validator/passport material",
        "svc-index cannot replace wallet/ledger truth",
        "quickchain_phase3_validator_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn manifest_pointer_dtos_reject_phase3_validator_passport_authority_fields() {
    for field in PHASE3_VALIDATOR_AUTHORITY_FIELDS {
        let asset = object_with_extra_field(base_asset_pointer(), field);
        let site = object_with_extra_field(base_site_pointer(), field);

        let asset_err = serde_json::from_value::<PutAssetManifestPointer>(asset)
            .expect_err("asset manifest pointer must reject Phase 3 authority field");
        let site_err = serde_json::from_value::<PutSiteManifestPointer>(site)
            .expect_err("site manifest pointer must reject Phase 3 authority field");

        assert!(
            asset_err.to_string().contains("unknown field"),
            "asset pointer field {field} should reject as unknown field, got: {asset_err}"
        );
        assert!(
            site_err.to_string().contains("unknown field"),
            "site pointer field {field} should reject as unknown field, got: {site_err}"
        );
    }
}

#[test]
fn manifest_pointer_dtos_still_allow_owner_passport_and_wallet_references_as_metadata() {
    let asset = serde_json::from_value::<PutAssetManifestPointer>(base_asset_pointer())
        .expect("ordinary owner references remain valid asset pointer metadata");
    let site = serde_json::from_value::<PutSiteManifestPointer>(base_site_pointer())
        .expect("ordinary owner references remain valid site pointer metadata");

    assert_eq!(
        asset.owner_passport_subject.as_deref(),
        Some("passport:creator-alpha")
    );
    assert_eq!(
        asset.owner_wallet_account.as_deref(),
        Some("acct:creator-alpha")
    );
    assert_eq!(
        site.owner_passport_subject.as_deref(),
        Some("passport:site-owner")
    );
    assert_eq!(
        site.owner_wallet_account.as_deref(),
        Some("acct:site-owner")
    );
}

#[test]
fn public_string_routes_do_not_expose_phase3_validator_passport_registry_or_bonding_surfaces() {
    let source = strip_line_comments(&read_all_src());
    let literals = string_literals(&source).join("\n").to_ascii_lowercase();

    for forbidden in [
        "/quickchain",
        "/validator",
        "/validators",
        "/passport/validator",
        "/registry/validator",
        "/capability/validator",
        "/validator-set",
        "/validator-admission",
        "/validator-revocation",
        "/validator-rotation",
        "/staking",
        "/slashing",
        "/bond",
        "/bonded",
        "/bridge",
        "/external-settlement",
        "/settlement",
        "/finality",
        "/rox",
        "/solana",
        "/wallet",
        "/ledger",
        "/paid/unlock",
    ] {
        assert_not_contains(&literals, forbidden, "svc-index route string literals");
    }
}

#[test]
fn production_source_does_not_implement_validator_lifecycle_or_paid_unlock_authority() {
    let source = strip_line_comments(&read_all_src()).to_ascii_lowercase();
    let compact = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    for forbidden in [
        "ron_proto::quickchain",
        "ron_policy::",
        "svc_wallet::",
        "ron_ledger::",
        "quickchain::",
        "admit_validator(",
        "revoke_validator(",
        "rotate_validator(",
        "authorize_validator(",
        "register_validator(",
        "deregister_validator(",
        "validator_identity_authority:true",
        "\"validator_identity_authority\":true",
        "passport_registry_authority:true",
        "\"passport_registry_authority\":true",
        "validator_capability_authority:true",
        "\"validator_capability_authority\":true",
        "validator_set_authority:true",
        "\"validator_set_authority\":true",
        "validator_paid_unlock:true",
        "\"validator_paid_unlock\":true",
        "unlock_from_validator_passport",
        "unlock_from_validator_capability",
        "unlock_from_validator_set",
        "unlock_from_passport_registry",
        "index_entry_proves_validator_membership",
        "manifest_pointer_proves_validator_membership",
        "index_entry_proves_payment",
        "manifest_pointer_proves_payment",
        "grant_paid_access(",
        "mutate_balance(",
        "create_receipt(",
        "produce_root(",
        "sign_checkpoint(",
    ] {
        assert_not_contains(&compact, forbidden, "svc-index production source");
    }
}

#[test]
fn cargo_manifest_does_not_add_phase3_validator_wallet_ledger_or_external_runtime_dependencies() {
    let manifest = normalized(&read_rel("Cargo.toml"));

    for forbidden in [
        "ron-ledger",
        "svc-wallet",
        "svc-passport",
        "svc-registry",
        "ron-auth",
        "solana",
        "spl-token",
        "anchor-lang",
        "staking",
        "slashing",
        "bridge",
    ] {
        assert_not_contains(&manifest, forbidden, "svc-index Cargo.toml");
    }
}

#[test]
fn empty_json_helper_remains_used_so_serde_json_map_import_is_not_accidental() {
    let map = Map::<String, Value>::new();
    assert!(map.is_empty());
}
