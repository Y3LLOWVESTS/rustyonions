//! RO:WHAT — Integration-level store tests for WEB3_2 manifest pointer records.
//! RO:WHY — Batch 3 requires asset/site pointer put + fetch without raw byte storage.
//! RO:INVARIANTS — store contains manifest pointers only; wallet/ledger are never called.

use svc_index::{
    store::Store,
    types::{AssetManifestPointer, SiteManifestPointer},
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

#[test]
fn store_roundtrips_asset_manifest_pointer() {
    let store = Store::new(false).expect("memory store");

    let pointer = AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "image".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:main:alice".to_owned()),
        owner_wallet_account: Some("acct_creator_alice".to_owned()),
        updated_at_ms: 1_776_000_000_000,
    };

    store
        .put_asset_manifest_pointer(&pointer)
        .expect("put asset pointer");

    let fetched = store
        .get_asset_manifest_pointer(ASSET_CID)
        .expect("fetch asset pointer");

    assert_eq!(fetched, pointer);
}

#[test]
fn store_roundtrips_site_manifest_pointer() {
    let store = Store::new(false).expect("memory store");

    let pointer = SiteManifestPointer {
        version: 1,
        name: "sealobsta.com".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:main:alice".to_owned()),
        owner_wallet_account: Some("acct_site_owner".to_owned()),
        updated_at_ms: 1_776_000_000_001,
    };

    store
        .put_site_manifest_pointer(&pointer)
        .expect("put site pointer");

    let fetched = store
        .get_site_manifest_pointer("sealobsta.com")
        .expect("fetch site pointer");

    assert_eq!(fetched, pointer);
}

#[test]
fn missing_pointers_return_none() {
    let store = Store::new(false).expect("memory store");

    assert!(store.get_asset_manifest_pointer(ASSET_CID).is_none());
    assert!(store.get_site_manifest_pointer("sealobsta.com").is_none());
}
