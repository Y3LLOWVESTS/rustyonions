#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 1 anchor evidence boundary tests for svc-rewarder.
//! RO:WHY — Reward manifest commitments may be referenced by anchor dry-run evidence,
//! but svc-rewarder must remain payout planning only.
//! RO:INTERACTS — RewardManifest commitment, compute_manifest, source boundary, docs.
//! RO:INVARIANTS — evidence-only; no payout execution; no wallet/ledger mutation;
//! no reward truth; no paid unlock; no outside-chain truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents anchor evidence from becoming reward, payout, wallet, ledger, bridge, or outside-chain authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase5_anchor_evidence_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    inputs::{
        AccountContribution, AccountingSnapshot, ContentCid, RewardFundingSource, RewardPolicy,
    },
    outputs::{IntentResult, RewardManifest},
};

const REPORT_SCHEMA: &str = "svc-rewarder.quickchain-anchor-evidence.v1";
const CHECKPOINT_COMMITMENT: &str =
    "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RewarderAnchorEvidenceReport {
    schema: String,
    produced_at_ms: u64,
    chain_id: String,
    epoch_id: String,
    source: String,
    anchor_id: String,
    checkpoint_commitment: String,
    reward_manifest_commitment: String,
    reward_run_key: String,
    reward_inputs_cid: String,
    payout_count: usize,
    report_only: bool,
    evidence_only: bool,
    rewarder_side_effect: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    payout_side_effect: bool,
    reward_truth: bool,
    paid_unlock_authority: bool,
    settlement_truth: bool,
    outside_chain_truth: bool,
}

impl RewarderAnchorEvidenceReport {
    fn for_manifest(manifest: &RewardManifest) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            produced_at_ms: 1_777_600_001_000,
            chain_id: "roc-dev".to_string(),
            epoch_id: manifest.epoch_id.clone(),
            source: "svc-rewarder".to_string(),
            anchor_id: "anchor-dry-run:phase5:r1:rewarder".to_string(),
            checkpoint_commitment: CHECKPOINT_COMMITMENT.to_string(),
            reward_manifest_commitment: manifest.commitment.clone(),
            reward_run_key: manifest.run_key.clone(),
            reward_inputs_cid: manifest.inputs_cid.clone(),
            payout_count: manifest.payouts.len(),
            report_only: true,
            evidence_only: true,
            rewarder_side_effect: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            reward_truth: false,
            paid_unlock_authority: false,
            settlement_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPORT_SCHEMA {
            return Err("invalid rewarder anchor evidence schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        for (name, value) in [
            ("chain_id", self.chain_id.as_str()),
            ("epoch_id", self.epoch_id.as_str()),
            ("source", self.source.as_str()),
            ("anchor_id", self.anchor_id.as_str()),
            ("reward_run_key", self.reward_run_key.as_str()),
        ] {
            validate_visible_token(name, value)?;
        }

        validate_b3("checkpoint_commitment", &self.checkpoint_commitment)?;
        validate_b3(
            "reward_manifest_commitment",
            &self.reward_manifest_commitment,
        )?;
        validate_b3("reward_inputs_cid", &self.reward_inputs_cid)?;

        if !self.report_only {
            return Err("rewarder anchor evidence must remain report-only".to_string());
        }

        if !self.evidence_only {
            return Err("rewarder anchor evidence must remain evidence-only".to_string());
        }

        if self.rewarder_side_effect {
            return Err("rewarder anchor evidence must not mutate rewarder state".to_string());
        }

        if self.wallet_side_effect {
            return Err("rewarder anchor evidence must not mutate wallet state".to_string());
        }

        if self.ledger_side_effect {
            return Err("rewarder anchor evidence must not mutate ledger state".to_string());
        }

        if self.payout_side_effect {
            return Err("rewarder anchor evidence must not execute payouts".to_string());
        }

        if self.reward_truth {
            return Err("rewarder anchor evidence must not become reward truth".to_string());
        }

        if self.paid_unlock_authority {
            return Err("rewarder anchor evidence must not unlock paid content".to_string());
        }

        if self.settlement_truth {
            return Err("rewarder anchor evidence must not become settlement truth".to_string());
        }

        if self.outside_chain_truth {
            return Err(
                "rewarder anchor evidence must not make any outside chain ROC truth".to_string(),
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

fn sample_manifest() -> RewardManifest {
    let snapshot = AccountingSnapshot {
        produced_at_millis: 1_777_600_000_000,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_phase5_rewarder_a".to_string(),
                bytes_stored: 800,
                bytes_served: 200,
                uptime_seconds: 60,
            },
            AccountContribution {
                account: "acct_phase5_rewarder_b".to_string(),
                bytes_stored: 400,
                bytes_served: 100,
                uptime_seconds: 30,
            },
        ],
    };

    let policy = RewardPolicy {
        id: "policy:phase5-anchor-dry-run".to_string(),
        hash: format!("b3:{}", "d".repeat(64)),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_string(),
    };

    let input = ComputeInput {
        epoch_id: "epoch:phase5:r1:rewarder".to_string(),
        inputs_cid: ContentCid::parse(format!("b3:{}", "e".repeat(64)))
            .expect("sample inputs cid should be canonical b3"),
        policy,
        snapshot,
        dry_run: true,
        idempotency_salt: "svc-rewarder|phase5-anchor-dry-run".to_string(),
    };

    compute_manifest(input, IntentResult::DryRun).expect("sample reward manifest should compute")
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "rewarder anchor evidence must not expose forbidden authority key `{forbidden}`"
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

#[test]
fn reward_manifest_commitment_can_be_referenced_as_anchor_evidence_only() {
    let manifest = sample_manifest();

    assert!(
        manifest.commitment.starts_with("b3:"),
        "reward manifest commitment must be canonical b3"
    );
    assert!(
        manifest.run_key.starts_with("b3:"),
        "reward manifest run key must be canonical b3"
    );
    assert!(
        !manifest.payouts.is_empty(),
        "sample manifest should contain deterministic payout planning rows"
    );

    let report = RewarderAnchorEvidenceReport::for_manifest(&manifest);
    report
        .validate()
        .expect("anchor evidence report should validate");

    assert_eq!(report.schema, REPORT_SCHEMA);
    assert_eq!(report.reward_manifest_commitment, manifest.commitment);
    assert_eq!(report.reward_run_key, manifest.run_key);
    assert_eq!(report.reward_inputs_cid, manifest.inputs_cid);
    assert_eq!(report.payout_count, manifest.payouts.len());

    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(!report.rewarder_side_effect);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.reward_truth);
    assert!(!report.paid_unlock_authority);
    assert!(!report.settlement_truth);
    assert!(!report.outside_chain_truth);

    let value = serde_json::to_value(&report).expect("report should serialize");
    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "reward_executed",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn rewarder_anchor_evidence_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "rewarder_side_effect",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "reward_truth",
        "paid_unlock_authority",
        "settlement_truth",
        "outside_chain_truth",
    ] {
        let mut report = RewarderAnchorEvidenceReport::for_manifest(&sample_manifest());

        match field {
            "rewarder_side_effect" => report.rewarder_side_effect = true,
            "wallet_side_effect" => report.wallet_side_effect = true,
            "ledger_side_effect" => report.ledger_side_effect = true,
            "payout_side_effect" => report.payout_side_effect = true,
            "reward_truth" => report.reward_truth = true,
            "paid_unlock_authority" => report.paid_unlock_authority = true,
            "settlement_truth" => report.settlement_truth = true,
            "outside_chain_truth" => report.outside_chain_truth = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            report.validate().is_err(),
            "authority flag must reject when enabled: {field}"
        );
    }

    let clean = serde_json::to_value(RewarderAnchorEvidenceReport::for_manifest(
        &sample_manifest(),
    ))
    .expect("report should serialize");

    for field in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "reward_executed",
        "settlement_status",
        "finalized",
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
            serde_json::from_value::<RewarderAnchorEvidenceReport>(poisoned).is_err(),
            "report DTO must reject unknown authority field: {field}"
        );
    }

    let mut report = RewarderAnchorEvidenceReport::for_manifest(&sample_manifest());
    report.checkpoint_commitment = "not-a-b3-hash".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint commitment must be canonical b3"
    );

    let mut report = RewarderAnchorEvidenceReport::for_manifest(&sample_manifest());
    report.reward_manifest_commitment = format!("b3:{}", "A".repeat(64));
    assert!(
        report.validate().is_err(),
        "reward manifest commitment must be lowercase canonical b3"
    );
}

#[test]
fn rewarder_phase5_docs_record_anchor_only_non_authority_boundary() {
    let docs = read(crate_dir().join("docs/quickchain-preflight.md"));
    let normalized = docs.to_ascii_lowercase();

    for required in [
        "phase 5 round 1 anchor-only dry-run evidence boundary",
        "this means a deterministic reward manifest commitment may be referenced",
        "the report is evidence only",
        "it is not payout execution",
        "it is not reward truth",
        "it is not wallet truth",
        "it is not ledger truth",
        "it is not balance truth",
        "it is not paid-unlock authority",
        "it is not external-chain roc truth",
        "svc-rewarder remains deterministic payout planning only",
        "svc-wallet remains the mutation front-door",
        "ron-ledger remains durable economic truth",
    ] {
        assert!(
            normalized.contains(required),
            "svc-rewarder docs missing Phase 5 Round 1 boundary marker: {required}"
        );
    }
}

#[test]
fn rewarder_runtime_source_does_not_gain_phase5_anchor_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-rewarder Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "anchor_payout",
            "payout_from_anchor",
            "settle_from_anchor",
            "commit_from_anchor",
            "apply_anchor",
            "anchor_wallet_receipt",
            "anchor_ledger_receipt",
            "paid_unlock_from_anchor",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "payout_side_effect: true",
            "reward_truth: true",
            "settlement_truth: true",
            "outside_chain_truth: true",
            "bridge_settlement",
            "external_settlement",
            "solana",
            "rox",
            "liquidity_pool",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-rewarder source must not implement Phase 5 anchor runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
