#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 3 selected external-posture artifact boundary tests for svc-storage.
//! RO:WHY — Storage may carry selected external-posture artifact bytes by canonical b3,
//! but those bytes must not become paid-unlock, payment, wallet, ledger, reward,
//! settlement, outside-chain, bridge, market, liquidity, or pruning authority.
//! RO:INTERACTS — MemoryStorage, Storage trait, AccountingExportRequest, UsageEventDto,
//! source boundary, strict evidence-only artifact report shape.
//! RO:INVARIANTS — b3 proves bytes only; artifact references are status/evidence only;
//! cache/storage cannot unlock paid content or mutate ROC.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — prevents selected external posture artifacts from becoming storage,
//! paid access, wallet, ledger, bridge, outside-program, exchange, or market authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase5_external_posture_artifact_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_storage::{
    accounting::{exporter::AccountingExportRequest, UsageEventDto},
    storage::{MemoryStorage, Storage},
};

const REPORT_SCHEMA: &str = "svc-storage.quickchain-external-posture-artifact.v1";
const CHECKPOINT_COMMITMENT: &str =
    "b3:dededededededededededededededededededededededededededededededede";
const ARTIFACT_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageExternalPostureArtifactReport {
    schema: String,
    produced_at_ms: u64,
    chain_id: String,
    artifact_cid: String,
    checkpoint_commitment: String,
    artifact_kind: String,
    chosen_posture: String,
    posture_semantics: String,
    source: String,
    anchor_only_selected: bool,
    artifact_reference_only: bool,
    evidence_only: bool,
    byte_identity_only: bool,
    wallet_ledger_truth_canonical: bool,
    storage_side_effect: bool,
    paid_unlock_authority: bool,
    payment_truth: bool,
    wallet_truth: bool,
    ledger_truth: bool,
    reward_truth: bool,
    settlement_truth: bool,
    outside_da_truth: bool,
    outside_chain_truth: bool,
    outside_program_authority: bool,
    bridge_authority: bool,
    exchange_facing_authority: bool,
    public_market: bool,
    liquidity_enabled: bool,
    pruning_authority: bool,
    bonded_economy_authority: bool,
}

impl StorageExternalPostureArtifactReport {
    fn new() -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            produced_at_ms: 1_777_700_002_000,
            chain_id: "roc-dev".to_string(),
            artifact_cid: ARTIFACT_CID.to_string(),
            checkpoint_commitment: CHECKPOINT_COMMITMENT.to_string(),
            artifact_kind: "selected_external_posture_status_copy".to_string(),
            chosen_posture: "anchor_only".to_string(),
            posture_semantics: "evidence_and_anchoring_only".to_string(),
            source: "svc-storage".to_string(),
            anchor_only_selected: true,
            artifact_reference_only: true,
            evidence_only: true,
            byte_identity_only: true,
            wallet_ledger_truth_canonical: true,
            storage_side_effect: false,
            paid_unlock_authority: false,
            payment_truth: false,
            wallet_truth: false,
            ledger_truth: false,
            reward_truth: false,
            settlement_truth: false,
            outside_da_truth: false,
            outside_chain_truth: false,
            outside_program_authority: false,
            bridge_authority: false,
            exchange_facing_authority: false,
            public_market: false,
            liquidity_enabled: false,
            pruning_authority: false,
            bonded_economy_authority: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPORT_SCHEMA {
            return Err("invalid storage external posture artifact schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("external posture artifact produced_at_ms must be nonzero".to_string());
        }

        if self.chain_id.is_empty() || self.artifact_kind.is_empty() || self.source != "svc-storage"
        {
            return Err("external posture artifact identity fields must be explicit".to_string());
        }

        validate_b3("artifact_cid", &self.artifact_cid)?;
        validate_b3("checkpoint_commitment", &self.checkpoint_commitment)?;

        if self.chosen_posture != "anchor_only" {
            return Err("storage external posture artifact must stay anchor-only".to_string());
        }

        if self.posture_semantics != "evidence_and_anchoring_only" {
            return Err(
                "storage external posture artifact must stay evidence-and-anchoring-only"
                    .to_string(),
            );
        }

        for (field, value) in [
            ("anchor_only_selected", self.anchor_only_selected),
            ("artifact_reference_only", self.artifact_reference_only),
            ("evidence_only", self.evidence_only),
            ("byte_identity_only", self.byte_identity_only),
            (
                "wallet_ledger_truth_canonical",
                self.wallet_ledger_truth_canonical,
            ),
        ] {
            if !value {
                return Err(format!(
                    "storage external posture artifact requires true field: {field}"
                ));
            }
        }

        for (field, value) in [
            ("storage_side_effect", self.storage_side_effect),
            ("paid_unlock_authority", self.paid_unlock_authority),
            ("payment_truth", self.payment_truth),
            ("wallet_truth", self.wallet_truth),
            ("ledger_truth", self.ledger_truth),
            ("reward_truth", self.reward_truth),
            ("settlement_truth", self.settlement_truth),
            ("outside_da_truth", self.outside_da_truth),
            ("outside_chain_truth", self.outside_chain_truth),
            ("outside_program_authority", self.outside_program_authority),
            ("bridge_authority", self.bridge_authority),
            ("exchange_facing_authority", self.exchange_facing_authority),
            ("public_market", self.public_market),
            ("liquidity_enabled", self.liquidity_enabled),
            ("pruning_authority", self.pruning_authority),
            ("bonded_economy_authority", self.bonded_economy_authority),
        ] {
            if value {
                return Err(format!(
                    "storage external posture artifact must not claim authority field: {field}"
                ));
            }
        }

        Ok(())
    }
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

fn cid_for(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn assert_canonical_b3(cid: &str) {
    let hex = cid.strip_prefix("b3:").expect("cid should use b3 prefix");
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')));
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

fn clean_accounting_export_json() -> Value {
    json!({
        "schema": "svc-storage.usage-events.v1",
        "cid": ARTIFACT_CID,
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
    })
}

fn decode_accounting_export(value: Value) -> Result<AccountingExportRequest, serde_json::Error> {
    let raw = serde_json::to_string(&value)?;
    let leaked: &'static str = Box::leak(raw.into_boxed_str());
    serde_json::from_str::<AccountingExportRequest>(leaked)
}

#[test]
fn storage_external_posture_artifact_report_is_anchor_only_byte_reference_only() {
    let report = StorageExternalPostureArtifactReport::new();

    report
        .validate()
        .expect("clean storage external posture artifact report should validate");

    assert_eq!(report.schema, REPORT_SCHEMA);
    assert_eq!(report.source, "svc-storage");
    assert_eq!(report.chosen_posture, "anchor_only");
    assert_eq!(report.posture_semantics, "evidence_and_anchoring_only");

    assert!(report.anchor_only_selected);
    assert!(report.artifact_reference_only);
    assert!(report.evidence_only);
    assert!(report.byte_identity_only);
    assert!(report.wallet_ledger_truth_canonical);

    assert!(!report.storage_side_effect);
    assert!(!report.paid_unlock_authority);
    assert!(!report.payment_truth);
    assert!(!report.wallet_truth);
    assert!(!report.ledger_truth);
    assert!(!report.reward_truth);
    assert!(!report.settlement_truth);
    assert!(!report.outside_da_truth);
    assert!(!report.outside_chain_truth);
    assert!(!report.outside_program_authority);
    assert!(!report.bridge_authority);
    assert!(!report.exchange_facing_authority);
    assert!(!report.public_market);
    assert!(!report.liquidity_enabled);
    assert!(!report.pruning_authority);
    assert!(!report.bonded_economy_authority);
}

#[test]
fn storage_external_posture_artifact_report_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(StorageExternalPostureArtifactReport::new())
        .expect("report should serialize");

    for field in [
        "anchor_only_selected",
        "artifact_reference_only",
        "evidence_only",
        "byte_identity_only",
        "wallet_ledger_truth_canonical",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(false);

        let decoded = serde_json::from_value::<StorageExternalPostureArtifactReport>(poisoned)
            .expect("known false required flag should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "storage artifact report must reject false required field {field}"
        );
    }

    for field in [
        "storage_side_effect",
        "paid_unlock_authority",
        "payment_truth",
        "wallet_truth",
        "ledger_truth",
        "reward_truth",
        "settlement_truth",
        "outside_da_truth",
        "outside_chain_truth",
        "outside_program_authority",
        "bridge_authority",
        "exchange_facing_authority",
        "public_market",
        "liquidity_enabled",
        "pruning_authority",
        "bonded_economy_authority",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(true);

        let decoded = serde_json::from_value::<StorageExternalPostureArtifactReport>(poisoned)
            .expect("known authority field should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "storage artifact report must reject authority field {field}"
        );
    }

    let mut unknown = clean;
    unknown["external_settlement_authorized"] = json!(true);

    assert!(
        serde_json::from_value::<StorageExternalPostureArtifactReport>(unknown).is_err(),
        "unknown external posture authority fields must reject"
    );
}

#[tokio::test]
async fn external_posture_artifact_bytes_store_by_b3_without_unlock_or_payment_authority() {
    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.external-posture-decision.v1","chosen_posture":"anchor_only","semantics":"evidence_only","paid_unlock_authority":false}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.external-posture-verification.v1","verified_evidence_only":true,"balance_mutation_detected":false}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.external-posture-status-copy.v1","wallet_ledger_truth_canonical":true,"storage_authority":false}"#,
        ),
    ];

    let store = MemoryStorage::new();

    for body in artifacts {
        let cid = cid_for(body.as_ref());
        assert_canonical_b3(&cid);

        store
            .put(&cid, body.clone())
            .await
            .expect("external posture artifact bytes should store by b3 cid");

        assert!(
            store
                .exists(&cid)
                .await
                .expect("exists check should succeed"),
            "stored external posture artifact should be discoverable by exact b3 cid"
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
            .expect("full external posture artifact read should succeed");
        assert_eq!(full, body);

        let (prefix, total) = store
            .get_range(&cid, 0, 3)
            .await
            .expect("range external posture artifact read should succeed");
        assert_eq!(total, body.len() as u64);
        assert_eq!(&prefix[..], &body[..4]);
    }
}

#[test]
fn storage_accounting_export_rejects_external_posture_authority_fields() {
    decode_accounting_export(clean_accounting_export_json())
        .expect("clean accounting export request should deserialize");

    for field in [
        "external_posture",
        "chosen_external_posture",
        "paid_unlock_authority",
        "payment_truth",
        "wallet_truth",
        "ledger_truth",
        "reward_truth",
        "settlement_truth",
        "outside_da_truth",
        "outside_chain_truth",
        "outside_program_authority",
        "bridge_authority",
        "exchange_facing_authority",
        "public_market",
        "liquidity_enabled",
        "pruning_authority",
    ] {
        let mut top_level = clean_accounting_export_json();
        top_level[field] = json!(true);

        assert!(
            decode_accounting_export(top_level).is_err(),
            "AccountingExportRequest must reject top-level external posture authority field: {field}"
        );

        let mut nested = clean_accounting_export_json();
        nested["events"][0][field] = json!(true);

        assert!(
            decode_accounting_export(nested).is_err(),
            "UsageEventDto must reject nested external posture authority field: {field}"
        );
    }
}

#[test]
fn storage_usage_event_export_remains_metering_not_external_posture_authority() {
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

    let request = AccountingExportRequest::new(ARTIFACT_CID, "wallet_txid_display_only", &[event]);

    let encoded = serde_json::to_string(&request).expect("accounting export should serialize");

    assert!(encoded.contains(r#""metric_kind":"bytes_stored""#));
    assert!(encoded.contains(r#""source_service":"svc-storage""#));

    for forbidden in [
        "external_posture",
        "chosen_external_posture",
        "paid_unlock_authority",
        "payment_truth",
        "wallet_truth",
        "ledger_truth",
        "reward_truth",
        "settlement_truth",
        "outside_chain_truth",
        "bridge_authority",
        "outside_program_authority",
        "exchange_facing_authority",
        "public_market",
        "liquidity_enabled",
        "pruning_authority",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "storage usage/accounting export must not expose external posture authority: {forbidden}"
        );
    }
}

#[test]
fn storage_source_does_not_construct_external_posture_runtime_or_unlock_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "external_posture_paid_unlock",
            "paid_unlock_from_external_posture",
            "unlock_from_external_posture",
            "unlock_from_posture_artifact",
            "external_posture_wallet_receipt",
            "external_posture_payment_truth",
            "external_posture_balance_truth",
            "external_posture_settlement_truth",
            "cache_external_posture_authority",
            "cache_unlock_authority_from_posture",
            "storage_unlock_authority_from_posture",
            "outside_da_truth: true",
            "outside_chain_truth: true",
            "outside_program_authority: true",
            "bridge_authority: true",
            "exchange_facing_authority: true",
            "public_market: true",
            "liquidity_enabled: true",
            "pruning_authority: true",
            "bonded_economy_authority: true",
            "solana_runtime",
            "rox_runtime",
            "bridge_settlement",
            "exchange_facing",
            "liquidity_pool",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not implement Phase 5 Round 3 external posture runtime/unlock authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
