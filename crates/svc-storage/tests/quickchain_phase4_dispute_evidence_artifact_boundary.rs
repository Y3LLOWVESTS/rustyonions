#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 2 dispute/evidence artifact boundary tests for svc-storage.
//! RO:WHY — Challenge/freeze/appeal/slash simulation may produce evidence bytes,
//! but storage must treat them as opaque b3-addressed artifacts only, never as
//! paid-unlock, wallet, ledger, bond, slash, staking, liquidity, bridge, or
//! external settlement authority.
//! RO:INTERACTS — MemoryStorage, AccountingExportRequest, UsageEventDto, source boundary.
//! RO:INVARIANTS — b3 proves bytes only; cache/storage cannot unlock paid content;
//! evidence artifacts do not authorize dispute lifecycle decisions.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — rejects Phase 4 Round 2 dispute/evidence authority smuggling.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use serde_json::{json, Value};
use svc_storage::{
    accounting::{exporter::AccountingExportRequest, UsageEventDto},
    storage::{MemoryStorage, Storage},
};

const PHASE4_ROUND2_DISPUTE_AUTHORITY_KEYS: &[&str] = &[
    "dispute_id",
    "dispute_status",
    "challenge_window",
    "challenge_window_open",
    "appeal_window",
    "appeal_window_open",
    "freeze_pending_appeal",
    "frozen_minor",
    "disputed_minor",
    "slash_evidence",
    "slash_decision",
    "slash_recommendation",
    "slash_capture",
    "automatic_slash",
    "auto_slash_now",
    "execute_slash",
    "commit_slash_decision",
    "capture_disputed_bond",
    "bond_forfeiture",
    "bond_penalty",
    "wallet_receipt",
    "ledger_receipt",
    "wallet_mutation",
    "ledger_mutation",
    "paid_unlock",
    "paid_unlock_from_dispute",
    "unlock_from_evidence",
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

fn static_json(value: Value) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
}

fn clean_usage_event_value() -> Value {
    json!({
        "timestamp_ms": 1_777_314_000_000_u64,
        "tenant": 1_u128,
        "subject": "svc_storage",
        "metric_kind": "bytes_stored",
        "value": 128_u64,
        "source_service": "svc-storage",
        "region": "local",
        "route": "/paid/o"
    })
}

fn clean_export_request_value() -> Value {
    json!({
        "schema": "svc-storage.usage-events.v1",
        "cid": format!("b3:{}", "a".repeat(64)),
        "wallet_txid": "tx_storage_paid_write_1",
        "source_service": "svc-storage",
        "events": [clean_usage_event_value()]
    })
}

#[test]
fn storage_accounting_export_rejects_dispute_authority_fields() {
    assert!(
        serde_json::from_str::<AccountingExportRequest>(static_json(clean_export_request_value()))
            .is_ok(),
        "clean accounting export shape should parse"
    );

    for field in PHASE4_ROUND2_DISPUTE_AUTHORITY_KEYS {
        let mut top_level = clean_export_request_value();
        top_level
            .as_object_mut()
            .expect("export request JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("client-supplied-dispute-authority"),
            );

        assert!(
            serde_json::from_str::<AccountingExportRequest>(static_json(top_level)).is_err(),
            "AccountingExportRequest must reject top-level Phase 4 Round 2 dispute authority field: {field}"
        );

        let mut nested_event = clean_usage_event_value();
        nested_event
            .as_object_mut()
            .expect("usage event JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("client-supplied-dispute-authority"),
            );

        let nested_request = json!({
            "schema": "svc-storage.usage-events.v1",
            "cid": format!("b3:{}", "b".repeat(64)),
            "wallet_txid": "tx_storage_paid_write_2",
            "source_service": "svc-storage",
            "events": [nested_event]
        });

        assert!(
            serde_json::from_str::<AccountingExportRequest>(static_json(nested_request)).is_err(),
            "AccountingExportRequest nested UsageEventDto must reject Phase 4 Round 2 dispute authority field: {field}"
        );
    }
}

#[test]
fn storage_usage_event_remains_metering_not_dispute_or_unlock_authority() {
    assert!(
        serde_json::from_str::<UsageEventDto>(static_json(clean_usage_event_value())).is_ok(),
        "clean usage event shape should parse"
    );

    for field in PHASE4_ROUND2_DISPUTE_AUTHORITY_KEYS {
        let mut event = clean_usage_event_value();
        event
            .as_object_mut()
            .expect("usage event JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("event-supplied-dispute-authority"),
            );

        assert!(
            serde_json::from_str::<UsageEventDto>(static_json(event)).is_err(),
            "UsageEventDto must reject Phase 4 Round 2 dispute/unlock authority field: {field}"
        );
    }
}

#[tokio::test]
async fn dispute_evidence_bytes_store_by_b3_without_unlock_or_slash_authority() {
    let body = Bytes::from_static(
        br#"{"schema":"quickchain.dispute-evidence.bytes-only.test","status":"opaque"}"#,
    );
    let cid = format!("b3:{}", blake3::hash(&body).to_hex());

    let store = MemoryStorage::new();

    store
        .put(&cid, body.clone())
        .await
        .expect("opaque dispute evidence artifact write should succeed");

    let full = store
        .get_full(&cid)
        .await
        .expect("opaque dispute evidence artifact read should succeed");
    assert_eq!(full, body);

    let (prefix, total) = store
        .get_range(&cid, 0, 3)
        .await
        .expect("opaque dispute evidence artifact range read should succeed");
    assert_eq!(total, body.len() as u64);
    assert_eq!(&prefix[..], &body[..4]);

    let head = store
        .head(&cid)
        .await
        .expect("opaque dispute evidence artifact head should succeed");

    assert_eq!(head.len, body.len() as u64);
    assert!(
        head.etag
            .contains(&blake3::hash(&body).to_hex().to_string()),
        "etag should remain content-hash-derived, not dispute/unlock authority"
    );
}

#[test]
fn storage_source_does_not_construct_phase4_round2_dispute_runtime_or_unlock_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "dispute_unlock",
            "paid_unlock_from_dispute",
            "unlock_from_dispute",
            "unlock_from_evidence",
            "evidence_paid_unlock",
            "cache_dispute_authority",
            "cache_unlock_authority_from_dispute",
            "slash_evidence_truth",
            "bond_dispute_truth",
            "execute_slash",
            "commit_slash_decision",
            "capture_disputed_bond",
            "bond_forfeiture",
            "wallet_slash",
            "ledger_slash",
            "auto_slash_now",
            "validator_reward_receipt",
            "public_staking_market",
            "liquidity_pool",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not construct Phase 4 Round 2 dispute evidence/unlock/slash authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
