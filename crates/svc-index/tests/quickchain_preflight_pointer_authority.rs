//! RO:WHAT — DTO boundary tests proving svc-index pointers cannot smuggle paid/economic/QuickChain authority.
//! RO:WHY — Manifest pointers may carry owner metadata, but must not become receipts, balances, finality, unlocks, roots, or checkpoints.
//! RO:INTERACTS — svc_index::types manifest pointer DTOs and validation helpers.
//! RO:INVARIANTS — owner/passport/wallet fields are references only; unknown authority-shaped fields reject.
//! RO:TEST — run with `cargo test -p svc-index --test quickchain_preflight_pointer_authority`.

use svc_index::types::{
    normalize_optional_ref, AssetManifestPointer, PutAssetManifestPointer, PutSiteManifestPointer,
    SiteManifestPointer,
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn assert_unknown_field_rejects<T>(value: serde_json::Value, field: &str)
where
    T: serde::de::DeserializeOwned,
{
    let rendered = match serde_json::from_value::<T>(value) {
        Ok(_) => panic!("authority-shaped field {field:?} must reject"),
        Err(err) => err.to_string(),
    };

    assert!(
        rendered.contains("unknown field"),
        "expected unknown-field rejection for {field:?}, got: {rendered}"
    );
}

#[test]
fn put_asset_manifest_pointer_rejects_authority_shaped_fields() {
    let forbidden_fields = [
        "paid_from_index",
        "unlock_from_index",
        "receipt_from_index",
        "balance_from_index",
        "finality_from_index",
        "checkpoint_from_index",
        "validator_from_index",
        "root_from_index",
        "paid_from_manifest",
        "unlock_from_manifest",
        "receipt_hash",
        "receipt_root",
        "state_root",
        "checkpoint_hash",
        "settlement_status",
        "entitlement_granted",
        "unlock_granted",
        "spend_authority",
        "capture_authority",
    ];

    for field in forbidden_fields {
        let mut value = serde_json::json!({
            "asset_kind": "image",
            "manifest_cid": MANIFEST_CID
        });
        value
            .as_object_mut()
            .expect("test JSON object")
            .insert(field.to_owned(), serde_json::json!(true));
        assert_unknown_field_rejects::<PutAssetManifestPointer>(value, field);
    }
}

#[test]
fn put_site_manifest_pointer_rejects_authority_shaped_fields() {
    let forbidden_fields = [
        "paid_from_index",
        "unlock_from_index",
        "receipt_from_index",
        "balance_from_index",
        "finality_from_index",
        "checkpoint_from_index",
        "validator_from_index",
        "root_from_index",
        "paid_from_manifest",
        "unlock_from_manifest",
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "state_root",
        "checkpoint_hash",
        "settlement_status",
        "entitlement_granted",
        "unlock_granted",
        "spend_authority",
        "capture_authority",
    ];

    for field in forbidden_fields {
        let mut value = serde_json::json!({
            "manifest_cid": MANIFEST_CID
        });
        value
            .as_object_mut()
            .expect("test JSON object")
            .insert(field.to_owned(), serde_json::json!(true));
        assert_unknown_field_rejects::<PutSiteManifestPointer>(value, field);
    }
}

#[test]
fn stored_asset_pointer_wire_shape_has_no_paid_or_chain_authority() {
    let pointer = AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "image".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:main:alice".to_owned()),
        owner_wallet_account: Some("acct_creator_alice".to_owned()),
        updated_at_ms: 1_776_000_000_000,
    };

    let json = serde_json::to_value(&pointer).expect("serialize asset pointer");
    let object = json.as_object().expect("pointer must serialize as object");

    assert!(object.contains_key("owner_passport_subject"));
    assert!(object.contains_key("owner_wallet_account"));

    for forbidden in [
        "paid",
        "paid_from_index",
        "unlock",
        "unlock_from_index",
        "receipt",
        "receipt_id",
        "receipt_hash",
        "balance",
        "finality",
        "entitlement",
        "state_root",
        "receipt_root",
        "checkpoint",
        "checkpoint_hash",
        "validator",
        "settlement",
        "spend_authority",
        "capture_authority",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "asset manifest pointer must not expose authority field {forbidden:?}"
        );
    }
}

#[test]
fn stored_site_pointer_wire_shape_has_no_paid_or_chain_authority() {
    let pointer = SiteManifestPointer {
        version: 1,
        name: "sealobsta.com".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:main:alice".to_owned()),
        owner_wallet_account: Some("acct_site_owner".to_owned()),
        updated_at_ms: 1_776_000_000_001,
    };

    let json = serde_json::to_value(&pointer).expect("serialize site pointer");
    let object = json.as_object().expect("pointer must serialize as object");

    assert!(object.contains_key("owner_passport_subject"));
    assert!(object.contains_key("owner_wallet_account"));

    for forbidden in [
        "paid",
        "paid_from_index",
        "unlock",
        "unlock_from_index",
        "receipt",
        "receipt_id",
        "receipt_hash",
        "balance",
        "finality",
        "entitlement",
        "state_root",
        "receipt_root",
        "checkpoint",
        "checkpoint_hash",
        "validator",
        "settlement",
        "spend_authority",
        "capture_authority",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "site manifest pointer must not expose authority field {forbidden:?}"
        );
    }
}

#[test]
fn owner_wallet_metadata_is_allowed_but_normalized_as_reference_only() {
    let normalized = normalize_optional_ref(
        "owner_wallet_account",
        Some("  acct_creator_alice  ".to_owned()),
    )
    .expect("owner wallet reference should normalize");

    assert_eq!(normalized.as_deref(), Some("acct_creator_alice"));

    let empty = normalize_optional_ref("owner_wallet_account", Some("   ".to_owned()))
        .expect("empty optional owner wallet reference should normalize to none");

    assert_eq!(empty, None);
}
