#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — Phase 1 Round 2 downstream-confirmation tests for svc-index.
//! RO:WHY — Confirms index can point to artifacts without becoming QuickChain proof, root, finality, paid-unlock, wallet, or ledger authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, svc_index::types, src/http/routes/index_manifests.rs.
//! RO:INVARIANTS — names, manifests, b3 CIDs, owner refs, cache hits, and policy metadata are lookup context only.
//! RO:METRICS — none; docs/source/DTO boundary test.
//! RO:CONFIG — source-only checks.
//! RO:SECURITY — prevents artifact pointers from being mislabeled as settlement proofs or spend authority.
//! RO:TEST — cargo test -p svc-index --test quickchain_phase1_round2_confirmation.

use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

use svc_index::types::{normalize_b3_cid, AssetManifestPointer, SiteManifestPointer};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

const FORBIDDEN_AUTHORITY_FIELDS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
    "receipt_inclusion_proof",
    "account_proof",
    "account_root",
    "state_root",
    "state_proof",
    "hold_root",
    "hold_proof",
    "epoch_root",
    "epoch_included",
    "root_hash",
    "root_proof",
    "proof_hash",
    "merkle_proof",
    "checkpoint_root",
    "checkpoint_hash",
    "checkpoint_proof",
    "validator_signature",
    "validator_set",
    "quorum_signature",
    "anchor_proof",
    "anchored",
    "external_anchor",
    "bridge_proof",
    "balance_minor",
    "wallet_balance",
    "ledger_balance",
    "paid_proof",
    "unlock_granted",
    "finality",
    "finalized",
    "settlement_status",
    "spend_authority",
    "operation_id",
    "idempotency_key",
    "account_sequence",
    "hold_id",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = crate_dir().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn normalized(text: &str) -> String {
    text.to_ascii_lowercase().replace('`', "")
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//")
                || trimmed.starts_with("//!")
                || trimmed.starts_with("///")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rs_sources(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            collect_rs_sources(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn read_all_src() -> String {
    let src_root = crate_dir().join("src");
    let mut files = Vec::new();
    collect_rs_sources(&src_root, &mut files);
    files.sort();

    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn string_literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            let mut escaped = false;

            while j < bytes.len() {
                let b = bytes[j];

                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    break;
                }

                j += 1;
            }

            if j < bytes.len() {
                out.push(source[start..j].to_owned());
                i = j + 1;
                continue;
            }
        }

        i += 1;
    }

    out
}

fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context} must preserve required marker: {needle}"
    );
}

fn assert_not_contains(haystack: &str, forbidden: &str, context: &str) {
    assert!(
        !haystack.contains(forbidden),
        "{context} must not contain forbidden authority marker: {forbidden}"
    );
}

fn assert_serialized_lacks_authority<T>(label: &str, value: &T)
where
    T: Serialize,
{
    let serialized = serde_json::to_value(value)
        .unwrap_or_else(|err| panic!("{label} must serialize to JSON: {err}"));

    let Value::Object(map) = serialized else {
        panic!("{label} must serialize as a JSON object");
    };

    for field in FORBIDDEN_AUTHORITY_FIELDS {
        assert!(
            !map.contains_key(*field),
            "{label} must not expose authority-shaped field {field:?}"
        );
    }
}

#[test]
fn docs_name_phase1_round2_index_downstream_confirmation_boundary() {
    let doc = normalized(&read("docs/quickchain-preflight.md"));

    for required in [
        "phase 1 round 2 downstream confirmation",
        "index can point to artifacts but not prove them",
        "artifact pointer is not proof",
        "manifest pointer is not proof",
        "index pointer is not quickchain root authority",
        "index pointer is not finality authority",
        "policy metadata on an index entry is not wallet or ledger proof",
        "svc-index cannot unlock paid content from cache alone",
        "svc-wallet remains the paid mutation path",
        "ron-ledger remains durable economic truth",
        "future statuses remain parked: accepted, epoch_included, finalized, anchored",
        "quickchain_phase1_round2_confirmation",
    ] {
        assert_contains(&doc, required, "svc-index quickchain-preflight.md");
    }
}

#[test]
fn manifest_pointers_can_reference_phase1_artifacts_without_proof_authority() {
    assert_eq!(
        normalize_b3_cid(ASSET_CID).expect("asset cid must be canonical"),
        ASSET_CID
    );
    assert_eq!(
        normalize_b3_cid(MANIFEST_CID).expect("manifest cid must be canonical"),
        MANIFEST_CID
    );

    let asset_pointer = AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "manifest".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:creator".to_owned()),
        owner_wallet_account: Some("acct_creator_reference_only".to_owned()),
        updated_at_ms: 1,
    };

    let site_pointer = SiteManifestPointer {
        version: 1,
        name: "creator-site".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:creator".to_owned()),
        owner_wallet_account: Some("acct_creator_reference_only".to_owned()),
        updated_at_ms: 1,
    };

    assert_serialized_lacks_authority("AssetManifestPointer", &asset_pointer);
    assert_serialized_lacks_authority("SiteManifestPointer", &site_pointer);

    let asset_json = serde_json::to_value(&asset_pointer).expect("asset pointer serializes");
    let site_json = serde_json::to_value(&site_pointer).expect("site pointer serializes");

    assert_eq!(asset_json["manifest_cid"], MANIFEST_CID);
    assert_eq!(site_json["manifest_cid"], MANIFEST_CID);
    assert_eq!(
        asset_json["owner_wallet_account"],
        "acct_creator_reference_only"
    );
    assert_eq!(
        site_json["owner_wallet_account"],
        "acct_creator_reference_only"
    );
}

#[test]
fn index_manifest_routes_remain_pointer_routes_not_root_or_proof_routes() {
    let source = strip_line_comments(&read("src/http/routes/index_manifests.rs"));

    for required in [
        "PutAssetManifestPointer",
        "PutSiteManifestPointer",
        "AssetManifestPointer",
        "SiteManifestPointer",
        "normalize_b3_cid",
        "put_asset_manifest_pointer",
        "get_asset_manifest_pointer",
        "put_site_manifest_pointer",
        "get_site_manifest_pointer",
        "StatusCode::ACCEPTED",
    ] {
        assert_contains(&source, required, "svc-index manifest pointer route source");
    }

    for forbidden in [
        "receipt_root",
        "receipt_proof",
        "receipt_inclusion_proof",
        "account_root",
        "state_root",
        "state_proof",
        "hold_root",
        "epoch_root",
        "epoch_included",
        "checkpoint_root",
        "checkpoint_hash",
        "checkpoint_proof",
        "validator_signature",
        "validator_set",
        "quorum_signature",
        "anchor_proof",
        "anchored",
        "external_anchor",
        "bridge_proof",
        "wallet_balance",
        "ledger_balance",
        "paid_proof",
        "unlock_granted",
        "finality",
        "finalized",
        "settlement_status",
        "spend_authority",
        "create_receipt",
        "mutate_balance",
        "produce_root",
        "produce_proof",
        "verify_finality",
        "grant_paid_access",
    ] {
        assert_not_contains(
            &source,
            forbidden,
            "svc-index manifest pointer route source",
        );
    }
}

#[test]
fn runtime_source_does_not_define_phase1_root_proof_or_finality_authority() {
    let source = strip_line_comments(&read_all_src()).to_ascii_lowercase();

    for forbidden in [
        "ron_proto::quickchain",
        "ron_policy::",
        "svc_wallet::",
        "ron_ledger::",
        "quickchain::",
        "quickchain_root",
        "receipt_root",
        "receipt_proof",
        "receipt_inclusion_proof",
        "account_root",
        "state_root",
        "state_proof",
        "hold_root",
        "epoch_root",
        "epoch_included",
        "checkpoint_root",
        "checkpoint_hash",
        "checkpoint_proof",
        "validator_signature",
        "validator_set",
        "quorum_signature",
        "anchor_proof",
        "anchored",
        "external_anchor",
        "bridge_proof",
        "settlement_status",
        "spend_authority",
        "index_entry_proves_payment",
        "manifest_pointer_proves_payment",
        "artifact_pointer_proves_proof",
        "policy_metadata_proves_payment",
        "cache_hit_proves_entitlement",
        "create_receipt(",
        "mutate_balance(",
        "produce_root(",
        "produce_proof(",
        "verify_finality(",
        "grant_spend_authority(",
        "grant_paid_access(",
    ] {
        assert_not_contains(&source, forbidden, "svc-index production source");
    }
}

#[test]
fn public_string_routes_do_not_expose_quickchain_proof_or_settlement_surfaces() {
    let source = strip_line_comments(&read_all_src());
    let literals = string_literals(&source).join("\n").to_ascii_lowercase();

    for forbidden in [
        "/quickchain",
        "/root",
        "/state-root",
        "/receipt-root",
        "/proof",
        "/checkpoint",
        "/validator",
        "/validators",
        "/finality",
        "/settlement",
        "/external-settlement",
        "/anchor",
        "/bridge",
        "/staking",
        "/liquidity",
        "/rox",
        "/solana",
        "/wallet",
        "/ledger",
        "/balance",
        "/receipt",
        "/paid/unlock",
    ] {
        assert_not_contains(&literals, forbidden, "svc-index route string literals");
    }
}
