#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 1 anchor artifact boundary tests for svc-storage.
//! RO:WHY — Anchor dry-run evidence bytes may be stored by b3,
//! but storage must not become settlement, wallet, ledger, paid-unlock, or outside-chain truth.
//! RO:INTERACTS — MemoryStorage, Storage trait, docs, source boundary.
//! RO:INVARIANTS — b3 proves bytes only; anchor artifacts are opaque evidence;
//! cache/storage cannot unlock paid content or mutate ROC.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — prevents anchor artifact storage from becoming economic authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase5_anchor_artifact_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_storage::storage::{MemoryStorage, Storage};

const REPORT_SCHEMA: &str = "svc-storage.quickchain-anchor-artifact.v1";
const CHECKPOINT_COMMITMENT: &str =
    "b3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageAnchorArtifactReport {
    schema: String,
    produced_at_ms: u64,
    chain_id: String,
    anchor_id: String,
    checkpoint_commitment: String,
    artifact_cid: String,
    artifact_len: u64,
    byte_identity_only: bool,
    report_only: bool,
    evidence_only: bool,
    storage_side_effect: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    balance_truth: bool,
    payment_truth: bool,
    paid_unlock_authority: bool,
    settlement_truth: bool,
    reward_truth: bool,
    outside_chain_truth: bool,
}

impl StorageAnchorArtifactReport {
    fn new(artifact_cid: impl Into<String>, artifact_len: u64) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            produced_at_ms: 1_777_600_002_000,
            chain_id: "roc-dev".to_string(),
            anchor_id: "anchor-dry-run:phase5:r1:storage".to_string(),
            checkpoint_commitment: CHECKPOINT_COMMITMENT.to_string(),
            artifact_cid: artifact_cid.into(),
            artifact_len,
            byte_identity_only: true,
            report_only: true,
            evidence_only: true,
            storage_side_effect: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            balance_truth: false,
            payment_truth: false,
            paid_unlock_authority: false,
            settlement_truth: false,
            reward_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPORT_SCHEMA {
            return Err("invalid storage anchor artifact schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        if self.artifact_len == 0 {
            return Err("artifact_len must be nonzero".to_string());
        }

        for (name, value) in [
            ("chain_id", self.chain_id.as_str()),
            ("anchor_id", self.anchor_id.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("checkpoint_commitment", &self.checkpoint_commitment)?;
        validate_b3("artifact_cid", &self.artifact_cid)?;

        if !self.byte_identity_only {
            return Err("storage anchor artifact must remain byte-identity only".to_string());
        }

        if !self.report_only {
            return Err("storage anchor artifact must remain report-only".to_string());
        }

        if !self.evidence_only {
            return Err("storage anchor artifact must remain evidence-only".to_string());
        }

        if self.storage_side_effect {
            return Err(
                "storage anchor artifact report must not imply storage mutation authority"
                    .to_string(),
            );
        }

        if self.wallet_side_effect {
            return Err("storage anchor artifact must not mutate wallet state".to_string());
        }

        if self.ledger_side_effect {
            return Err("storage anchor artifact must not mutate ledger state".to_string());
        }

        if self.balance_truth {
            return Err("storage anchor artifact must not become balance truth".to_string());
        }

        if self.payment_truth {
            return Err("storage anchor artifact must not become payment truth".to_string());
        }

        if self.paid_unlock_authority {
            return Err("storage anchor artifact must not unlock paid content".to_string());
        }

        if self.settlement_truth {
            return Err("storage anchor artifact must not become settlement truth".to_string());
        }

        if self.reward_truth {
            return Err("storage anchor artifact must not become reward truth".to_string());
        }

        if self.outside_chain_truth {
            return Err(
                "storage anchor artifact must not make any outside chain ROC truth".to_string(),
            );
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

fn validate_visible_token(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 160 {
        return Err(format!("{name} must be 1..=160 bytes"));
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/' | '@'))
    {
        return Err(format!("{name} contains unsupported characters"));
    }

    Ok(())
}

fn validate_b3(name: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(format!("{name} must be b3:<64 lowercase hex>"));
    };

    if hex.len() != 64 || !hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(format!("{name} must be b3:<64 lowercase hex>"));
    }

    Ok(())
}

fn artifact_bytes() -> Bytes {
    Bytes::from_static(
        br#"{"schema":"quickchain.anchor-dry-run-artifact.v1","note":"opaque evidence bytes only"}"#,
    )
}

fn artifact_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "storage anchor artifact report must not expose forbidden authority key `{forbidden}`"
                );
                assert_no_key(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key(nested, forbidden);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn storage_can_retain_anchor_dry_run_artifact_bytes_by_b3_only() {
    let bytes = artifact_bytes();
    let cid = artifact_cid(&bytes);

    let store = MemoryStorage::default();
    store
        .put(&cid, bytes.clone())
        .await
        .expect("anchor dry-run artifact bytes should store by b3");

    let head = store
        .head(&cid)
        .await
        .expect("anchor dry-run artifact head should load");
    assert_eq!(head.len, bytes.len() as u64);

    let full = store
        .get_full(&cid)
        .await
        .expect("anchor dry-run artifact bytes should load");
    assert_eq!(full, bytes);

    let (range, total) = store
        .get_range(&cid, 0, 7)
        .await
        .expect("anchor dry-run artifact range should load");
    assert_eq!(total, bytes.len() as u64);
    assert_eq!(&range[..], &bytes[..8]);

    let report = StorageAnchorArtifactReport::new(cid, bytes.len() as u64);
    report
        .validate()
        .expect("storage anchor artifact report should validate");

    assert!(report.byte_identity_only);
    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(!report.storage_side_effect);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.balance_truth);
    assert!(!report.payment_truth);
    assert!(!report.paid_unlock_authority);
    assert!(!report.settlement_truth);
    assert!(!report.reward_truth);
    assert!(!report.outside_chain_truth);
}

#[test]
fn storage_anchor_artifact_report_rejects_authority_flags_unknown_fields_and_bad_b3() {
    let bytes = artifact_bytes();
    let cid = artifact_cid(&bytes);
    let clean_report = StorageAnchorArtifactReport::new(cid, bytes.len() as u64);

    for field in [
        "storage_side_effect",
        "wallet_side_effect",
        "ledger_side_effect",
        "balance_truth",
        "payment_truth",
        "paid_unlock_authority",
        "settlement_truth",
        "reward_truth",
        "outside_chain_truth",
    ] {
        let mut report = clean_report.clone();

        match field {
            "storage_side_effect" => report.storage_side_effect = true,
            "wallet_side_effect" => report.wallet_side_effect = true,
            "ledger_side_effect" => report.ledger_side_effect = true,
            "balance_truth" => report.balance_truth = true,
            "payment_truth" => report.payment_truth = true,
            "paid_unlock_authority" => report.paid_unlock_authority = true,
            "settlement_truth" => report.settlement_truth = true,
            "reward_truth" => report.reward_truth = true,
            "outside_chain_truth" => report.outside_chain_truth = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            report.validate().is_err(),
            "authority flag must reject when enabled: {field}"
        );
    }

    let clean = serde_json::to_value(clean_report).expect("report should serialize");
    for field in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payment_receipt",
        "paid_unlock",
        "cache_unlock",
        "settlement_status",
        "finalized",
        "reward_payout",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(field.to_string(), json!("client-supplied-anchor-authority"));

        assert!(
            serde_json::from_value::<StorageAnchorArtifactReport>(poisoned).is_err(),
            "report DTO must reject unknown authority field: {field}"
        );
    }

    let mut report = StorageAnchorArtifactReport::new(artifact_cid(&bytes), bytes.len() as u64);
    report.artifact_cid = "not-a-b3-hash".to_string();
    assert!(report.validate().is_err(), "artifact cid must be b3");

    let mut report = StorageAnchorArtifactReport::new(artifact_cid(&bytes), bytes.len() as u64);
    report.checkpoint_commitment = format!("b3:{}", "A".repeat(64));
    assert!(
        report.validate().is_err(),
        "checkpoint commitment must be lowercase canonical b3"
    );

    let report = StorageAnchorArtifactReport::new(artifact_cid(&bytes), 0);
    assert!(
        report.validate().is_err(),
        "artifact length must be nonzero"
    );
}

#[test]
fn storage_anchor_artifact_report_exposes_no_payment_or_unlock_authority_keys() {
    let bytes = artifact_bytes();
    let report = StorageAnchorArtifactReport::new(artifact_cid(&bytes), bytes.len() as u64);
    let value = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "payment_receipt",
        "wallet_balance",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "paid_unlock",
        "cache_unlock",
        "cache_only_unlock",
        "settlement_status",
        "finalized",
        "reward_payout",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn storage_phase5_docs_record_anchor_artifact_non_authority_boundary() {
    let docs = read(crate_dir().join("docs/quickchain-preflight.md"));
    let normalized = docs.to_ascii_lowercase();

    for required in [
        "phase 5 round 1 anchor-only dry-run artifact boundary",
        "opaque anchor dry-run evidence bytes may be stored and retrieved by canonical b3",
        "the b3 proves byte identity only",
        "it is not payment proof",
        "it is not paid-unlock authority",
        "it is not wallet truth",
        "it is not ledger truth",
        "it is not balance truth",
        "it is not reward truth",
        "it is not settlement truth",
        "it is not external-chain roc truth",
        "svc-storage remains bytes by b3",
        "cache remains convenience only",
        "paid access remains backend wallet/gateway/omnigate derived",
    ] {
        assert!(
            normalized.contains(required),
            "svc-storage docs missing Phase 5 Round 1 boundary marker: {required}"
        );
    }
}

#[test]
fn storage_runtime_source_does_not_gain_phase5_anchor_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "anchor_paid_unlock",
            "paid_unlock_from_anchor",
            "cache_unlock_from_anchor",
            "settle_from_anchor",
            "commit_from_anchor",
            "apply_anchor",
            "anchor_wallet_receipt",
            "anchor_ledger_receipt",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "balance_truth: true",
            "payment_truth: true",
            "paid_unlock_authority: true",
            "settlement_truth: true",
            "reward_truth: true",
            "outside_chain_truth: true",
            "bridge_settlement",
            "external_settlement",
            "solana_runtime",
            "solana_settlement",
            "solana_anchor_authority",
            "rox_runtime",
            "rox_settlement",
            "liquidity_pool",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not implement Phase 5 anchor runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
