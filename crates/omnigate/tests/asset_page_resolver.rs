//! RO:WHAT — Tests for omnigate WEB3_2 crab/b3 asset-page resolver.
//! RO:WHY — Batch 4/7 acceptance: deterministic parsing, read-only response composition, richer manifest hydration.
//! RO:INVARIANTS — no wallet/ledger/storage mutation; invalid crab URLs fail closed.

use omnigate::routes::v1::crab::{
    compose_asset_page, compose_asset_page_with_manifest, manifest_details_from_json,
    parse_b3_asset_segment, parse_crab_asset_url, resolver_counters, AssetManifestPointer,
    AssetParseError, ManifestHydrationError, StorageSummary,
};
use serde_json::json;

const H: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

#[test]
fn parses_canonical_crab_asset_url() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.image")).expect("valid crab URL");

    assert_eq!(parsed.raw_hash_hex, H);
    assert_eq!(parsed.asset_cid, ASSET_CID);
    assert_eq!(parsed.asset_kind, "image");
    assert_eq!(parsed.canonical_crab, format!("crab://{H}.image"));
}

#[test]
fn parses_http_b3_asset_segment() {
    let parsed = parse_b3_asset_segment(&format!("{H}.video")).expect("valid b3 asset segment");

    assert_eq!(parsed.raw_hash_hex, H);
    assert_eq!(parsed.asset_cid, ASSET_CID);
    assert_eq!(parsed.asset_kind, "video");
    assert_eq!(parsed.canonical_crab, format!("crab://{H}.video"));
}

#[test]
fn rejects_old_b3_slash_prefix() {
    let err = parse_crab_asset_url(&format!("crab://b3/{H}.image")).unwrap_err();

    assert_eq!(err, AssetParseError::B3SlashPrefixRejected);
    assert_eq!(err.code(), "b3_slash_prefix_rejected");
}

#[test]
fn rejects_uppercase_hash() {
    let err = parse_crab_asset_url(
        "crab://0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef.image",
    )
    .unwrap_err();

    assert_eq!(err, AssetParseError::InvalidHashCharacters);
}

#[test]
fn rejects_unknown_kind() {
    let err = parse_crab_asset_url(&format!("crab://{H}.binary")).unwrap_err();

    assert_eq!(err, AssetParseError::UnsupportedAssetKind);
}

#[test]
fn rejects_missing_kind() {
    let err = parse_crab_asset_url(&format!("crab://{H}")).unwrap_err();

    assert_eq!(err, AssetParseError::MissingAssetKind);
}

#[test]
fn rejects_path_and_query_in_asset_segment() {
    let err = parse_crab_asset_url(&format!("crab://{H}.image?x=1")).unwrap_err();
    assert_eq!(err, AssetParseError::QueryOrFragmentRejected);

    let err = parse_crab_asset_url(&format!("crab://{H}.image/extra")).unwrap_err();
    assert_eq!(err, AssetParseError::UnsupportedPath);
}

#[test]
fn composes_full_asset_page_from_pointer_and_storage() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.image")).expect("valid crab URL");

    let pointer = pointer();

    let storage = StorageSummary {
        available: true,
        size_bytes: Some(12_345),
        content_type: Some("image/png".to_owned()),
        provider_ref: Some("provider:local".to_owned()),
    };

    let page = compose_asset_page(parsed, Some(pointer), storage, Vec::new());

    assert_eq!(page.schema, "omnigate.asset-page.v1");
    assert_eq!(page.asset_cid, ASSET_CID);
    assert_eq!(page.asset_kind, "image");
    assert_eq!(page.manifest.status, "present");
    assert_eq!(page.manifest.hydration_status, "pointer_only");
    assert_eq!(page.manifest.manifest_cid.as_deref(), Some(MANIFEST_CID));
    let manifest_route = format!("/o/{MANIFEST_CID}");
    assert_eq!(
        page.links.manifest.as_deref(),
        Some(manifest_route.as_str())
    );
    assert_eq!(
        page.owner
            .as_ref()
            .map(|owner| owner.passport_subject.as_str()),
        Some("passport:main:alice")
    );
    assert_eq!(
        page.payout
            .as_ref()
            .map(|payout| payout.recipient_account.as_str()),
        Some("acct_creator_alice")
    );
    assert!(page.storage.available);
    assert_eq!(page.links.crab, format!("crab://{H}.image"));
    assert_eq!(page.links.raw, format!("/o/{ASSET_CID}"));
    assert!(page.metadata.is_none());
    assert!(page.receipts.is_empty());
    assert!(page.warnings.is_empty());
}

#[test]
fn extracts_manifest_details_from_real_manifest_json() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.image")).expect("valid crab URL");
    let bytes = manifest_json_bytes();

    let details =
        manifest_details_from_json(&parsed, MANIFEST_CID, &bytes).expect("manifest parses");

    assert_eq!(
        details
            .owner
            .as_ref()
            .map(|owner| owner.passport_subject.as_str()),
        Some("passport:main:manifest_owner")
    );
    assert_eq!(
        details
            .payout
            .as_ref()
            .map(|payout| payout.default_action.as_str()),
        Some("paid_content_view")
    );
    assert_eq!(
        details
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.title.as_deref()),
        Some("Crab Demo Image")
    );
    let expected_tags = vec!["demo".to_owned(), "image".to_owned()];
    assert_eq!(
        details
            .metadata
            .as_ref()
            .map(|metadata| metadata.tags.as_slice()),
        Some(expected_tags.as_slice())
    );
    assert_eq!(details.receipts.len(), 1);
    assert_eq!(details.receipts[0].tx_id, "tx_paid_storage_1");
    assert_eq!(details.receipts[0].receipt_kind, "paid_storage");
    assert_eq!(details.receipts[0].amount_minor_units, Some(84));
}

#[test]
fn manifest_details_reject_asset_kind_mismatch() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.video")).expect("valid crab URL");
    let bytes = manifest_json_bytes();

    let err = manifest_details_from_json(&parsed, MANIFEST_CID, &bytes).unwrap_err();

    assert_eq!(err, ManifestHydrationError::AssetKindMismatch);
    assert_eq!(err.code(), "manifest_asset_kind_mismatch");
}

#[test]
fn manifest_details_reject_manifest_cid_mismatch() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.image")).expect("valid crab URL");
    let bytes = manifest_json_bytes();
    let wrong_manifest_cid = "b3:1111111111111111111111111111111111111111111111111111111111111111";

    let err = manifest_details_from_json(&parsed, wrong_manifest_cid, &bytes).unwrap_err();

    assert_eq!(err, ManifestHydrationError::ManifestCidMismatch);
    assert_eq!(err.code(), "manifest_cid_mismatch");
}

#[test]
fn composes_hydrated_image_page_from_manifest_details() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.image")).expect("valid crab URL");
    let details = manifest_details_from_json(&parsed, MANIFEST_CID, &manifest_json_bytes())
        .expect("manifest parses");

    let storage = StorageSummary {
        available: true,
        size_bytes: Some(12_345),
        content_type: Some("image/png".to_owned()),
        provider_ref: Some("provider:local".to_owned()),
    };

    let page = compose_asset_page_with_manifest(
        parsed,
        Some(pointer()),
        storage,
        Some(details),
        Vec::new(),
    );

    assert_eq!(page.manifest.status, "present");
    assert_eq!(page.manifest.hydration_status, "hydrated");
    assert_eq!(
        page.owner
            .as_ref()
            .map(|owner| owner.passport_subject.as_str()),
        Some("passport:main:manifest_owner")
    );
    assert_eq!(
        page.payout
            .as_ref()
            .map(|payout| payout.recipient_account.as_str()),
        Some("acct_manifest_creator")
    );
    assert_eq!(
        page.metadata
            .as_ref()
            .and_then(|metadata| metadata.title.as_deref()),
        Some("Crab Demo Image")
    );
    assert_eq!(page.receipts.len(), 1);
    assert_eq!(page.receipts[0].tx_id, "tx_paid_storage_1");
    let manifest_route = format!("/o/{MANIFEST_CID}");
    assert_eq!(
        page.links.manifest.as_deref(),
        Some(manifest_route.as_str())
    );
    assert!(page.warnings.is_empty());
}

#[test]
fn composes_partial_asset_page_when_manifest_pointer_missing() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.image")).expect("valid crab URL");

    let storage = StorageSummary {
        available: true,
        size_bytes: Some(100),
        content_type: Some("image/jpeg".to_owned()),
        provider_ref: None,
    };

    let page = compose_asset_page(parsed, None, storage, Vec::new());

    assert_eq!(page.manifest.status, "missing");
    assert_eq!(page.manifest.hydration_status, "missing");
    assert!(page.manifest.manifest_cid.is_none());
    assert!(page.owner.is_none());
    assert!(page.payout.is_none());
    assert!(page.metadata.is_none());
    assert!(page.receipts.is_empty());
    assert!(page.storage.available);
    assert!(page
        .warnings
        .contains(&"manifest_pointer_missing".to_owned()));
}

#[test]
fn asset_page_json_wire_shape_is_stable() {
    let parsed = parse_crab_asset_url(&format!("crab://{H}.comment")).expect("valid crab URL");

    let page = compose_asset_page(
        parsed,
        None,
        StorageSummary {
            available: false,
            size_bytes: None,
            content_type: None,
            provider_ref: None,
        },
        vec!["storage_object_missing".to_owned()],
    );

    let json = serde_json::to_string_pretty(&page).expect("serialize page");

    assert!(json.contains("\"schema\""));
    assert!(json.contains("\"omnigate.asset-page.v1\""));
    assert!(json.contains("\"asset_cid\""));
    assert!(json.contains("\"asset_kind\""));
    assert!(json.contains("\"manifest\""));
    assert!(json.contains("\"hydration_status\""));
    assert!(json.contains("\"storage\""));
    assert!(json.contains("\"metadata\""));
    assert!(json.contains("\"receipts\""));
    assert!(json.contains("\"links\""));
    assert!(json.contains(&format!("crab://{H}.comment")));

    let decoded: serde_json::Value = serde_json::from_str(&json).expect("valid json");
    assert_eq!(decoded["schema"], "omnigate.asset-page.v1");
    assert_eq!(decoded["asset_cid"], ASSET_CID);
    assert_eq!(decoded["asset_kind"], "comment");
    assert_eq!(decoded["manifest"]["hydration_status"], "missing");
}

#[test]
fn resolver_counters_are_readable() {
    let (total, errors) = resolver_counters();

    assert!(total >= errors);
}

fn pointer() -> AssetManifestPointer {
    AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "image".to_owned(),
        manifest_cid: MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:main:alice".to_owned()),
        owner_wallet_account: Some("acct_creator_alice".to_owned()),
        updated_at_ms: 1_776_000_000_000,
    }
}

fn manifest_json_bytes() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": 1,
        "asset_cid": ASSET_CID,
        "asset_kind": "image",
        "manifest_cid": MANIFEST_CID,
        "owner": {
            "passport_subject": "passport:main:manifest_owner",
            "wallet_account": "acct_manifest_owner"
        },
        "payout": {
            "default_action": "paid_content_view",
            "recipient_account": "acct_manifest_creator",
            "splits": [
                {
                    "role": "creator",
                    "account": "acct_manifest_creator",
                    "bps": 10_000
                }
            ]
        },
        "metadata": {
            "title": "Crab Demo Image",
            "description": "A WEB3_2 crab://image demo asset.",
            "tags": ["demo", "image"],
            "license": "CC0-1.0",
            "content_type": "image/png"
        },
        "provenance": {
            "created_at_ms": 1_776_000_000_000u64,
            "source": "test-fixture",
            "parent_cids": []
        },
        "storage": {
            "available": true,
            "size_bytes": 12_345,
            "content_type": "image/png",
            "provider_ref": "provider:local",
            "raw_url": format!("/o/{ASSET_CID}")
        },
        "receipts": [
            {
                "tx_id": "tx_paid_storage_1",
                "receipt_kind": "paid_storage",
                "amount_minor_units": 84,
                "account": "acct_manifest_creator",
                "created_at_ms": 1_776_000_000_001u64
            }
        ],
        "curator": {
            "curator_passport_subject": "passport:curator:demo",
            "notes": "fixture",
            "tags": ["curated"]
        }
    }))
    .expect("manifest JSON serializes")
}
