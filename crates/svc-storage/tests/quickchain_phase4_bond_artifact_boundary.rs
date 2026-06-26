#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 1 bond artifact boundary tests for svc-storage.
//! RO:WHY — Storage may retain opaque bond/slash/evidence bytes by canonical b3,
//! but those bytes must not become paid-unlock, wallet, ledger, staking,
//! slashing, validator reward, liquidity, bridge, or external settlement authority.
//! RO:INTERACTS — MemoryStorage, AccountingExportRequest, UsageEventDto, source boundary.
//! RO:INVARIANTS — b3 proves bytes only; wallet/ledger receipts prove payment;
//! storage/cache artifacts do not authorize bond lifecycle decisions.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — rejects Phase 4 bond authority smuggling through storage/accounting DTOs.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary.

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

const PHASE4_ROUND1_BOND_AUTHORITY_KEYS: &[&str] = &[
    "bond_account_id",
    "bond_account_status",
    "bond_lifecycle_decision",
    "bond_lifecycle_status",
    "bond_intent_id",
    "bond_owner_account",
    "bond_locked_minor",
    "bond_pending_unlock_minor",
    "bond_evidence_reserved_minor",
    "slash_evidence",
    "slash_decision",
    "slash_capture",
    "automatic_slash",
    "auto_slash_now",
    "validator_reward",
    "validator_reward_receipt",
    "staking_power",
    "staking_pool",
    "public_staking_market",
    "liquidity_pool",
    "bond_receipt",
    "wallet_receipt",
    "ledger_receipt",
    "wallet_mutation",
    "ledger_mutation",
    "payout_execution",
    "paid_unlock",
    "cache_only_unlock",
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
    assert!(cid.starts_with("b3:"), "cid must start with b3:");
    assert_eq!(cid.len(), 67, "cid must be b3:<64 hex>");
    assert!(
        cid[3..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "cid must use lowercase hex"
    );
}

fn accounting_export_json_with_top_level(field: &str) -> String {
    json!({
        "schema": "svc-storage.usage-events.v1",
        "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid": "wallet_txid_display_only",
        "source_service": "svc-storage",
        "events": [],
        field: "client-supplied-phase4-bond-authority"
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
                field: "client-supplied-phase4-bond-authority"
            }
        ]
    })
    .to_string()
}

#[test]
fn storage_accounting_export_rejects_phase4_bond_authority_fields() {
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

    for field in PHASE4_ROUND1_BOND_AUTHORITY_KEYS {
        let top_level = Box::leak(accounting_export_json_with_top_level(field).into_boxed_str());
        assert!(
            serde_json::from_str::<AccountingExportRequest>(top_level).is_err(),
            "AccountingExportRequest must reject top-level Phase 4 bond authority field: {field}"
        );

        let nested = Box::leak(accounting_export_json_with_nested_event(field).into_boxed_str());
        assert!(
            serde_json::from_str::<AccountingExportRequest>(nested).is_err(),
            "UsageEventDto must reject nested Phase 4 bond authority field: {field}"
        );
    }
}

#[test]
fn storage_usage_event_export_remains_metering_not_bond_or_paid_unlock_authority() {
    let event = UsageEventDto {
        timestamp_ms: 1,
        tenant: 7,
        subject: "svc-storage".to_owned(),
        metric_kind: "bytes_stored",
        value: 4096,
        source_service: "svc-storage",
        region: "dev".to_owned(),
        route: "/paid/o",
    };

    let request = AccountingExportRequest::new(
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid_display_only",
        &[event],
    );

    let encoded = serde_json::to_string(&request).expect("accounting export should serialize");

    for forbidden in PHASE4_ROUND1_BOND_AUTHORITY_KEYS {
        assert!(
            !encoded.contains(forbidden),
            "storage usage/accounting export must not expose Phase 4 authority vocabulary: {forbidden}"
        );
    }

    assert!(encoded.contains(r#""metric_kind":"bytes_stored""#));
    assert!(encoded.contains(r#""source_service":"svc-storage""#));
}

#[tokio::test]
async fn bond_artifact_bytes_store_by_b3_without_unlock_slash_or_validator_authority() {
    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.phase4-bond-artifact.v1","kind":"bond_report_copy","note":"opaque bytes only","unlocks_paid_content":false}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.phase4-bond-artifact.v1","kind":"slash_evidence_copy","note":"opaque bytes only","automatic_slash":false}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.phase4-bond-artifact.v1","kind":"unlock_request_copy","note":"opaque bytes only","wallet_mutation":false}"#,
        ),
    ];

    let store = MemoryStorage::new();

    for body in artifacts {
        let cid = cid_for(body.as_ref());
        assert_canonical_b3(&cid);

        store
            .put(&cid, body.clone())
            .await
            .expect("opaque Phase 4 bond artifact bytes should store by b3 cid");

        assert!(
            store
                .exists(&cid)
                .await
                .expect("exists check should succeed"),
            "stored Phase 4 artifact bytes should be discoverable by exact b3 cid"
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
fn storage_source_does_not_implement_phase4_bond_runtime_authority() {
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

    for forbidden in [
        "QuickChainValidatorBondAccount",
        "QuickChainSlashEvidence",
        "bond_lifecycle_decision",
        "bond_account_status",
        "apply_bond(",
        "commit_bond(",
        "execute_bond(",
        "apply_slash(",
        "commit_slash(",
        "execute_slash(",
        "slash_bond(",
        "auto_slash_now",
        "validator_reward_receipt",
        "public_staking_market",
        "liquidity_pool",
        "paid_unlock_from_bond",
        "unlock_from_bond_artifact",
        "cache_only_unlock",
        "bridge_settlement",
        "external_settlement",
        "solana",
        "rox",
        "ron_ledger::",
    ] {
        assert!(
            !source.contains(forbidden),
            "svc-storage source must not implement Phase 4 bond runtime authority via `{forbidden}`"
        );
    }
}
