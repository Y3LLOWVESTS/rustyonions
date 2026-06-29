#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 3 selected external-posture boundary tests for svc-rewarder.
//! RO:WHY — Reward manifests may be referenced by the chosen anchor-only external posture,
//! but svc-rewarder must remain deterministic payout planning only.
//! RO:INTERACTS — RewardManifest commitment, ComputeEpochRequest, SettlementBatch,
//! source boundary, strict evidence-only report shape.
//! RO:INVARIANTS — anchor-only; report-only; evidence-only; no direct reward eligibility;
//! no payout execution; no wallet/ledger mutation; no paid unlock; no bridge/market/liquidity.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents the chosen external posture from becoming reward, payout,
//! wallet, ledger, settlement, bridge, outside-program, exchange, or market authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase5_external_posture_reward_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    http::dto::ComputeEpochRequest,
    inputs::{
        AccountContribution, AccountingSnapshot, ContentCid, RewardFundingSource, RewardPolicy,
    },
    outputs::{IntentResult, RewardManifest, SettlementBatch},
};

const REPORT_SCHEMA: &str = "svc-rewarder.quickchain-external-posture-evidence.v1";
const CHECKPOINT_COMMITMENT: &str =
    "b3:cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";
const INPUTS_CID: &str = "b3:abababababababababababababababababababababababababababababababab";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RewarderExternalPostureEvidenceReport {
    schema: String,
    produced_at_ms: u64,
    chain_id: String,
    epoch_id: String,
    source: String,
    posture_id: String,
    chosen_posture: String,
    posture_semantics: String,
    checkpoint_commitment: String,
    reward_manifest_commitment: String,
    reward_run_key: String,
    reward_inputs_cid: String,
    payout_count: usize,
    anchor_only_selected: bool,
    report_only: bool,
    evidence_only: bool,
    wallet_ledger_truth_canonical: bool,
    direct_reward_eligibility: bool,
    rewarder_side_effect: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    payout_side_effect: bool,
    reward_truth: bool,
    paid_unlock_authority: bool,
    settlement_truth: bool,
    outside_da_truth: bool,
    outside_chain_truth: bool,
    outside_program_authority: bool,
    bridge_authority: bool,
    exchange_facing_authority: bool,
    public_market: bool,
    liquidity_enabled: bool,
    bonded_economy_authority: bool,
}

impl RewarderExternalPostureEvidenceReport {
    fn for_manifest(manifest: &RewardManifest) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            produced_at_ms: 1_777_700_001_000,
            chain_id: "roc-dev".to_string(),
            epoch_id: manifest.epoch_id.clone(),
            source: "svc-rewarder".to_string(),
            posture_id: "posture:phase5-round3:anchor-only".to_string(),
            chosen_posture: "anchor_only".to_string(),
            posture_semantics: "evidence_and_anchoring_only".to_string(),
            checkpoint_commitment: CHECKPOINT_COMMITMENT.to_string(),
            reward_manifest_commitment: manifest.commitment.clone(),
            reward_run_key: manifest.run_key.clone(),
            reward_inputs_cid: manifest.inputs_cid.clone(),
            payout_count: manifest.payouts.len(),
            anchor_only_selected: true,
            report_only: true,
            evidence_only: true,
            wallet_ledger_truth_canonical: true,
            direct_reward_eligibility: false,
            rewarder_side_effect: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            reward_truth: false,
            paid_unlock_authority: false,
            settlement_truth: false,
            outside_da_truth: false,
            outside_chain_truth: false,
            outside_program_authority: false,
            bridge_authority: false,
            exchange_facing_authority: false,
            public_market: false,
            liquidity_enabled: false,
            bonded_economy_authority: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPORT_SCHEMA {
            return Err("invalid external posture report schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("external posture report produced_at_ms must be nonzero".to_string());
        }

        for (field, value) in [
            ("checkpoint_commitment", self.checkpoint_commitment.as_str()),
            (
                "reward_manifest_commitment",
                self.reward_manifest_commitment.as_str(),
            ),
            ("reward_inputs_cid", self.reward_inputs_cid.as_str()),
        ] {
            validate_b3(field, value)?;
        }

        if self.chain_id.is_empty()
            || self.epoch_id.is_empty()
            || self.source != "svc-rewarder"
            || self.posture_id.is_empty()
            || self.reward_run_key.is_empty()
        {
            return Err("external posture report identity fields must be explicit".to_string());
        }

        if self.chosen_posture != "anchor_only" {
            return Err("svc-rewarder external posture report must stay anchor-only".to_string());
        }

        if self.posture_semantics != "evidence_and_anchoring_only" {
            return Err(
                "svc-rewarder external posture report must stay evidence-and-anchoring-only"
                    .to_string(),
            );
        }

        if !self.anchor_only_selected {
            return Err("anchor-only posture must be explicitly selected".to_string());
        }

        if !self.report_only {
            return Err("external posture report must remain report-only".to_string());
        }

        if !self.evidence_only {
            return Err("external posture report must remain evidence-only".to_string());
        }

        if !self.wallet_ledger_truth_canonical {
            return Err("wallet/ledger truth must remain canonical".to_string());
        }

        for (field, value) in [
            ("direct_reward_eligibility", self.direct_reward_eligibility),
            ("rewarder_side_effect", self.rewarder_side_effect),
            ("wallet_side_effect", self.wallet_side_effect),
            ("ledger_side_effect", self.ledger_side_effect),
            ("payout_side_effect", self.payout_side_effect),
            ("reward_truth", self.reward_truth),
            ("paid_unlock_authority", self.paid_unlock_authority),
            ("settlement_truth", self.settlement_truth),
            ("outside_da_truth", self.outside_da_truth),
            ("outside_chain_truth", self.outside_chain_truth),
            ("outside_program_authority", self.outside_program_authority),
            ("bridge_authority", self.bridge_authority),
            ("exchange_facing_authority", self.exchange_facing_authority),
            ("public_market", self.public_market),
            ("liquidity_enabled", self.liquidity_enabled),
            ("bonded_economy_authority", self.bonded_economy_authority),
        ] {
            if value {
                return Err(format!(
                    "external posture report must not claim authority field: {field}"
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

fn accounting_snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_777_700_000_000,
        pool_minor_units: AmountMinor(10_000),
        contributions: vec![
            AccountContribution {
                account: "acct_phase5_rewarder_alice".to_string(),
                bytes_stored: 1_000,
                bytes_served: 500,
                uptime_seconds: 120,
            },
            AccountContribution {
                account: "acct_phase5_rewarder_bob".to_string(),
                bytes_stored: 2_000,
                bytes_served: 250,
                uptime_seconds: 60,
            },
        ],
    }
}

fn reward_policy() -> RewardPolicy {
    RewardPolicy {
        id: "policy:phase5:external-posture".to_string(),
        hash: POLICY_HASH.to_string(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(10_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_string(),
    }
}

fn manifest() -> RewardManifest {
    let input = ComputeInput {
        epoch_id: "epoch:phase5:round3:rewarder".to_string(),
        inputs_cid: ContentCid::parse(INPUTS_CID).expect("test inputs cid should parse"),
        policy: reward_policy(),
        snapshot: accounting_snapshot(),
        dry_run: true,
        idempotency_salt: "svc-rewarder|phase5-round3".to_string(),
    };

    compute_manifest(input, IntentResult::DryRun).expect("reward manifest should compute")
}

#[test]
fn rewarder_external_posture_report_is_anchor_only_evidence_and_not_reward_authority() {
    let manifest = manifest();
    let report = RewarderExternalPostureEvidenceReport::for_manifest(&manifest);

    report
        .validate()
        .expect("clean external posture report should validate");

    assert_eq!(report.schema, REPORT_SCHEMA);
    assert_eq!(report.chosen_posture, "anchor_only");
    assert_eq!(report.posture_semantics, "evidence_and_anchoring_only");
    assert_eq!(report.reward_manifest_commitment, manifest.commitment);
    assert_eq!(report.reward_run_key, manifest.run_key);
    assert_eq!(report.payout_count, manifest.payouts.len());

    assert!(report.anchor_only_selected);
    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(report.wallet_ledger_truth_canonical);

    assert!(!report.direct_reward_eligibility);
    assert!(!report.rewarder_side_effect);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.reward_truth);
    assert!(!report.paid_unlock_authority);
    assert!(!report.settlement_truth);
    assert!(!report.outside_da_truth);
    assert!(!report.outside_chain_truth);
    assert!(!report.outside_program_authority);
    assert!(!report.bridge_authority);
    assert!(!report.exchange_facing_authority);
    assert!(!report.public_market);
    assert!(!report.liquidity_enabled);
    assert!(!report.bonded_economy_authority);
}

#[test]
fn rewarder_external_posture_report_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(RewarderExternalPostureEvidenceReport::for_manifest(
        &manifest(),
    ))
    .expect("report should serialize");

    for field in [
        "anchor_only_selected",
        "report_only",
        "evidence_only",
        "wallet_ledger_truth_canonical",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(false);

        let decoded = serde_json::from_value::<RewarderExternalPostureEvidenceReport>(poisoned)
            .expect("known false required flag should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "external posture report must reject false required field {field}"
        );
    }

    for field in [
        "direct_reward_eligibility",
        "rewarder_side_effect",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "reward_truth",
        "paid_unlock_authority",
        "settlement_truth",
        "outside_da_truth",
        "outside_chain_truth",
        "outside_program_authority",
        "bridge_authority",
        "exchange_facing_authority",
        "public_market",
        "liquidity_enabled",
        "bonded_economy_authority",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(true);

        let decoded = serde_json::from_value::<RewarderExternalPostureEvidenceReport>(poisoned)
            .expect("known authority field should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "external posture report must reject authority field {field}"
        );
    }

    let mut unknown = clean;
    unknown["external_settlement_authorized"] = json!(true);

    assert!(
        serde_json::from_value::<RewarderExternalPostureEvidenceReport>(unknown).is_err(),
        "unknown external posture authority fields must reject"
    );
}

#[test]
fn rewarder_compute_request_rejects_external_posture_authority_poison_fields() {
    let clean = json!({
        "inputs_cid": INPUTS_CID,
        "policy_id": "policy:phase5:external-posture",
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": accounting_snapshot(),
        "policy": reward_policy()
    });

    serde_json::from_value::<ComputeEpochRequest>(clean.clone())
        .expect("clean compute request should deserialize");

    for field in [
        "external_posture",
        "chosen_external_posture",
        "direct_reward_eligibility",
        "payout_from_external_posture",
        "wallet_side_effect",
        "ledger_side_effect",
        "reward_truth",
        "paid_unlock_authority",
        "bridge_authority",
        "outside_program_authority",
        "exchange_facing_authority",
        "public_market",
        "liquidity_enabled",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = json!("poison");

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(poisoned).is_err(),
            "ComputeEpochRequest must reject external posture authority field {field}"
        );
    }
}

#[test]
fn reward_manifest_and_settlement_batch_do_not_become_external_posture_payout_or_truth() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest)
        .expect("reward manifest should still produce wallet handoff planning batch");

    let manifest_json = serde_json::to_string(&manifest).expect("manifest should serialize");
    let settlement_json = serde_json::to_string(&settlement).expect("settlement should serialize");
    let combined = format!("{manifest_json}\n{settlement_json}");

    assert!(combined.contains("dry_run"));
    assert!(combined.contains("policy"));
    assert!(combined.contains("payout"));

    for forbidden in [
        "external_posture",
        "chosen_external_posture",
        "external_da_selected",
        "external_l2_selected",
        "hybrid_selected",
        "direct_reward_eligibility",
        "payout_from_external_posture",
        "reward_truth",
        "settlement_truth",
        "outside_chain_truth",
        "paid_unlock_authority",
        "bridge_authority",
        "outside_program_authority",
        "exchange_facing_authority",
        "public_market",
        "liquidity_enabled",
        "rox_runtime",
        "solana_runtime",
    ] {
        assert!(
            !combined.contains(forbidden),
            "rewarder manifest/settlement batch must not expose external posture authority: {forbidden}"
        );
    }
}

#[test]
fn rewarder_source_does_not_construct_external_posture_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-rewarder Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "external_posture_payout",
            "payout_from_external_posture",
            "reward_from_external_posture",
            "settle_from_external_posture",
            "commit_from_external_posture",
            "apply_external_posture",
            "external_posture_wallet_receipt",
            "external_posture_ledger_receipt",
            "paid_unlock_from_external_posture",
            "direct_reward_eligibility: true",
            "rewarder_side_effect: true",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "payout_side_effect: true",
            "reward_truth: true",
            "settlement_truth: true",
            "outside_da_truth: true",
            "outside_chain_truth: true",
            "outside_program_authority: true",
            "bridge_authority: true",
            "exchange_facing_authority: true",
            "public_market: true",
            "liquidity_enabled: true",
            "bonded_economy_authority: true",
            "external_da_selected: true",
            "external_l2_selected: true",
            "hybrid_selected: true",
            "solana_runtime",
            "rox_runtime",
            "bridge_settlement",
            "exchange_facing",
            "liquidity_pool",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-rewarder source must not implement Phase 5 Round 3 external posture runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
