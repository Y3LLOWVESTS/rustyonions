use ron_proto::{
    canonical_crab_asset_url, AssetKind, AssetManifestV1, AssetMetadataV1, AssetOwnerV1,
    AssetPageLinksV1, AssetPageV1, AssetProvenanceV1, AssetValidationError, ContentId,
    PayoutTarget, ReceiptKind, ReceiptRefV1, StorageAvailabilityV1, ASSET_MANIFEST_VERSION,
    ASSET_PAGE_VERSION,
};

const IMAGE_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const VIDEO_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn cid(s: &str) -> ContentId {
    s.parse().expect("valid ContentId")
}

fn owner() -> AssetOwnerV1 {
    AssetOwnerV1 {
        passport_subject: "passport:main:alice".to_owned(),
        wallet_account: "acct_creator_alice".to_owned(),
    }
}

fn payout() -> PayoutTarget {
    PayoutTarget {
        default_action: "paid_content_view".to_owned(),
        recipient_account: "acct_creator_alice".to_owned(),
        splits: Vec::new(),
    }
}

fn manifest() -> AssetManifestV1 {
    AssetManifestV1 {
        version: ASSET_MANIFEST_VERSION,
        asset_cid: cid(IMAGE_CID),
        asset_kind: AssetKind::Image,
        manifest_cid: None,
        owner: owner(),
        payout: payout(),
        metadata: AssetMetadataV1 {
            title: "Hydrated image".to_owned(),
            description: Some("Asset page response test.".to_owned()),
            tags: vec!["demo".to_owned()],
            license: None,
            content_type: Some("image/png".to_owned()),
        },
        provenance: AssetProvenanceV1 {
            created_at_ms: 1_776_000_000_000,
            source: Some("asset-page-test".to_owned()),
            parent_cids: Vec::new(),
        },
        storage: None,
        receipts: Vec::new(),
        curator: None,
    }
}

fn page() -> AssetPageV1 {
    let cid = cid(IMAGE_CID);
    let asset_kind = AssetKind::Image;

    AssetPageV1 {
        version: ASSET_PAGE_VERSION,
        asset_cid: cid.clone(),
        asset_kind,
        manifest: Some(manifest()),
        owner: Some(owner()),
        payout: Some(payout()),
        storage: StorageAvailabilityV1 {
            available: true,
            size_bytes: Some(12_345),
            content_type: Some("image/png".to_owned()),
            provider_ref: Some("provider:local-dev".to_owned()),
            raw_url: Some(format!("/o/{cid}")),
        },
        receipts: vec![ReceiptRefV1 {
            tx_id: "tx_paid_storage_1".to_owned(),
            receipt_kind: ReceiptKind::PaidStorage,
            amount_minor_units: Some(50),
            account: Some("acct_creator_alice".to_owned()),
            created_at_ms: Some(1_776_000_000_001),
        }],
        links: AssetPageLinksV1 {
            canonical_crab: canonical_crab_asset_url(&cid, asset_kind),
            raw: Some(format!("/o/{cid}")),
            manifest: None,
            paid_view: Some("/v1/paid/content-view/prepare".to_owned()),
        },
        warnings: Vec::new(),
    }
}

#[test]
fn asset_page_validates() {
    page().validate().expect("page validates");
}

#[test]
fn asset_page_json_roundtrips() {
    let page = page();

    let json = serde_json::to_string_pretty(&page).expect("serialize page");
    assert!(json.contains("\"asset_cid\""));
    assert!(json.contains("\"asset_kind\""));
    assert!(json.contains("\"storage\""));
    assert!(json.contains("\"receipts\""));
    assert!(json.contains("\"canonical_crab\""));
    assert!(json
        .contains("crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image"));

    let decoded: AssetPageV1 = serde_json::from_str(&json).expect("deserialize page");
    assert_eq!(decoded, page);
    decoded.validate().expect("decoded page validates");
}

#[test]
fn asset_page_rejects_unknown_fields() {
    let mut value = serde_json::to_value(page()).expect("page to value");
    value
        .as_object_mut()
        .expect("object")
        .insert("extra".to_owned(), serde_json::json!("nope"));

    let err = serde_json::from_value::<AssetPageV1>(value).expect_err("unknown field rejects");
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn storage_metadata_rejects_oversized_content_type() {
    let mut page = page();
    page.storage.content_type = Some("x".repeat(129));

    let err = page.validate().expect_err("oversized content type rejects");
    assert!(matches!(
        err,
        AssetValidationError::FieldTooLong {
            field: "storage.content_type",
            max: 128,
            actual: 129
        }
    ));
}

#[test]
fn receipt_requires_tx_id() {
    let mut page = page();
    page.receipts[0].tx_id.clear();

    let err = page.validate().expect_err("empty receipt tx_id rejects");
    assert!(matches!(
        err,
        AssetValidationError::EmptyField {
            field: "receipts[].tx_id"
        }
    ));
}

#[test]
fn page_rejects_manifest_cid_mismatch() {
    let mut page = page();
    page.manifest.as_mut().expect("manifest").asset_cid = cid(VIDEO_CID);

    let err = page
        .validate()
        .expect_err("mismatched manifest cid rejects");
    assert!(matches!(
        err,
        AssetValidationError::AssetKindMismatch { .. }
    ));
}

#[test]
fn page_rejects_manifest_kind_mismatch() {
    let mut page = page();
    page.manifest.as_mut().expect("manifest").asset_kind = AssetKind::Video;

    let err = page
        .validate()
        .expect_err("mismatched manifest kind rejects");
    assert!(matches!(
        err,
        AssetValidationError::AssetKindMismatch { expected, actual }
        if expected == "image" && actual == "video"
    ));
}

#[test]
fn page_link_requires_crab_scheme() {
    let mut page = page();
    page.links.canonical_crab = "http://127.0.0.1:9080/v1/b3/hash.image".to_owned();

    let err = page
        .validate()
        .expect_err("non-crab canonical link rejects");
    assert!(matches!(
        err,
        AssetValidationError::EmptyField {
            field: "links.canonical_crab"
        }
    ));
}

#[test]
fn generic_missing_manifest_page_can_still_validate() {
    let cid = cid(IMAGE_CID);
    let page = AssetPageV1 {
        version: ASSET_PAGE_VERSION,
        asset_cid: cid.clone(),
        asset_kind: AssetKind::Image,
        manifest: None,
        owner: None,
        payout: None,
        storage: StorageAvailabilityV1 {
            available: false,
            size_bytes: None,
            content_type: None,
            provider_ref: None,
            raw_url: None,
        },
        receipts: Vec::new(),
        links: AssetPageLinksV1 {
            canonical_crab: canonical_crab_asset_url(&cid, AssetKind::Image),
            raw: None,
            manifest: None,
            paid_view: None,
        },
        warnings: vec!["manifest_not_found".to_owned()],
    };

    page.validate()
        .expect("generic missing-manifest page validates");
}
