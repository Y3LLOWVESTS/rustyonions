//! RO:WHAT — Phase 1 pointer DTO strictness tests for svc-index QuickChain boundaries.
//! RO:WHY — Prevent index pointer records from smuggling paid proof, receipt, balance, finality, root, checkpoint, validator, bridge, or settlement authority.
//! RO:INTERACTS — `svc_index::types::{PutAssetManifestPointer, PutSiteManifestPointer, AssetManifestPointer, SiteManifestPointer}`.
//! RO:INVARIANTS — svc-index stores lookup/pointer metadata only; unknown authority-shaped fields must reject.
//! RO:TEST — `cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness`.

use serde_json::{json, Value};
use svc_index::types::{
    AssetManifestPointer, PutAssetManifestPointer, PutSiteManifestPointer, SiteManifestPointer,
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

const FORBIDDEN_AUTHORITY_FIELDS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
    "account_proof",
    "balance_minor",
    "wallet_balance",
    "ledger_balance",
    "paid_proof",
    "unlock_granted",
    "finality",
    "finalized",
    "settlement_status",
    "state_root",
    "checkpoint_root",
    "checkpoint_hash",
    "validator_signature",
    "bridge_proof",
    "operation_id",
    "idempotency_key",
    "account_sequence",
    "hold_id",
];

#[test]
fn put_asset_manifest_pointer_rejects_authority_smuggling_fields() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut value = json!({
            "asset_kind": "image",
            "manifest_cid": MANIFEST_CID,
            "owner_passport_subject": "passport:creator",
            "owner_wallet_account": "acct_creator",
            "updated_at_ms": 1
        });

        insert_authority_field(&mut value, field);

        let err = serde_json::from_value::<PutAssetManifestPointer>(value)
            .expect_err("asset pointer input must reject unknown authority-shaped fields");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} must reject as unknown, got: {err}"
        );
    }
}

#[test]
fn put_site_manifest_pointer_rejects_authority_smuggling_fields() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut value = json!({
            "manifest_cid": MANIFEST_CID,
            "owner_passport_subject": "passport:creator",
            "owner_wallet_account": "acct_creator",
            "updated_at_ms": 1
        });

        insert_authority_field(&mut value, field);

        let err = serde_json::from_value::<PutSiteManifestPointer>(value)
            .expect_err("site pointer input must reject unknown authority-shaped fields");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} must reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stored_asset_manifest_pointer_rejects_authority_smuggling_fields() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut value = json!({
            "version": 1,
            "asset_cid": ASSET_CID,
            "asset_kind": "image",
            "manifest_cid": MANIFEST_CID,
            "owner_passport_subject": "passport:creator",
            "owner_wallet_account": "acct_creator",
            "updated_at_ms": 1
        });

        insert_authority_field(&mut value, field);

        let err = serde_json::from_value::<AssetManifestPointer>(value)
            .expect_err("stored asset pointer must reject unknown authority-shaped fields");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} must reject as unknown, got: {err}"
        );
    }
}

#[test]
fn stored_site_manifest_pointer_rejects_authority_smuggling_fields() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut value = json!({
            "version": 1,
            "name": "creator-site",
            "manifest_cid": MANIFEST_CID,
            "owner_passport_subject": "passport:creator",
            "owner_wallet_account": "acct_creator",
            "updated_at_ms": 1
        });

        insert_authority_field(&mut value, field);

        let err = serde_json::from_value::<SiteManifestPointer>(value)
            .expect_err("stored site pointer must reject unknown authority-shaped fields");

        assert!(
            err.to_string().contains("unknown field"),
            "field {field:?} must reject as unknown, got: {err}"
        );
    }
}

#[test]
fn owner_reference_fields_remain_references_not_authority_objects() {
    let value = json!({
        "asset_kind": "image",
        "manifest_cid": MANIFEST_CID,
        "owner_passport_subject": "passport:creator",
        "owner_wallet_account": "acct_creator",
        "updated_at_ms": 1
    });

    let pointer = serde_json::from_value::<PutAssetManifestPointer>(value)
        .expect("plain owner/passport/wallet references should remain allowed");

    assert_eq!(
        pointer.owner_passport_subject.as_deref(),
        Some("passport:creator")
    );
    assert_eq!(
        pointer.owner_wallet_account.as_deref(),
        Some("acct_creator")
    );
}

fn insert_authority_field(value: &mut Value, field: &str) {
    value
        .as_object_mut()
        .expect("test fixture is an object")
        .insert(field.to_owned(), json!("forbidden-authority-claim"));
}
