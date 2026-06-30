//! RO:WHAT — Internal ROC Beta paid-content pointer boundary tests for svc-index.
//! RO:WHY — Phase 1 Round 1 must prove post/comment/article/content_view pointers can resolve as reference metadata without index becoming payment truth.
//! RO:INTERACTS — Store, manifest pointer DTOs, ResolveResponse/ProvidersResponse shapes.
//! RO:INVARIANTS — names/pointers/reference graphs are navigation/lookup only; index cannot create receipt, balance, entitlement, unlock, finality, bridge, staking, liquidity, or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — in-memory store only.
//! RO:SECURITY — pointer metadata may guide gateway/omnigate policy, but cannot unlock paid content alone.
//! RO:TEST — cargo test -p svc-index --test internal_roc_beta_paid_content_pointer_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_index::{
    store::Store,
    types::{
        normalize_asset_kind, normalize_b3_cid, normalize_site_name, AssetManifestPointer,
        ProviderEntry, ProvidersResponse, PutAssetManifestPointer, PutSiteManifestPointer,
        ResolveResponse, SiteManifestPointer,
    },
};

const POST_CID: &str = "b3:1111111111111111111111111111111111111111111111111111111111111111";
const COMMENT_CID: &str = "b3:2222222222222222222222222222222222222222222222222222222222222222";
const ARTICLE_CID: &str = "b3:3333333333333333333333333333333333333333333333333333333333333333";
const VIEW_CID: &str = "b3:4444444444444444444444444444444444444444444444444444444444444444";
const POST_MANIFEST_CID: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMENT_MANIFEST_CID: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const ARTICLE_MANIFEST_CID: &str =
    "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const VIEW_MANIFEST_CID: &str =
    "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaidPointerDisplayMetadata {
    version: u16,
    action: String,
    content_kind: String,
    crab_route: String,
    manifest_cid: String,
    artifact_cid: String,
    display_policy_hint: String,
    backend_access_source: String,
}

#[test]
fn paid_post_comment_article_and_content_view_pointers_are_lookup_metadata_only() {
    let store = Store::new(false).expect("memory store");

    let cases = [
        ("paid_post", "post", POST_CID, POST_MANIFEST_CID),
        ("paid_comment", "comment", COMMENT_CID, COMMENT_MANIFEST_CID),
        ("paid_article", "article", ARTICLE_CID, ARTICLE_MANIFEST_CID),
        ("content_view", "content_view", VIEW_CID, VIEW_MANIFEST_CID),
    ];

    for (idx, (action, kind, asset_cid, manifest_cid)) in cases.iter().enumerate() {
        assert_eq!(normalize_b3_cid(asset_cid).expect("asset cid"), *asset_cid);
        assert_eq!(
            normalize_b3_cid(manifest_cid).expect("manifest cid"),
            *manifest_cid
        );

        let pointer = AssetManifestPointer {
            version: 1,
            asset_cid: (*asset_cid).to_owned(),
            asset_kind: (*kind).to_owned(),
            manifest_cid: (*manifest_cid).to_owned(),
            owner_passport_subject: Some(format!("passport:creator:{kind}")),
            owner_wallet_account: Some(format!("acct_creator_{kind}")),
            updated_at_ms: 1_850_000 + idx as u64,
        };

        store
            .put_asset_manifest_pointer(&pointer)
            .expect("asset pointer should store");

        let fetched = store
            .get_asset_manifest_pointer(asset_cid)
            .expect("asset pointer should fetch");

        assert_eq!(fetched, pointer);

        let pointer_json = serde_json::to_value(&fetched).expect("pointer JSON");
        assert_json_has_no_authority_keys(&pointer_json);
        assert_eq!(
            pointer_json["owner_wallet_account"],
            format!("acct_creator_{kind}"),
            "owner wallet account is a reference string, not spend authority"
        );

        let display = PaidPointerDisplayMetadata {
            version: 1,
            action: (*action).to_owned(),
            content_kind: (*kind).to_owned(),
            crab_route: format!("crab://{asset_cid}.{kind}"),
            manifest_cid: (*manifest_cid).to_owned(),
            artifact_cid: (*asset_cid).to_owned(),
            display_policy_hint: "requires_backend_wallet_ledger_access".to_owned(),
            backend_access_source: "gateway/omnigate must verify wallet-ledger receipt".to_owned(),
        };

        let display_json = serde_json::to_value(&display).expect("display metadata JSON");
        assert_json_has_no_authority_keys(&display_json);
    }

    let site_pointer = SiteManifestPointer {
        version: 1,
        name: "crablink-beta.test".to_owned(),
        manifest_cid: POST_MANIFEST_CID.to_owned(),
        owner_passport_subject: Some("passport:site:creator".to_owned()),
        owner_wallet_account: Some("acct_site_creator".to_owned()),
        updated_at_ms: 1_850_100,
    };

    store
        .put_site_manifest_pointer(&site_pointer)
        .expect("site pointer should store");

    let fetched_site = store
        .get_site_manifest_pointer("crablink-beta.test")
        .expect("site pointer should fetch");

    assert_eq!(fetched_site, site_pointer);
    assert_json_has_no_authority_keys(&serde_json::to_value(&fetched_site).expect("site JSON"));

    store.put_manifest("name:crablink-beta-post", POST_MANIFEST_CID);
    assert_eq!(
        store.get_manifest("name:crablink-beta-post").as_deref(),
        Some(POST_MANIFEST_CID),
        "legacy name pointer resolves to manifest CID only"
    );
}

#[test]
fn paid_content_pointer_inputs_reject_receipt_balance_unlock_and_finality_poison() {
    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut asset_value = json!({
            "asset_kind": "post",
            "manifest_cid": POST_MANIFEST_CID,
            "owner_passport_subject": "passport:creator",
            "owner_wallet_account": "acct_creator",
            "updated_at_ms": 1
        });

        asset_value
            .as_object_mut()
            .expect("asset pointer object")
            .insert((*field).to_owned(), json!(true));

        let asset_err = serde_json::from_value::<PutAssetManifestPointer>(asset_value)
            .expect_err("asset pointer input must reject authority-shaped fields");

        assert!(
            asset_err.to_string().contains("unknown field"),
            "asset field {field:?} should reject as unknown, got: {asset_err}"
        );

        let mut site_value = json!({
            "manifest_cid": POST_MANIFEST_CID,
            "owner_passport_subject": "passport:site:creator",
            "owner_wallet_account": "acct_site_creator",
            "updated_at_ms": 1
        });

        site_value
            .as_object_mut()
            .expect("site pointer object")
            .insert((*field).to_owned(), json!(true));

        let site_err = serde_json::from_value::<PutSiteManifestPointer>(site_value)
            .expect_err("site pointer input must reject authority-shaped fields");

        assert!(
            site_err.to_string().contains("unknown field"),
            "site field {field:?} should reject as unknown, got: {site_err}"
        );
    }
}

#[test]
fn resolve_and_provider_responses_are_navigation_not_entitlement_truth() {
    let response = ResolveResponse {
        key: format!(
            "name:{}",
            normalize_site_name("CrabLink-Beta.Test").expect("site name")
        ),
        manifest: Some(POST_MANIFEST_CID.to_owned()),
        providers: vec![ProviderEntry {
            id: "provider:local:storage".to_owned(),
            region: Some("local".to_owned()),
            score: 0.9,
        }],
        etag: None,
        cached: false,
    };

    let response_json = serde_json::to_value(&response).expect("resolve response JSON");
    assert_json_has_no_authority_keys(&response_json);
    assert_eq!(
        response_json["manifest"], POST_MANIFEST_CID,
        "manifest pointer is navigation metadata only"
    );

    let providers = ProvidersResponse {
        cid: POST_CID.to_owned(),
        providers: response.providers.clone(),
        truncated: false,
        etag: None,
    };

    let providers_json = serde_json::to_value(&providers).expect("providers response JSON");
    assert_json_has_no_authority_keys(&providers_json);
    assert_eq!(providers_json["cid"], POST_CID);

    for field in FORBIDDEN_AUTHORITY_FIELDS {
        let mut poisoned = serde_json::to_value(&response).expect("resolve response JSON");
        poisoned
            .as_object_mut()
            .expect("resolve object")
            .insert((*field).to_owned(), json!(true));

        let err = serde_json::from_value::<ResolveResponse>(poisoned)
            .expect_err("resolve response must reject authority-shaped unknown fields");

        assert!(
            err.to_string().contains("unknown field"),
            "resolve field {field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn normalizers_preserve_content_and_navigation_identity_without_economic_authority() {
    assert_eq!(normalize_b3_cid(POST_CID).expect("b3 cid"), POST_CID);
    assert_eq!(
        normalize_b3_cid(&POST_CID[3..]).expect("bare b3 cid normalizes"),
        POST_CID
    );
    assert_eq!(normalize_asset_kind("POST").expect("post kind"), "post");
    assert_eq!(
        normalize_site_name("CrabLink-Beta.Test").expect("site name"),
        "crablink-beta.test"
    );

    for bad in [
        "receipt_id",
        "wallet_balance",
        "ledger_balance",
        "unlock_granted",
        "finalized",
        "bridge_proof",
        "staking_position_id",
        "../paid-post",
    ] {
        assert!(
            normalize_site_name(bad).is_err() || FORBIDDEN_AUTHORITY_FIELDS.contains(&bad),
            "bad navigation/authority-like form should not become safe economic proof: {bad}"
        );
    }
}

#[test]
fn index_source_does_not_construct_paid_authority_shortcuts() {
    let source = read_crate_sources(&[
        "src/types.rs",
        "src/store/mod.rs",
        "src/store/keys.rs",
        "src/pipeline/resolve.rs",
        "src/pipeline/providers.rs",
        "src/http/routes/index_manifests.rs",
        "src/http/routes/resolve.rs",
        "src/http/routes/providers.rs",
        "src/router.rs",
    ])
    .to_ascii_lowercase();

    for required in [
        "assetmanifestpointer",
        "sitemanifestpointer",
        "put_asset_manifest_pointer",
        "get_asset_manifest_pointer",
        "put_site_manifest_pointer",
        "get_site_manifest_pointer",
        "resolveresponse",
        "providersresponse",
    ] {
        assert!(
            source.contains(required),
            "expected index source to retain pointer/lookup seam `{required}`"
        );
    }

    for forbidden in [
        "paid_from_index",
        "paid_from_pointer",
        "paid_from_manifest",
        "paid_from_cache",
        "unlock_from_index",
        "unlock_from_pointer",
        "unlock_from_manifest",
        "receipt_from_index",
        "balance_from_index",
        "finality_from_index",
        "settlement_from_index",
        "state_root_from_index",
        "checkpoint_from_index",
        "bridge_from_index",
        "staking_from_index",
        "liquidity_from_index",
        "raw_engagement_mints_roc",
    ] {
        assert!(
            !source.contains(forbidden),
            "svc-index source must not construct paid authority shortcut `{forbidden}`"
        );
    }
}

const FORBIDDEN_AUTHORITY_FIELDS: &[&str] = &[
    "wallet_mutation",
    "ledger_mutation",
    "balance_truth",
    "receipt_truth",
    "paid_unlock_authority",
    "entitlement_truth",
    "client_finality_claim",
    "cache_unlock_authority",
    "gateway_receipt_truth",
    "omnigate_receipt_truth",
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
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
    "bridge_txid",
    "bridge_proof",
    "staking_position_id",
    "liquidity_pool_id",
    "external_settlement_id",
    "raw_engagement_mints_roc",
];

fn assert_json_has_no_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !FORBIDDEN_AUTHORITY_FIELDS
                        .iter()
                        .any(|forbidden| key == forbidden),
                    "index pointer/lookup JSON must not expose authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_json_has_no_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_json_has_no_authority_keys(nested);
            }
        }
        _ => {}
    }
}

fn read_crate_sources(paths: &[&str]) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = String::new();

    for path in paths {
        let full = root.join(path);
        out.push_str(
            &std::fs::read_to_string(&full)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", full.display())),
        );
        out.push('\n');
    }

    out
}
