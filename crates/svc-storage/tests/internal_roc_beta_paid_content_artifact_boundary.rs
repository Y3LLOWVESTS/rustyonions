//! RO:WHAT — Internal ROC Beta paid-content artifact boundary tests for svc-storage.
//! RO:WHY — Phase 1 Round 1 must prove post/comment/article/content_view artifacts can be stored by b3 without storage becoming payment truth.
//! RO:INTERACTS — storage::MemoryStorage, Storage trait, accounting::UsageEventDto.
//! RO:INVARIANTS — b3 proves bytes; storage bytes/manifests/cache/usage events are not receipt, balance, entitlement, finality, bridge, staking, liquidity, or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — no config changes.
//! RO:SECURITY — no cache-only unlock; no client-supplied receipt/finality/root fields accepted by beta artifact metadata.
//! RO:TEST — cargo test -p svc-storage --test internal_roc_beta_paid_content_artifact_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_storage::{
    accounting::UsageEventDto,
    storage::{MemoryStorage, Storage},
};

const POST_BYTES: &[u8] =
    br#"{"kind":"post","title":"Internal ROC beta post","body":"paid post bytes"}"#;
const COMMENT_BYTES: &[u8] =
    br#"{"kind":"comment","title":"Internal ROC beta comment","body":"paid comment bytes"}"#;
const ARTICLE_BYTES: &[u8] =
    br#"{"kind":"article","title":"Internal ROC beta article","body":"paid article bytes"}"#;
const VIEW_BYTES: &[u8] =
    br#"{"kind":"content_view","title":"Internal ROC beta view","body":"generic content view bytes"}"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPaidContentDescriptor {
    version: u16,
    action: String,
    content_kind: String,
    artifact_cid: String,
    title: String,
    display_policy_hint: String,
    backend_access_source: String,
    price_hint_minor: String,
    references: Vec<String>,
}

#[tokio::test]
async fn paid_post_comment_article_and_content_view_artifacts_store_by_b3_not_payment_truth() {
    let store = MemoryStorage::new();

    for (action, kind, body) in [
        ("paid_post", "post", POST_BYTES),
        ("paid_comment", "comment", COMMENT_BYTES),
        ("paid_article", "article", ARTICLE_BYTES),
        ("content_view", "content_view", VIEW_BYTES),
    ] {
        let artifact = Bytes::from_static(body);
        let artifact_cid = cid_for_bytes(&artifact);
        assert_canonical_b3(&artifact_cid);

        store
            .put(&artifact_cid, artifact.clone())
            .await
            .expect("paid-content artifact bytes should store by b3");

        assert!(
            store
                .exists(&artifact_cid)
                .await
                .expect("exists check should succeed"),
            "{action} artifact should be discoverable by exact b3 cid"
        );

        let head = store
            .head(&artifact_cid)
            .await
            .expect("head should succeed");
        assert_eq!(head.len, artifact.len() as u64);
        assert_eq!(
            head.etag,
            format!("\"{}\"", blake3::hash(artifact.as_ref()).to_hex()),
            "ETag must remain byte/content hash, not payment proof"
        );

        let full = store
            .get_full(&artifact_cid)
            .await
            .expect("full artifact read should succeed");
        assert_eq!(full, artifact);

        let (range, total) = store
            .get_range(&artifact_cid, 0, 3)
            .await
            .expect("bounded range read should succeed");
        assert_eq!(total, artifact.len() as u64);
        assert_eq!(&range[..], &artifact[..4]);

        let descriptor = StoredPaidContentDescriptor {
            version: 1,
            action: action.to_owned(),
            content_kind: kind.to_owned(),
            artifact_cid: artifact_cid.clone(),
            title: format!("Internal ROC beta {kind}"),
            display_policy_hint: "requires_backend_wallet_ledger_access".to_owned(),
            backend_access_source: "svc-wallet/ron-ledger receipt path".to_owned(),
            price_hint_minor: "70".to_owned(),
            references: vec![artifact_cid],
        };

        let descriptor_json =
            serde_json::to_value(&descriptor).expect("descriptor should serialize");
        assert_json_has_no_authority_keys(&descriptor_json);
        assert_eq!(
            descriptor_json["display_policy_hint"], "requires_backend_wallet_ledger_access",
            "paid metadata is display/policy input only"
        );
        assert_eq!(
            descriptor_json["backend_access_source"], "svc-wallet/ron-ledger receipt path",
            "storage may label required backend source but cannot create it"
        );
    }
}

#[test]
fn paid_content_storage_descriptor_rejects_authority_poison_fields() {
    for forbidden_field in FORBIDDEN_AUTHORITY_FIELDS {
        let artifact_cid = cid_for_slice(POST_BYTES);
        let mut value = json!({
            "version": 1,
            "action": "paid_post",
            "content_kind": "post",
            "artifact_cid": artifact_cid,
            "title": "Internal ROC beta post",
            "display_policy_hint": "requires_backend_wallet_ledger_access",
            "backend_access_source": "svc-wallet/ron-ledger receipt path",
            "price_hint_minor": "70",
            "references": [artifact_cid]
        });

        value
            .as_object_mut()
            .expect("descriptor object")
            .insert((*forbidden_field).to_owned(), json!(true));

        let err = serde_json::from_value::<StoredPaidContentDescriptor>(value)
            .expect_err("paid-content storage descriptor must reject authority poison");

        assert!(
            err.to_string().contains("unknown field"),
            "field {forbidden_field:?} should reject as unknown, got: {err}"
        );
    }
}

#[test]
fn storage_usage_events_are_metering_only_not_receipt_balance_or_unlock_truth() {
    let artifact_cid = cid_for_slice(ARTICLE_BYTES);

    let events = vec![
        UsageEventDto {
            timestamp_ms: 1_850_000,
            tenant: 77,
            subject: "svc_storage:paid_article".to_owned(),
            metric_kind: "bytes_stored",
            value: ARTICLE_BYTES.len() as u64,
            source_service: "svc-storage",
            region: "local".to_owned(),
            route: "/paid/o",
        },
        UsageEventDto {
            timestamp_ms: 1_850_001,
            tenant: 77,
            subject: "svc_storage:content_view".to_owned(),
            metric_kind: "request_ok",
            value: 1,
            source_service: "svc-storage",
            region: "local".to_owned(),
            route: "/paid/o",
        },
    ];

    let json = serde_json::to_value(&events).expect("usage events should serialize");
    assert_json_has_no_authority_keys(&json);

    let text = json.to_string();
    assert!(text.contains("bytes_stored"));
    assert!(text.contains("request_ok"));
    assert!(text.contains("svc-storage"));
    assert!(
        !text.contains(&artifact_cid),
        "usage events should not need to expose object cid as payment authority"
    );
}

#[test]
fn storage_source_does_not_construct_paid_authority_shortcuts_for_artifacts() {
    let source = read_crate_sources(&[
        "src/storage/mod.rs",
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/paid_object.rs",
        "src/accounting/mod.rs",
        "src/accounting/exporter.rs",
        "src/policy/paid_write.rs",
        "src/policy/settlement.rs",
    ])
    .to_ascii_lowercase();

    for required in [
        "b3:",
        "get_full",
        "get_range",
        "verifiedpaidwrite",
        "usageeventdto",
        "accountingexportrequest",
    ] {
        assert!(
            source.contains(required),
            "expected storage source to retain paid/b3/artifact seam `{required}`"
        );
    }

    for forbidden in [
        "paid_from_b3",
        "paid_from_bytes",
        "paid_from_cache",
        "unlock_from_storage",
        "unlock_from_cache",
        "unlock_from_b3",
        "receipt_from_storage",
        "balance_from_storage",
        "finality_from_storage",
        "state_root_from_storage",
        "checkpoint_from_storage",
        "bridge_from_storage",
        "staking_from_storage",
        "liquidity_from_storage",
        "raw_engagement_mints_roc",
    ] {
        assert!(
            !source.contains(forbidden),
            "svc-storage source must not construct paid authority shortcut `{forbidden}`"
        );
    }
}

fn cid_for_slice(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn cid_for_bytes(bytes: &Bytes) -> String {
    cid_for_slice(bytes.as_ref())
}

fn assert_canonical_b3(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected b3:<64 lowercase hex>, got {value}"
    );

    let hex = &value[3..];
    assert_eq!(hex.len(), 64, "expected 64 hex chars, got {value}");
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase hex, got {value}"
    );
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
                    "storage artifact/metadata JSON must not expose authority key `{key}`"
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
