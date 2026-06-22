//! RO:WHAT — QC-1A pair-level interlock tests for ron-accounting payout/wallet boundaries.
//! RO:WHY — Accounting snapshots may feed reward planning, but must never become wallet payout execution or balance truth.
//! RO:INTERACTS — RewardSnapshotExport, ProjectedRewardSnapshot, UsageEvent ingest DTOs, src tree.
//! RO:INVARIANTS — artifact CIDs are not roots; reward snapshots are planning input only; no wallet/ledger mutation APIs.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects payout/wallet/root/finality poison fields before accounting artifacts can be treated as authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_phase1_pair_interlock.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    http_ingest::{UsageEventsIngestRequest, STORAGE_USAGE_EVENTS_SCHEMA},
    ProjectedRewardSnapshot, RewardContributionExport, RewardProjectionReport,
    RewardSnapshotExport, UsageEvent,
};
use serde_json::{json, Value};

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
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1_777_309_851_000,
        "1000",
        vec![RewardContributionExport::new(
            "acct_qc1a_pair_provider",
            4096,
            2048,
            60,
        )],
    )
    .expect("sample reward snapshot should validate")
}

fn sample_projected_snapshot() -> ProjectedRewardSnapshot {
    let snapshot = sample_snapshot();
    let snapshot_cid = snapshot
        .canonical_cid()
        .expect("sample snapshot CID should compute");

    ProjectedRewardSnapshot {
        snapshot,
        snapshot_cid,
        report: RewardProjectionReport {
            input_slices: 1,
            input_rows: 1,
            projected_accounts: 1,
            bytes_stored: 4096,
            bytes_served: 2048,
            uptime_seconds: 60,
            ignored_rows: 0,
        },
    }
}

fn assert_no_key_recursive(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    !key.contains(forbidden),
                    "accounting planning artifact must not expose authority key fragment `{forbidden}` in key `{key}`"
                );
                assert_no_key_recursive(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key_recursive(nested, forbidden);
            }
        }
        _ => {}
    }
}

#[test]
fn reward_snapshot_rejects_wallet_payout_execution_poison_fields() {
    let clean = serde_json::to_value(sample_snapshot()).expect("snapshot should serialize");

    for field in [
        "wallet_issue_request",
        "wallet_transfer_request",
        "wallet_hold_request",
        "wallet_capture_request",
        "wallet_release_request",
        "wallet_payout_intent",
        "approved_payout",
        "execute_payout",
        "ledger_mutation",
        "ledger_commit",
        "receipt_hash",
        "receipt_root",
        "state_root",
        "accounting_root",
        "reward_root",
        "checkpoint_root",
        "operation_id",
        "account_sequence",
        "settlement_status",
        "finality",
        "validator_signature",
        "anchor_status",
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("snapshot JSON should be an object")
            .insert(field.to_string(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned).is_err(),
            "RewardSnapshotExport must reject payout/wallet/root authority poison field: {field}"
        );
    }
}

#[test]
fn projected_reward_snapshot_rejects_wallet_payout_execution_poison_fields() {
    let clean = serde_json::to_value(sample_projected_snapshot())
        .expect("projected snapshot should serialize");

    for field in [
        "payout_plan",
        "payout_intents",
        "wallet_requests",
        "wallet_receipts",
        "ledger_receipts",
        "ledger_mutations",
        "rewarder_approval",
        "state_root",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "checkpoint_root",
        "settlement_status",
        "external_anchor",
        "validator_set",
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("projected snapshot JSON should be an object")
            .insert(field.to_string(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<ProjectedRewardSnapshot>(poisoned).is_err(),
            "ProjectedRewardSnapshot must reject payout/wallet/root authority poison field: {field}"
        );
    }
}

#[test]
fn serialized_projected_snapshot_contains_planning_artifact_fields_only() {
    let value = serde_json::to_value(sample_projected_snapshot())
        .expect("projected snapshot should serialize");

    for forbidden_fragment in [
        "wallet",
        "ledger",
        "payout",
        "receipt",
        "root",
        "checkpoint",
        "validator",
        "settlement",
        "finality",
        "anchor",
        "bridge",
        "stake",
        "liquidity",
    ] {
        assert_no_key_recursive(&value, forbidden_fragment);
    }

    assert!(
        value
            .get("snapshot_cid")
            .and_then(Value::as_str)
            .is_some_and(|cid| cid.starts_with("b3:") && cid.len() == 67),
        "projected snapshot may expose an artifact CID, not a QuickChain root"
    );
}

#[test]
fn usage_ingest_rejects_pair_interlock_authority_fields() {
    for field in [
        "wallet_issue_request",
        "wallet_transfer_request",
        "wallet_payout_intent",
        "rewarder_decision",
        "approved_payout",
        "execute_payout",
        "ledger_mutation",
        "receipt_hash",
        "settlement_status",
        "finality",
        "accounting_root",
        "reward_root",
        "state_root",
        "checkpoint_root",
    ] {
        let mut top_level = json!({
            "schema": STORAGE_USAGE_EVENTS_SCHEMA,
            "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "wallet_txid": "tx_observation_only_not_receipt_authority",
            "source_service": "svc-storage",
            "events": []
        });
        top_level
            .as_object_mut()
            .expect("ingest request should be an object")
            .insert(field.to_string(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<UsageEventsIngestRequest>(top_level).is_err(),
            "top-level accounting ingest must reject pair interlock authority field: {field}"
        );

        let mut event = json!({
            "timestamp_ms": 1,
            "tenant": 7,
            "subject": "provider_a",
            "metric_kind": "bytes_stored",
            "value": 128
        });
        event
            .as_object_mut()
            .expect("usage event should be an object")
            .insert(field.to_string(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<UsageEvent>(event).is_err(),
            "nested UsageEvent must reject pair interlock authority field: {field}"
        );
    }
}

#[test]
fn source_tree_has_no_wallet_payout_execution_entrypoints() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "svc_wallet::",
            "WalletClient",
            "WalletState",
            "IssueRequest",
            "TransferRequest",
            "BurnRequest",
            "HoldRequest",
            "CaptureRequest",
            "ReleaseRequest",
            "PayoutIntent",
            "WalletPayout",
            "execute_payout",
            "submit_payout",
            "issue_payout_receipt",
            "wallet.issue",
            "wallet.transfer",
            "wallet.capture",
            "ledger.commit",
            "mutate_wallet",
            "mutate_ledger",
            "settlement_status",
            "checkpoint_root",
            "validator_signature",
            "external_anchor",
        ] {
            assert!(
                !code.contains(forbidden),
                "{} must not contain wallet payout or settlement authority fragment: {forbidden}",
                path.display()
            );
        }
    }
}
