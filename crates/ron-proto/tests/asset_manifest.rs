use ron_proto::{
    canonical_crab_asset_url, AssetKind, AssetManifestV1, AssetMetadataV1, AssetOwnerV1,
    AssetProvenanceV1, AssetValidationError, ContentId, PayoutRole, PayoutSplitV1, PayoutTarget,
    ASSET_MANIFEST_VERSION,
};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn cid(s: &str) -> ContentId {
    s.parse().expect("valid ContentId")
}

fn valid_manifest() -> AssetManifestV1 {
    AssetManifestV1 {
        version: ASSET_MANIFEST_VERSION,
        asset_cid: cid(ASSET_CID),
        asset_kind: AssetKind::Image,
        manifest_cid: Some(cid(MANIFEST_CID)),
        owner: AssetOwnerV1 {
            passport_subject: "passport:main:alice".to_owned(),
            wallet_account: "acct_creator_alice".to_owned(),
        },
        payout: PayoutTarget {
            default_action: "asset_publish".to_owned(),
            recipient_account: "acct_creator_alice".to_owned(),
            splits: Vec::new(),
        },
        metadata: AssetMetadataV1 {
            title: "Launch image".to_owned(),
            description: Some("First WEB3_2 asset manifest test image.".to_owned()),
            tags: vec!["art".to_owned(), "demo".to_owned()],
            license: Some("CC-BY-4.0".to_owned()),
            content_type: Some("image/png".to_owned()),
        },
        provenance: AssetProvenanceV1 {
            created_at_ms: 1_776_000_000_000,
            source: Some("local-upload".to_owned()),
            parent_cids: Vec::new(),
        },
        storage: None,
        receipts: Vec::new(),
        curator: None,
    }
}

#[test]
fn valid_manifest_validates_and_generates_clean_crab_url() {
    let manifest = valid_manifest();

    manifest.validate().expect("manifest validates");
    manifest
        .validate_for_crab_suffix("IMAGE")
        .expect("kind matches suffix case-insensitively");

    assert_eq!(
        canonical_crab_asset_url(&manifest.asset_cid, manifest.asset_kind),
        "crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image"
    );
}

#[test]
fn manifest_json_roundtrips_with_stable_core_fields() {
    let manifest = valid_manifest();

    let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    assert!(json.contains("\"asset_cid\""));
    assert!(json.contains("\"asset_kind\""));
    assert!(json.contains("\"owner\""));
    assert!(json.contains("\"payout\""));
    assert!(json.contains("\"metadata\""));

    let decoded: AssetManifestV1 = serde_json::from_str(&json).expect("deserialize manifest");
    assert_eq!(decoded, manifest);
    decoded.validate().expect("decoded manifest validates");
}

#[test]
fn unknown_manifest_fields_reject() {
    let manifest = valid_manifest();
    let mut value = serde_json::to_value(manifest).expect("manifest to value");
    value
        .as_object_mut()
        .expect("object")
        .insert("unexpected".to_owned(), serde_json::json!(true));

    let err = serde_json::from_value::<AssetManifestV1>(value).expect_err("unknown field rejects");
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn invalid_cid_rejects_during_deserialize() {
    let mut value = serde_json::to_value(valid_manifest()).expect("manifest to value");
    value
        .as_object_mut()
        .expect("object")
        .insert("asset_cid".to_owned(), serde_json::json!("b3:ABC"));

    let err = serde_json::from_value::<AssetManifestV1>(value).expect_err("bad CID rejects");
    assert!(
        err.to_string().contains("hex length") || err.to_string().contains("lowercase"),
        "unexpected error: {err}"
    );
}

#[test]
fn missing_owner_rejects_during_deserialize() {
    let mut value = serde_json::to_value(valid_manifest()).expect("manifest to value");
    value.as_object_mut().expect("object").remove("owner");

    let err = serde_json::from_value::<AssetManifestV1>(value).expect_err("missing owner rejects");
    assert!(err.to_string().contains("missing field"));
}

#[test]
fn empty_owner_passport_rejects_during_validation() {
    let mut manifest = valid_manifest();
    manifest.owner.passport_subject.clear();

    let err = manifest.validate().expect_err("empty owner rejects");
    assert!(matches!(
        err,
        AssetValidationError::EmptyField {
            field: "owner.passport_subject"
        }
    ));
}

#[test]
fn empty_payout_recipient_rejects_during_validation() {
    let mut manifest = valid_manifest();
    manifest.payout.recipient_account.clear();

    let err = manifest
        .validate()
        .expect_err("empty payout recipient rejects");
    assert!(matches!(
        err,
        AssetValidationError::EmptyField {
            field: "payout.recipient_account"
        }
    ));
}

#[test]
fn payout_splits_must_total_exactly_10000_bps() {
    let mut manifest = valid_manifest();
    manifest.payout.splits = vec![
        PayoutSplitV1 {
            role: PayoutRole::Creator,
            account: "acct_creator_alice".to_owned(),
            bps: 7_000,
        },
        PayoutSplitV1 {
            role: PayoutRole::StorageProvider,
            account: "acct_provider_1".to_owned(),
            bps: 2_000,
        },
    ];

    let err = manifest.validate().expect_err("bad bps total rejects");
    assert!(matches!(
        err,
        AssetValidationError::InvalidBpsTotal {
            expected_bps: 10_000,
            actual_bps: 9_000
        }
    ));

    manifest.payout.splits[1].bps = 3_000;
    manifest.validate().expect("10_000 bps validates");
}

#[test]
fn duplicate_tags_reject() {
    let mut manifest = valid_manifest();
    manifest.metadata.tags = vec!["Meme".to_owned(), "meme".to_owned()];

    let err = manifest.validate().expect_err("duplicate tags reject");
    assert!(matches!(
        err,
        AssetValidationError::DuplicateTag { tag } if tag == "meme"
    ));
}

#[test]
fn kind_suffix_mismatch_rejects() {
    let manifest = valid_manifest();

    let err = manifest
        .validate_for_crab_suffix("video")
        .expect_err("wrong suffix rejects");

    assert!(matches!(
        err,
        AssetValidationError::AssetKindMismatch { expected, actual }
        if expected == "video" && actual == "image"
    ));
}

#[test]
fn asset_kind_suffixes_are_stable() {
    let cases = [
        (AssetKind::Image, "image"),
        (AssetKind::Video, "video"),
        (AssetKind::Music, "music"),
        (AssetKind::Song, "song"),
        (AssetKind::Article, "article"),
        (AssetKind::Post, "post"),
        (AssetKind::Comment, "comment"),
        (AssetKind::Page, "page"),
        (AssetKind::Site, "site"),
        (AssetKind::App, "app"),
        (AssetKind::Manifest, "manifest"),
    ];

    for (kind, suffix) in cases {
        assert_eq!(kind.suffix(), suffix);
        assert_eq!(kind.to_string(), suffix);
        assert!(kind.matches_suffix(&suffix.to_ascii_uppercase()));
    }
}
