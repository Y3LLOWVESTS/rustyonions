#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback artifact boundary tests for svc-storage.
//! RO:WHY — DA/archive/challenge artifacts may be stored and restored by b3, but storage must not become payment, reward, paid-unlock, pruning, settlement, wallet, ledger, or outside-truth authority.
//! RO:INTERACTS — MemoryStorage, Storage trait, strict artifact report shape, source boundary.
//! RO:INVARIANTS — b3 proves bytes only; archive restore is evidence-only; challenge evidence blocks pruning but cannot authorize it.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — prevents stored DA fallback artifacts from becoming economic authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase5_da_fallback_artifact_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_storage::storage::{MemoryStorage, Storage};

const REPORT_SCHEMA: &str = "svc-storage.quickchain-da-fallback-artifact.v1";
const CHECKPOINT_HASH: &str = "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DATA_AVAILABILITY_ROOT: &str =
    "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageDaFallbackArtifactReport {
    schema: String,
    produced_at_ms: u64,
    chain_id: String,
    fallback_plan_id: String,
    checkpoint_hash: String,
    data_availability_root: String,
    challenged_chunk_id: Option<String>,
    artifact_kind: String,
    retention_class: String,
    artifact_cid: String,
    artifact_len: u64,
    byte_identity_only: bool,
    report_only: bool,
    evidence_only: bool,
    archive_fallback_checked: bool,
    missing_data_challenge_checked: bool,
    restore_path_checked: bool,
    pruning_blocked: bool,
    storage_side_effect: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    balance_truth: bool,
    payment_truth: bool,
    reward_truth: bool,
    paid_unlock_authority: bool,
    pruning_authority: bool,
    settlement_truth: bool,
    outside_data_availability_truth: bool,
    outside_chain_truth: bool,
}

impl StorageDaFallbackArtifactReport {
    fn new(artifact_cid: impl Into<String>, artifact_len: u64) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            produced_at_ms: 1_777_700_002_000,
            chain_id: "roc-dev".to_string(),
            fallback_plan_id: "fallback-plan:phase5:r2:storage".to_string(),
            checkpoint_hash: CHECKPOINT_HASH.to_string(),
            data_availability_root: DATA_AVAILABILITY_ROOT.to_string(),
            challenged_chunk_id: Some("chunk:receipt-batch:0001".to_string()),
            artifact_kind: "receipt-batch-chunk".to_string(),
            retention_class: "archive-first-no-pruning".to_string(),
            artifact_cid: artifact_cid.into(),
            artifact_len,
            byte_identity_only: true,
            report_only: true,
            evidence_only: true,
            archive_fallback_checked: true,
            missing_data_challenge_checked: true,
            restore_path_checked: true,
            pruning_blocked: true,
            storage_side_effect: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            balance_truth: false,
            payment_truth: false,
            reward_truth: false,
            paid_unlock_authority: false,
            pruning_authority: false,
            settlement_truth: false,
            outside_data_availability_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPORT_SCHEMA {
            return Err("invalid storage DA fallback artifact schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        if self.artifact_len == 0 {
            return Err("artifact_len must be nonzero".to_string());
        }

        validate_token("chain_id", &self.chain_id)?;
        validate_token("fallback_plan_id", &self.fallback_plan_id)?;
        validate_token("artifact_kind", &self.artifact_kind)?;
        validate_token("retention_class", &self.retention_class)?;
        if let Some(challenged_chunk_id) = self.challenged_chunk_id.as_deref() {
            validate_token("challenged_chunk_id", challenged_chunk_id)?;
        }

        validate_b3("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3("data_availability_root", &self.data_availability_root)?;
        validate_b3("artifact_cid", &self.artifact_cid)?;

        if !self.byte_identity_only {
            return Err("storage DA fallback artifact must remain byte-identity-only".to_string());
        }
        if !self.report_only {
            return Err("storage DA fallback artifact must remain report-only".to_string());
        }
        if !self.evidence_only {
            return Err("storage DA fallback artifact must remain evidence-only".to_string());
        }
        if !self.archive_fallback_checked {
            return Err("storage DA fallback artifact must check archive fallback".to_string());
        }
        if !self.missing_data_challenge_checked {
            return Err(
                "storage DA fallback artifact must check missing-data challenge handling"
                    .to_string(),
            );
        }
        if !self.restore_path_checked {
            return Err("storage DA fallback artifact must check restore path".to_string());
        }
        if !self.pruning_blocked {
            return Err("storage DA fallback artifact must keep pruning blocked".to_string());
        }

        if self.storage_side_effect {
            return Err("storage DA fallback report must not mutate storage policy".to_string());
        }
        if self.wallet_side_effect {
            return Err("storage DA fallback report must not mutate wallet state".to_string());
        }
        if self.ledger_side_effect {
            return Err("storage DA fallback report must not mutate ledger state".to_string());
        }
        if self.balance_truth {
            return Err("storage DA fallback artifact must not become balance truth".to_string());
        }
        if self.payment_truth {
            return Err("storage DA fallback artifact must not become payment truth".to_string());
        }
        if self.reward_truth {
            return Err("storage DA fallback artifact must not become reward truth".to_string());
        }
        if self.paid_unlock_authority {
            return Err("storage DA fallback artifact must not unlock paid content".to_string());
        }
        if self.pruning_authority {
            return Err("storage DA fallback artifact must not authorize pruning".to_string());
        }
        if self.settlement_truth {
            return Err(
                "storage DA fallback artifact must not become settlement truth".to_string(),
            );
        }
        if self.outside_data_availability_truth {
            return Err("outside DA material must not become storage truth".to_string());
        }
        if self.outside_chain_truth {
            return Err("outside chain material must not become ROC truth".to_string());
        }

        Ok(())
    }
}

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

fn validate_token(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 160 {
        return Err(format!("{field} must be 1..=160 bytes"));
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/' | '@'))
    {
        return Err(format!("{field} contains unsupported characters"));
    }

    Ok(())
}

fn validate_b3(field: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(format!("{field} must be b3:<64 lowercase hex>"));
    };

    if hex.len() != 64 || !hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(format!("{field} must be b3:<64 lowercase hex>"));
    }

    Ok(())
}

fn assert_no_key(value: &Value, key: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(key),
                "serialized storage DA fallback artifact report must not expose authority key `{key}`: {value}"
            );
            for child in map.values() {
                assert_no_key(child, key);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_no_key(child, key);
            }
        }
        _ => {}
    }
}

fn artifact_body() -> Bytes {
    Bytes::from_static(
        br#"{"schema":"quickchain.da-artifact.fixture.v1","kind":"receipt-batch-chunk","bytes":"opaque"}"#,
    )
}

fn artifact_cid(body: &Bytes) -> String {
    format!("b3:{}", blake3::hash(body).to_hex())
}

fn report_for(body: &Bytes) -> StorageDaFallbackArtifactReport {
    let report = StorageDaFallbackArtifactReport::new(artifact_cid(body), body.len() as u64);
    report
        .validate()
        .expect("storage DA fallback artifact report should validate");
    report
}

#[tokio::test]
async fn storage_da_fallback_artifact_is_restored_by_b3_but_remains_non_authority() {
    let body = artifact_body();
    let cid = artifact_cid(&body);
    let store = MemoryStorage::new();

    store
        .put(&cid, body.clone())
        .await
        .expect("opaque DA fallback artifact write should succeed");

    let full = store
        .get_full(&cid)
        .await
        .expect("opaque DA fallback artifact read should succeed");
    assert_eq!(full, body);

    let (prefix, total) = store
        .get_range(&cid, 0, 15)
        .await
        .expect("opaque DA fallback artifact range read should succeed");
    assert_eq!(total, body.len() as u64);
    assert_eq!(&prefix[..], &body[..16]);

    let head = store
        .head(&cid)
        .await
        .expect("opaque DA fallback artifact head should succeed");
    assert_eq!(head.len, body.len() as u64);
    assert!(
        head.etag
            .contains(&blake3::hash(&body).to_hex().to_string()),
        "etag should remain content-hash-derived, not DA fallback authority"
    );

    let report = report_for(&body);

    assert_eq!(report.schema, REPORT_SCHEMA);
    assert_eq!(report.artifact_cid, cid);
    assert!(report.byte_identity_only);
    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(report.archive_fallback_checked);
    assert!(report.missing_data_challenge_checked);
    assert!(report.restore_path_checked);
    assert!(report.pruning_blocked);

    assert!(!report.storage_side_effect);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.balance_truth);
    assert!(!report.payment_truth);
    assert!(!report.reward_truth);
    assert!(!report.paid_unlock_authority);
    assert!(!report.pruning_authority);
    assert!(!report.settlement_truth);
    assert!(!report.outside_data_availability_truth);
    assert!(!report.outside_chain_truth);

    let encoded = serde_json::to_value(&report).expect("report should serialize");
    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "payment_receipt",
        "paid_unlock",
        "unlock_token",
        "pruning_allowed",
        "external_settlement",
        "bridge_settlement",
        "solana_runtime",
        "rox_runtime",
    ] {
        assert_no_key(&encoded, forbidden);
    }
}

#[test]
fn storage_da_fallback_artifact_report_rejects_authority_flags_and_unknown_fields() {
    let body = artifact_body();
    let clean = serde_json::to_value(report_for(&body)).expect("report should serialize");

    for field in [
        "byte_identity_only",
        "report_only",
        "evidence_only",
        "archive_fallback_checked",
        "missing_data_challenge_checked",
        "restore_path_checked",
        "pruning_blocked",
    ] {
        let mut poisoned = serde_json::from_value::<StorageDaFallbackArtifactReport>(clean.clone())
            .expect("clean report should deserialize");

        match field {
            "byte_identity_only" => poisoned.byte_identity_only = false,
            "report_only" => poisoned.report_only = false,
            "evidence_only" => poisoned.evidence_only = false,
            "archive_fallback_checked" => poisoned.archive_fallback_checked = false,
            "missing_data_challenge_checked" => poisoned.missing_data_challenge_checked = false,
            "restore_path_checked" => poisoned.restore_path_checked = false,
            "pruning_blocked" => poisoned.pruning_blocked = false,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "required DA fallback storage check/blocker flag must reject when disabled: {field}"
        );
    }

    for field in [
        "storage_side_effect",
        "wallet_side_effect",
        "ledger_side_effect",
        "balance_truth",
        "payment_truth",
        "reward_truth",
        "paid_unlock_authority",
        "pruning_authority",
        "settlement_truth",
        "outside_data_availability_truth",
        "outside_chain_truth",
    ] {
        let mut poisoned = serde_json::from_value::<StorageDaFallbackArtifactReport>(clean.clone())
            .expect("clean report should deserialize");

        match field {
            "storage_side_effect" => poisoned.storage_side_effect = true,
            "wallet_side_effect" => poisoned.wallet_side_effect = true,
            "ledger_side_effect" => poisoned.ledger_side_effect = true,
            "balance_truth" => poisoned.balance_truth = true,
            "payment_truth" => poisoned.payment_truth = true,
            "reward_truth" => poisoned.reward_truth = true,
            "paid_unlock_authority" => poisoned.paid_unlock_authority = true,
            "pruning_authority" => poisoned.pruning_authority = true,
            "settlement_truth" => poisoned.settlement_truth = true,
            "outside_data_availability_truth" => poisoned.outside_data_availability_truth = true,
            "outside_chain_truth" => poisoned.outside_chain_truth = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "DA fallback storage authority flag must reject when enabled: {field}"
        );
    }

    for field in [
        "wallet_receipt",
        "ledger_receipt",
        "payment_receipt",
        "unlock_token",
        "paid_unlock",
        "balance_minor",
        "reward_payout",
        "pruning_allowed",
        "external_settlement",
        "bridge_settlement",
        "solana_runtime",
        "rox_runtime",
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(field.to_string(), json!("smuggled-storage-authority"));

        assert!(
            serde_json::from_value::<StorageDaFallbackArtifactReport>(poisoned).is_err(),
            "DA fallback artifact report must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn storage_da_fallback_artifact_report_rejects_bad_hashes_and_empty_artifacts() {
    let body = artifact_body();

    let mut report = report_for(&body);
    report.checkpoint_hash = "not-a-b3-hash".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );

    let mut report = report_for(&body);
    report.data_availability_root = "b3:ABC".to_string();
    assert!(
        report.validate().is_err(),
        "DA root must be canonical lowercase b3"
    );

    let mut report = report_for(&body);
    report.artifact_cid = format!("b3:{}", "g".repeat(64));
    assert!(
        report.validate().is_err(),
        "artifact cid must be canonical lowercase b3"
    );

    let mut report = report_for(&body);
    report.challenged_chunk_id = Some("bad chunk id with spaces".to_string());
    assert!(
        report.validate().is_err(),
        "challenged chunk id must be bounded visible token"
    );

    let mut report = report_for(&body);
    report.artifact_len = 0;
    assert!(
        report.validate().is_err(),
        "empty artifacts must not satisfy restore evidence"
    );
}

#[test]
fn storage_source_does_not_construct_da_fallback_paid_unlock_or_pruning_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "unlock_from_da_fallback",
            "paid_unlock_from_da_fallback",
            "unlock_from_archive_restore",
            "unlock_from_missing_data_challenge",
            "da_fallback_unlock_authority",
            "archive_restore_payment_truth",
            "missing_data_payment_truth",
            "artifact_payment_truth",
            "artifact_balance_truth",
            "artifact_reward_truth",
            "prune_from_da_fallback",
            "allow_pruning_from_da",
            "pruning_authority: true",
            "outside_data_availability_truth: true",
            "outside_chain_truth: true",
            "wallet_mutation_from_archive",
            "ledger_mutation_from_archive",
            "bridge_settlement",
            "external_settlement",
            "solana_runtime",
            "rox_runtime",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not construct DA fallback artifact/unlock/pruning authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
