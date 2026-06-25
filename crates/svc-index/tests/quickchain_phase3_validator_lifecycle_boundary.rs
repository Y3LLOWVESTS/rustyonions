#![allow(
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines
)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle boundary tests for svc-index.
//! RO:WHY — Index may point to backend-derived lifecycle artifacts later, but pointers are not lifecycle, governance, paid-unlock, finality, wallet, ledger, bridge, staking, slashing, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, manifest pointer DTOs, route/source literals.
//! RO:INVARIANTS — index points; index does not prove, unlock, settle, rotate, revoke, mark downtime, accept evidence, update governance params, or mutate wallet/ledger state.
//! RO:METRICS — none; source/docs/DTO boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — blocks validator lifecycle/evidence/governance authority smuggling through pointer DTOs.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase3_validator_lifecycle_boundary.

use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

use svc_index::types::{
    AssetManifestPointer, PutAssetManifestPointer, PutSiteManifestPointer, SiteManifestPointer,
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const SITE_NAME: &str = "phase3-lifecycle-site";

const PHASE3_LIFECYCLE_AUTHORITY_FIELDS: &[&str] = &[
    "validator_lifecycle_authority",
    "validator_lifecycle_decision",
    "lifecycle_decision",
    "validator_rotation",
    "validator_revocation",
    "validator_downtime",
    "validator_degraded",
    "validator_equivocation",
    "validator_equivocation_evidence",
    "validator_double_attestation",
    "double_attestation_evidence",
    "validator_split_brain",
    "split_brain_evidence",
    "replay_challenge",
    "replay_challenge_evidence",
    "governance_parameter_update",
    "governance_approval",
    "validator_lifecycle_paid_unlock",
    "validator_lifecycle_receipt_truth",
    "validator_lifecycle_balance_truth",
    "validator_lifecycle_finality_truth",
    "validator_lifecycle_settlement_truth",
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

fn source_text_under(rel: &str) -> String {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join(rel), &mut files);

    let mut out = String::new();
    for path in files {
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read source {}: {err}", path.display()));
        out.push_str(&strip_line_comments(&src));
        out.push('\n');
    }

    out.to_ascii_lowercase()
}

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        haystack.contains(needle),
        "{label} must contain required Phase 3 Round 2 marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    assert!(
        !haystack.contains(needle),
        "{label} must not contain forbidden Phase 3 Round 2 authority marker: {needle}"
    );
}

fn insert_authority_field(value: &mut Value, field: &str) {
    let Value::Object(object) = value else {
        panic!("fixture must be object");
    };

    object.insert(field.to_owned(), json!("forbidden-authority"));
}

fn assert_unknown_field_rejects<T>(label: &str, mut value: Value, authority_field: &str)
where
    T: serde::de::DeserializeOwned + std::fmt::Debug,
{
    insert_authority_field(&mut value, authority_field);

    let err = serde_json::from_value::<T>(value)
        .expect_err("authority-shaped lifecycle field must reject");

    assert!(
        err.to_string().contains("unknown field"),
        "{label} must reject lifecycle authority field {authority_field:?}, got {err}"
    );
}

#[test]
fn docs_name_phase3_round2_index_validator_lifecycle_boundary() {
    let doc = normalized(&read_rel("docs/quickchain-preflight.md"));

    for required in [
        "phase 3 round 2 validator lifecycle boundary",
        "svc-index may store and return backend-derived lookup pointers and metadata only",
        "svc-index is not validator lifecycle authority",
        "svc-index is not validator rotation authority",
        "svc-index is not validator revocation authority",
        "svc-index is not validator downtime authority",
        "svc-index is not validator degraded-status authority",
        "svc-index is not validator equivocation authority",
        "svc-index is not replay challenge authority",
        "svc-index is not governance parameter-update authority",
        "validator rotation, revocation, downtime, degraded status, equivocation evidence, double-attestation evidence, split-brain evidence, replay challenge evidence, and governance-gated parameter updates are not index truth",
        "index entries, names, manifests, profile pointers, b3 hashes, provider records, cache hits, and route metadata cannot unlock paid content",
        "validator lifecycle metadata cannot mint, transfer, burn, hold, capture, release, issue receipts, mutate balances, prove finality, prove settlement, or replace wallet/ledger truth",
        "svc-index must reject validator lifecycle/evidence/governance authority smuggling through pointer dtos, routes, and source boundaries",
        "quickchain_phase3_validator_lifecycle_boundary",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn pointer_dtos_reject_validator_lifecycle_evidence_and_governance_authority_fields() {
    let put_asset = json!({
        "asset_kind": "image",
        "manifest_cid": MANIFEST_CID,
        "owner_passport_subject": "passport:creator",
        "owner_wallet_account": "acct_creator",
        "updated_at_ms": 1
    });

    let put_site = json!({
        "manifest_cid": MANIFEST_CID,
        "owner_passport_subject": "passport:creator",
        "owner_wallet_account": "acct_creator",
        "updated_at_ms": 1
    });

    let asset_pointer = json!({
        "version": 1,
        "asset_cid": ASSET_CID,
        "asset_kind": "image",
        "manifest_cid": MANIFEST_CID,
        "owner_passport_subject": "passport:creator",
        "owner_wallet_account": "acct_creator",
        "updated_at_ms": 1
    });

    let site_pointer = json!({
        "version": 1,
        "name": SITE_NAME,
        "manifest_cid": MANIFEST_CID,
        "owner_passport_subject": "passport:creator",
        "owner_wallet_account": "acct_creator",
        "updated_at_ms": 1
    });

    for authority_field in PHASE3_LIFECYCLE_AUTHORITY_FIELDS {
        assert_unknown_field_rejects::<PutAssetManifestPointer>(
            "PutAssetManifestPointer",
            put_asset.clone(),
            authority_field,
        );
        assert_unknown_field_rejects::<PutSiteManifestPointer>(
            "PutSiteManifestPointer",
            put_site.clone(),
            authority_field,
        );
        assert_unknown_field_rejects::<AssetManifestPointer>(
            "AssetManifestPointer",
            asset_pointer.clone(),
            authority_field,
        );
        assert_unknown_field_rejects::<SiteManifestPointer>(
            "SiteManifestPointer",
            site_pointer.clone(),
            authority_field,
        );
    }
}

#[test]
fn route_source_does_not_expose_validator_lifecycle_governance_or_evidence_routes() {
    let routes = source_text_under("src/http/routes");

    for forbidden in [
        "\"/quickchain",
        "\"/validator-lifecycle",
        "\"/validator-rotation",
        "\"/validator-revocation",
        "\"/validator-downtime",
        "\"/validator-degraded",
        "\"/validator-equivocation",
        "\"/validator-double-attestation",
        "\"/validator-split-brain",
        "\"/replay-challenge",
        "\"/governance-parameter",
        "\"/governance-approval",
        "\"/staking",
        "\"/slashing",
        "\"/bond",
        "\"/bridge",
        "\"/external-settlement",
        "\"/rox",
        "\"/solana",
    ] {
        assert_not_contains(&routes, forbidden, "svc-index route source");
    }
}

#[test]
fn runtime_source_does_not_construct_validator_lifecycle_or_paid_unlock_authority() {
    let source = source_text_under("src");

    for forbidden in [
        "validator_lifecycle_authority:true",
        "\"validator_lifecycle_authority\":true",
        "validator_rotation_authority:true",
        "\"validator_rotation_authority\":true",
        "validator_revocation_authority:true",
        "\"validator_revocation_authority\":true",
        "validator_downtime_authority:true",
        "\"validator_downtime_authority\":true",
        "validator_degraded_authority:true",
        "\"validator_degraded_authority\":true",
        "validator_equivocation_authority:true",
        "\"validator_equivocation_authority\":true",
        "replay_challenge_authority:true",
        "\"replay_challenge_authority\":true",
        "governance_parameter_update_authority:true",
        "\"governance_parameter_update_authority\":true",
        "index_proves_validator_lifecycle",
        "pointer_proves_validator_lifecycle",
        "manifest_proves_validator_lifecycle",
        "lookup_proves_validator_lifecycle",
        "unlock_from_validator_lifecycle",
        "unlock_from_validator_rotation",
        "unlock_from_validator_revocation",
        "unlock_from_validator_evidence",
        "unlock_from_replay_challenge",
        "unlock_from_governance_approval",
        "paid_from_validator_lifecycle",
        "receipt_from_validator_lifecycle",
        "balance_from_validator_lifecycle",
        "finality_from_validator_lifecycle",
        "settlement_from_validator_lifecycle",
        "admit_validator(",
        "revoke_validator(",
        "rotate_validator(",
        "mark_validator_down(",
        "mark_validator_degraded(",
        "accept_equivocation_evidence(",
        "accept_replay_challenge(",
        "commit_governance_parameter_update(",
        "slash_validator(",
        "stake_validator(",
        "bridge_settlement(",
        "external_settlement(",
    ] {
        assert_not_contains(&source, forbidden, "svc-index production source");
    }
}

#[test]
fn runtime_source_does_not_import_wallet_ledger_policy_or_external_settlement_runtime() {
    let source = source_text_under("src");

    for forbidden in [
        "svc_wallet::",
        "ron_ledger::",
        "ron_policy::",
        "ron_proto::quickchain",
        "quickchain::",
        "solana_client",
        "spl_token",
        "anchor_lang",
        "mutate_balance(",
        "create_receipt(",
        "grant_paid_access(",
        "commit_checkpoint(",
        "anchor_checkpoint(",
        "bridge_settlement(",
    ] {
        assert_not_contains(&source, forbidden, "svc-index production source");
    }
}
