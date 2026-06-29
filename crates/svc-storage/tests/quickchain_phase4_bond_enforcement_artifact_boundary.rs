#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 3 controlled bond-enforcement artifact boundary tests.
//! RO:WHY — Controlled internal bond-enforcement artifacts may be stored by b3,
//! but svc-storage must not treat them as paid-unlock, wallet, ledger, bond,
//! slash, staking, liquidity, bridge, or public-market authority.
//! RO:INTERACTS — MemoryStorage, AccountingExportRequest, UsageEventDto, source boundary.
//! RO:INVARIANTS — b3 proves bytes only; cache/storage cannot unlock paid content;
//! controlled enforcement artifacts do not authorize lifecycle decisions.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — rejects Phase 4 Round 3 enforcement authority smuggling.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase4_bond_enforcement_artifact_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use serde_json::json;
use svc_storage::{
    accounting::{exporter::AccountingExportRequest, UsageEventDto},
    storage::{MemoryStorage, Storage},
};

const PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS: &[&str] = &[
    "enforcement_id",
    "enforcement_status",
    "enforcement_decision",
    "enforcement_action",
    "bond_enforcement_decision",
    "bond_enforcement_confirmation",
    "bond_enforcement_receipt",
    "bond_account_id",
    "bond_account_status",
    "bond_operator_subject",
    "slash_reserved_minor",
    "slash_reserve_minor",
    "reserve_slash",
    "release_slash_reserve",
    "capture_slash_reserve",
    "slash_capture",
    "slash_release",
    "slash_reserve",
    "controlled_bond_enforcement",
    "apply_controlled_bond_enforcement",
    "execute_bond_enforcement",
    "commit_bond_enforcement",
    "policy_gated_slash",
    "governance_approval_ref",
    "operator_approval_ref",
    "wallet_receipt",
    "ledger_receipt",
    "wallet_mutation",
    "ledger_mutation",
    "balance_side_effect",
    "paid_unlock",
    "paid_unlock_from_enforcement",
    "unlock_from_enforcement_artifact",
    "cache_only_unlock",
    "cache_unlock_authority",
    "validator_reward",
    "validator_reward_receipt",
    "public_staking_market",
    "liquidity_pool",
    "bridge_settlement",
    "external_settlement",
    "solana",
    "rox",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn read_sources(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| read(crate_dir().join(path)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn cid_for(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn assert_canonical_b3(cid: &str) {
    let rest = cid.strip_prefix("b3:").expect("cid must have b3 prefix");
    assert_eq!(rest.len(), 64, "b3 cid must have 64 lowercase hex chars");
    assert!(
        rest.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "b3 cid must be lowercase hex"
    );
}

fn accounting_export_json_with_top_level(field: &str) -> String {
    json!({
        "schema": "svc-storage.usage-events.v1",
        "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid": "wallet_txid_display_only",
        "source_service": "svc-storage",
        "events": [
            {
                "timestamp_ms": 1_u64,
                "tenant": 7_u128,
                "subject": "svc-storage",
                "metric_kind": "bytes_stored",
                "value": 4096_u64,
                "source_service": "svc-storage",
                "region": "dev",
                "route": "/paid/o"
            }
        ],
        field: "client-supplied-enforcement-authority"
    })
    .to_string()
}

fn accounting_export_json_with_nested_event(field: &str) -> String {
    json!({
        "schema": "svc-storage.usage-events.v1",
        "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid": "wallet_txid_display_only",
        "source_service": "svc-storage",
        "events": [
            {
                "timestamp_ms": 1_u64,
                "tenant": 7_u128,
                "subject": "svc-storage",
                "metric_kind": "bytes_stored",
                "value": 4096_u64,
                "source_service": "svc-storage",
                "region": "dev",
                "route": "/paid/o",
                field: "client-supplied-enforcement-authority"
            }
        ]
    })
    .to_string()
}

#[test]
fn storage_accounting_export_rejects_round3_enforcement_authority_fields() {
    let clean: &'static str = r#"{
        "schema": "svc-storage.usage-events.v1",
        "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid": "wallet_txid_display_only",
        "source_service": "svc-storage",
        "events": [
            {
                "timestamp_ms": 1,
                "tenant": 7,
                "subject": "svc-storage",
                "metric_kind": "bytes_stored",
                "value": 4096,
                "source_service": "svc-storage",
                "region": "dev",
                "route": "/paid/o"
            }
        ]
    }"#;

    serde_json::from_str::<AccountingExportRequest>(clean)
        .expect("clean accounting export request should deserialize");

    for field in PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS {
        let top_level = Box::leak(accounting_export_json_with_top_level(field).into_boxed_str());
        assert!(
            serde_json::from_str::<AccountingExportRequest>(top_level).is_err(),
            "AccountingExportRequest must reject top-level Phase 4 Round 3 authority field: {field}"
        );

        let nested = Box::leak(accounting_export_json_with_nested_event(field).into_boxed_str());
        assert!(
            serde_json::from_str::<AccountingExportRequest>(nested).is_err(),
            "UsageEventDto must reject nested Phase 4 Round 3 authority field: {field}"
        );
    }
}

#[test]
fn storage_usage_events_remain_metering_not_bond_enforcement_or_paid_unlock_authority() {
    let event = UsageEventDto {
        timestamp_ms: 1,
        tenant: 7,
        subject: "svc-storage".to_string(),
        metric_kind: "bytes_stored",
        value: 4096,
        source_service: "svc-storage",
        region: "dev".to_string(),
        route: "/paid/o",
    };

    let request = AccountingExportRequest::new(
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid_display_only",
        &[event],
    );

    let encoded = serde_json::to_string(&request).expect("accounting export should serialize");

    for forbidden in PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS {
        assert!(
            !encoded.contains(forbidden),
            "storage usage/accounting export must not expose Phase 4 Round 3 authority vocabulary: {forbidden}"
        );
    }

    assert!(encoded.contains(r#""metric_kind":"bytes_stored""#));
    assert!(encoded.contains(r#""source_service":"svc-storage""#));
}

#[tokio::test]
async fn enforcement_artifact_bytes_store_by_b3_without_unlock_slash_or_wallet_authority() {
    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.phase4-enforcement-artifact.v1","kind":"reserve_slash_copy","note":"opaque bytes only","paid_unlock":false}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.phase4-enforcement-artifact.v1","kind":"release_slash_reserve_copy","note":"opaque bytes only","wallet_mutation":false}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.phase4-enforcement-artifact.v1","kind":"capture_slash_reserve_copy","note":"opaque bytes only","ledger_mutation":false}"#,
        ),
    ];

    let store = MemoryStorage::new();

    for body in artifacts {
        let cid = cid_for(body.as_ref());
        assert_canonical_b3(&cid);

        store
            .put(&cid, body.clone())
            .await
            .expect("opaque Phase 4 Round 3 artifact bytes should store by b3 cid");

        assert!(
            store
                .exists(&cid)
                .await
                .expect("exists check should succeed"),
            "stored Phase 4 Round 3 artifact should be discoverable by exact b3 cid"
        );

        let head = store.head(&cid).await.expect("head should succeed");
        assert_eq!(head.len, body.len() as u64);
        assert_eq!(
            head.etag,
            format!("\"{}\"", blake3::hash(body.as_ref()).to_hex())
        );

        let full = store
            .get_full(&cid)
            .await
            .expect("full artifact read should succeed");
        assert_eq!(full, body);

        let (prefix, total) = store
            .get_range(&cid, 0, 3)
            .await
            .expect("range artifact read should succeed");
        assert_eq!(total, body.len() as u64);
        assert_eq!(&prefix[..], &body[..4]);
    }
}

#[test]
fn storage_source_does_not_implement_phase4_round3_bond_enforcement_authority() {
    let source = strip_line_comments(&read_sources(&[
        "src/accounting/mod.rs",
        "src/accounting/exporter.rs",
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/paid_estimate.rs",
        "src/http/routes/paid_object.rs",
        "src/policy/economics.rs",
        "src/policy/paid_write.rs",
        "src/policy/settlement.rs",
        "src/storage/cache.rs",
        "src/storage/cas.rs",
        "src/storage/mod.rs",
        "src/storage/fs.rs",
    ]));
    let lower = source.to_ascii_lowercase();

    for forbidden in [
        "quickchainbondenforcement",
        "bond_enforcement_decision",
        "bond_enforcement_confirmation",
        "bond_enforcement_route",
        "controlled_bond_enforcement",
        "apply_controlled_bond_enforcement",
        "execute_bond_enforcement",
        "commit_bond_enforcement",
        "reserve_slash",
        "release_slash_reserve",
        "capture_slash_reserve",
        "slash_reserved",
        "slash_capture",
        "slash_release",
        "policy_gated_slash",
        "governance_approval_ref",
        "operator_approval_ref",
        "paid_unlock_from_enforcement",
        "unlock_from_enforcement_artifact",
        "cache_only_unlock",
        "cache_unlock_authority",
        "validator_reward_receipt",
        "public_staking_market",
        "liquidity_pool",
        "bridge_settlement",
        "external_settlement",
        "solana",
        "rox",
        "ron_ledger::",
    ] {
        assert!(
            !lower.contains(forbidden),
            "svc-storage source must not implement Phase 4 Round 3 bond enforcement authority via `{forbidden}`"
        );
    }
}
