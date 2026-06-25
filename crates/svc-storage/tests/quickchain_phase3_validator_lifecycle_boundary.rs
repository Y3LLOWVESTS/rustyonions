#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle boundary tests for svc-storage.
//! RO:WHY — Validator rotation, revocation, equivocation evidence, replay challenges, downtime/degraded status, and governance parameter updates may be stored only as opaque bytes by b3; they must not become paid-unlock, wallet, ledger, staking, slashing, bridge, or settlement authority.
//! RO:INTERACTS — MemoryStorage, AccountingExportRequest, UsageEventDto, paid storage source boundary.
//! RO:INVARIANTS — b3 proves bytes only; wallet/ledger receipts prove payment; cache/storage artifacts do not authorize paid unlocks or validator lifecycle decisions.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — rejects lifecycle authority smuggling and preserves storage-as-bytes, accounting-as-metering, wallet/ledger-as-truth boundaries.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase3_validator_lifecycle_boundary.

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

const PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS: &[&str] = &[
    "validator_rotation",
    "validator_rotation_epoch",
    "validator_rotation_decision",
    "validator_revocation",
    "validator_revocation_reason",
    "validator_revocation_decision",
    "validator_lifecycle_decision",
    "validator_lifecycle_status",
    "validator_lifecycle_rejection_code",
    "equivocation_evidence",
    "double_attestation_evidence",
    "split_brain_evidence",
    "replay_challenge_evidence",
    "invalid_attestation_evidence",
    "validator_downtime_status",
    "validator_degraded_status",
    "downtime_report",
    "governance_parameter_update",
    "validator_set_parameter_update",
    "quorum_parameter_update",
    "checkpoint_parameter_update",
    "slash_evidence",
    "slashing",
    "staking_power",
    "validator_bond",
    "bonded_economics",
    "validator_reward",
    "bridge_settlement",
    "external_settlement",
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

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "target")
        {
            continue;
        }

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
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
        field: "client-supplied-lifecycle-authority"
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
                field: "client-supplied-lifecycle-authority"
            }
        ]
    })
    .to_string()
}

#[test]
fn storage_accounting_export_rejects_validator_lifecycle_authority_fields() {
    let clean = r#"{
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

    for field in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        let top_level = Box::leak(accounting_export_json_with_top_level(field).into_boxed_str());
        assert!(
            serde_json::from_str::<AccountingExportRequest>(top_level).is_err(),
            "AccountingExportRequest must reject top-level Phase 3 Round 2 lifecycle authority field: {field}"
        );

        let nested = Box::leak(accounting_export_json_with_nested_event(field).into_boxed_str());
        assert!(
            serde_json::from_str::<AccountingExportRequest>(nested).is_err(),
            "UsageEventDto must reject nested Phase 3 Round 2 lifecycle authority field: {field}"
        );
    }
}

#[test]
fn storage_usage_events_remain_metering_not_lifecycle_or_paid_unlock_authority() {
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

    let value = serde_json::to_value(&request).expect("accounting export should serialize");

    for field in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert!(
            !value.to_string().contains(field),
            "storage usage/accounting export must not expose lifecycle authority vocabulary: {field}"
        );
    }

    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "paid_unlock",
        "settlement_finality",
        "finalized",
        "anchored",
        "cache_only_unlock",
    ] {
        assert!(
            !value.to_string().contains(forbidden),
            "storage usage/accounting export must remain derivative metering only: {forbidden}"
        );
    }
}

#[tokio::test]
async fn lifecycle_evidence_bytes_store_by_b3_without_unlock_or_validator_authority() {
    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.validator-lifecycle.evidence.v1","kind":"equivocation","note":"opaque bytes only"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.validator-lifecycle.evidence.v1","kind":"replay_challenge","note":"opaque bytes only"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.validator-lifecycle.evidence.v1","kind":"downtime","note":"opaque bytes only"}"#,
        ),
    ];

    let store = MemoryStorage::new();

    for body in artifacts {
        let cid = cid_for(body.as_ref());

        assert!(
            cid.starts_with("b3:") && cid.len() == 67,
            "test CID must be canonical b3:<64 hex>"
        );

        store
            .put(&cid, body.clone())
            .await
            .expect("opaque lifecycle evidence bytes should store by b3 cid");

        assert!(
            store
                .exists(&cid)
                .await
                .expect("exists check should succeed"),
            "stored lifecycle evidence bytes should be discoverable by exact b3 cid"
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
            .expect("full lifecycle artifact read should succeed");
        assert_eq!(full, body);

        let (prefix, total) = store
            .get_range(&cid, 0, 3)
            .await
            .expect("range lifecycle artifact read should succeed");
        assert_eq!(total, body.len() as u64);
        assert_eq!(&prefix[..], &body[..4]);
    }
}

#[test]
fn storage_source_does_not_construct_validator_lifecycle_or_cache_unlock_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "validator_lifecycle_decision",
            "validator_lifecycle_status",
            "validator_rotation_decision",
            "validator_revocation_decision",
            "equivocation_evidence",
            "double_attestation_evidence",
            "split_brain_evidence",
            "replay_challenge_evidence",
            "invalid_attestation_evidence",
            "downtime_report",
            "validator_degraded_status",
            "governance_parameter_update",
            "admit_validator",
            "rotate_validator",
            "revoke_validator",
            "slash_validator",
            "stake_validator",
            "validator_reward",
            "mint_from_storage",
            "storage_validator_runtime",
            "cache_validator_authority",
            "cache_lifecycle_authority",
            "cache_unlock_authority",
            "cache_only_unlock",
            "unlock_from_lifecycle",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not construct Phase 3 Round 2 validator lifecycle/cache/economy authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
