//! RO:WHAT — HTTP/DTO contract tests for svc-index WEB3_2 manifest pointers.
//! RO:WHY — Replaces placeholder assertions with real schema and validation checks.
//! RO:INVARIANTS — unknown fields reject; pointer records contain no raw bytes or wallet mutation behavior.

use svc_index::types::{
    normalize_asset_kind, normalize_b3_cid, normalize_site_name, AssetManifestPointer,
    PutAssetManifestPointer,
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

#[test]
fn put_asset_manifest_pointer_rejects_unknown_fields() {
    let value = serde_json::json!({
        "asset_kind": "image",
        "manifest_cid": MANIFEST_CID,
        "unexpected": true
    });

    let err = serde_json::from_value::<PutAssetManifestPointer>(value)
        .expect_err("unknown fields must reject");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn asset_manifest_pointer_wire_shape_is_stable() {
    let pointer = AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "image".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:main:alice".to_owned()),
        owner_wallet_account: Some("acct_creator_alice".to_owned()),
        updated_at_ms: 1_776_000_000_000,
    };

    let json = serde_json::to_string_pretty(&pointer).expect("serialize pointer");
    assert!(json.contains("\"asset_cid\""));
    assert!(json.contains("\"asset_kind\""));
    assert!(json.contains("\"manifest_cid\""));
    assert!(!json.contains("raw_bytes"));

    let decoded: AssetManifestPointer = serde_json::from_str(&json).expect("deserialize pointer");
    assert_eq!(decoded, pointer);
}

#[test]
fn validators_accept_beta_asset_forms() {
    assert_eq!(normalize_b3_cid(ASSET_CID).unwrap(), ASSET_CID);
    assert_eq!(
        normalize_b3_cid("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap(),
        ASSET_CID
    );
    assert_eq!(normalize_asset_kind("IMAGE").unwrap(), "image");
    assert_eq!(normalize_asset_kind("music").unwrap(), "music");
    assert_eq!(normalize_asset_kind("podcast").unwrap(), "podcast");
    assert_eq!(normalize_asset_kind("post").unwrap(), "post");
    assert_eq!(
        normalize_site_name("SeaLobsta.COM").unwrap(),
        "sealobsta.com"
    );
}
