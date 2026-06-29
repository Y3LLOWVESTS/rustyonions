#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback boundary tests for svc-rewarder.
//! RO:WHY — Reward manifests may be referenced by DA fallback evidence, but challenge/archive material must never become direct reward, payout, wallet, ledger, pruning, paid-unlock, or outside-truth authority.
//! RO:INTERACTS — RewardManifest commitment, compute_manifest, source boundary, strict evidence-only report shape.
//! RO:INVARIANTS — report-only; evidence-only; challenge checked; archive restore checked; pruning blocked; no payout execution; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents DA/archive/challenge evidence from becoming reward entitlement or mutation authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase5_da_fallback_reward_boundary.

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

const REPORT_SCHEMA: &str = "svc-rewarder.quickchain-da-fallback-evidence.v1";
const CHECKPOINT_HASH: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DATA_AVAILABILITY_ROOT: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RewarderDaFallbackEvidenceReport {
    schema: String,
    produced_at_ms: u64,
    chain_id: String,
    epoch_id: String,
    source: String,
    fallback_plan_id: String,
    checkpoint_hash: String,
    data_availability_root: String,
    challenged_chunk_id: Option<String>,
    reward_manifest_commitment: String,
    reward_run_key: String,
    reward_inputs_cid: String,
    payout_count: usize,
    report_only: bool,
    evidence_only: bool,
    archive_fallback_checked: bool,
    missing_data_challenge_checked: bool,
    restore_path_checked: bool,
    pruning_blocked: bool,
    direct_reward_eligibility: bool,
    rewarder_side_effect: bool,
    wallet_side_effect: bool,
    ledger_side_effect: bool,
    payout_side_effect: bool,
    reward_truth: bool,
    carrier_reward_authority: bool,
    archive_reward_authority: bool,
    paid_unlock_authority: bool,
    pruning_authority: bool,
    settlement_truth: bool,
    outside_data_availability_truth: bool,
    outside_chain_truth: bool,
}

impl RewarderDaFallbackEvidenceReport {
    fn for_manifest(manifest: &RewardManifest) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            produced_at_ms: 1_777_700_001_000,
            chain_id: "roc-dev".to_string(),
            epoch_id: manifest.epoch_id.clone(),
            source: "svc-rewarder".to_string(),
            fallback_plan_id: "fallback-plan:phase5:r2:rewarder".to_string(),
            checkpoint_hash: CHECKPOINT_HASH.to_string(),
            data_availability_root: DATA_AVAILABILITY_ROOT.to_string(),
            challenged_chunk_id: Some("chunk:reward-manifest:0001".to_string()),
            reward_manifest_commitment: manifest.commitment.clone(),
            reward_run_key: manifest.run_key.clone(),
            reward_inputs_cid: manifest.inputs_cid.clone(),
            payout_count: manifest.payouts.len(),
            report_only: true,
            evidence_only: true,
            archive_fallback_checked: true,
            missing_data_challenge_checked: true,
            restore_path_checked: true,
            pruning_blocked: true,
            direct_reward_eligibility: false,
            rewarder_side_effect: false,
            wallet_side_effect: false,
            ledger_side_effect: false,
            payout_side_effect: false,
            reward_truth: false,
            carrier_reward_authority: false,
            archive_reward_authority: false,
            paid_unlock_authority: false,
            pruning_authority: false,
            settlement_truth: false,
            outside_data_availability_truth: false,
            outside_chain_truth: false,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPORT_SCHEMA {
            return Err("invalid rewarder DA fallback evidence schema".to_string());
        }

        if self.produced_at_ms == 0 {
            return Err("produced_at_ms must be nonzero".to_string());
        }

        if self.payout_count == 0 {
            return Err("DA fallback report must reference a nonempty reward manifest".to_string());
        }

        validate_token("chain_id", &self.chain_id)?;
        validate_token("epoch_id", &self.epoch_id)?;
        validate_token("source", &self.source)?;
        validate_token("fallback_plan_id", &self.fallback_plan_id)?;
        if let Some(challenged_chunk_id) = self.challenged_chunk_id.as_deref() {
            validate_token("challenged_chunk_id", challenged_chunk_id)?;
        }

        validate_b3("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3("data_availability_root", &self.data_availability_root)?;
        validate_b3(
            "reward_manifest_commitment",
            &self.reward_manifest_commitment,
        )?;
        validate_b3("reward_run_key", &self.reward_run_key)?;
        validate_b3("reward_inputs_cid", &self.reward_inputs_cid)?;

        if !self.report_only {
            return Err("rewarder DA fallback evidence must remain report-only".to_string());
        }
        if !self.evidence_only {
            return Err("rewarder DA fallback evidence must remain evidence-only".to_string());
        }
        if !self.archive_fallback_checked {
            return Err("rewarder DA fallback evidence must check archive fallback".to_string());
        }
        if !self.missing_data_challenge_checked {
            return Err(
                "rewarder DA fallback evidence must check missing-data challenge handling"
                    .to_string(),
            );
        }
        if !self.restore_path_checked {
            return Err("rewarder DA fallback evidence must check restore path".to_string());
        }
        if !self.pruning_blocked {
            return Err("rewarder DA fallback evidence must keep pruning blocked".to_string());
        }

        if self.direct_reward_eligibility {
            return Err(
                "DA fallback evidence must not become direct reward eligibility".to_string(),
            );
        }
        if self.rewarder_side_effect {
            return Err("DA fallback evidence must not mutate rewarder state".to_string());
        }
        if self.wallet_side_effect {
            return Err("DA fallback evidence must not mutate wallet state".to_string());
        }
        if self.ledger_side_effect {
            return Err("DA fallback evidence must not mutate ledger state".to_string());
        }
        if self.payout_side_effect {
            return Err("DA fallback evidence must not execute payouts".to_string());
        }
        if self.reward_truth {
            return Err("DA fallback evidence must not become reward truth".to_string());
        }
        if self.carrier_reward_authority {
            return Err(
                "DA fallback evidence must not become carrier reward authority".to_string(),
            );
        }
        if self.archive_reward_authority {
            return Err(
                "DA fallback evidence must not become archive reward authority".to_string(),
            );
        }
        if self.paid_unlock_authority {
            return Err("DA fallback evidence must not unlock paid content".to_string());
        }
        if self.pruning_authority {
            return Err("DA fallback evidence must not authorize pruning".to_string());
        }
        if self.settlement_truth {
            return Err("DA fallback evidence must not become settlement truth".to_string());
        }
        if self.outside_data_availability_truth {
            return Err("outside DA material must not become rewarder truth".to_string());
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

fn read_sources(paths: &[&str]) -> String {
    let mut out = String::new();
    for rel in paths {
        out.push_str(&format!("\n// FILE: {rel}\n"));
        out.push_str(&read(crate_dir().join(rel)));
    }
    out
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
                "serialized DA fallback rewarder report must not expose authority key `{key}`: {value}"
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

fn manifest() -> RewardManifest {
    let snapshot = AccountingSnapshot {
        produced_at_millis: 1_777_700_000_000,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_phase5_da_archive_a".to_string(),
                bytes_stored: 100,
                bytes_served: 40,
                uptime_seconds: 10,
            },
            AccountContribution {
                account: "acct_phase5_da_archive_b".to_string(),
                bytes_stored: 80,
                bytes_served: 20,
                uptime_seconds: 30,
            },
        ],
    };

    let policy = RewardPolicy {
        id: "policy:phase5:da-fallback".to_string(),
        hash: format!("b3:{}", "c".repeat(64)),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_string(),
    };

    let input = ComputeInput {
        epoch_id: "epoch:phase5:r2:rewarder".to_string(),
        inputs_cid: ContentCid::parse(format!("b3:{}", "d".repeat(64)))
            .expect("input cid should be canonical"),
        policy,
        snapshot,
        dry_run: true,
        idempotency_salt: "svc-rewarder|quickchain-phase5-da-fallback".to_string(),
    };

    compute_manifest(input, IntentResult::DryRun).expect("manifest should compute")
}

fn report() -> RewarderDaFallbackEvidenceReport {
    let manifest = manifest();
    let report = RewarderDaFallbackEvidenceReport::for_manifest(&manifest);
    report
        .validate()
        .expect("rewarder DA fallback evidence report should validate");
    report
}

#[test]
fn rewarder_da_fallback_report_is_evidence_only_and_blocks_pruning_authority() {
    let report = report();

    assert_eq!(report.schema, REPORT_SCHEMA);
    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(report.archive_fallback_checked);
    assert!(report.missing_data_challenge_checked);
    assert!(report.restore_path_checked);
    assert!(report.pruning_blocked);
    assert!(report.payout_count > 0);

    assert!(!report.direct_reward_eligibility);
    assert!(!report.rewarder_side_effect);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.reward_truth);
    assert!(!report.carrier_reward_authority);
    assert!(!report.archive_reward_authority);
    assert!(!report.paid_unlock_authority);
    assert!(!report.pruning_authority);
    assert!(!report.settlement_truth);
    assert!(!report.outside_data_availability_truth);
    assert!(!report.outside_chain_truth);

    let encoded = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "reward_payout",
        "wallet_receipt",
        "ledger_receipt",
        "carrier_reward_receipt",
        "archive_reward_receipt",
        "paid_unlock",
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
fn rewarder_da_fallback_report_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(report()).expect("report should serialize");

    for field in [
        "report_only",
        "evidence_only",
        "archive_fallback_checked",
        "missing_data_challenge_checked",
        "restore_path_checked",
        "pruning_blocked",
    ] {
        let mut poisoned =
            serde_json::from_value::<RewarderDaFallbackEvidenceReport>(clean.clone())
                .expect("clean report should deserialize");

        match field {
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
            "required DA fallback check/blocker flag must reject when disabled: {field}"
        );
    }

    for field in [
        "direct_reward_eligibility",
        "rewarder_side_effect",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "reward_truth",
        "carrier_reward_authority",
        "archive_reward_authority",
        "paid_unlock_authority",
        "pruning_authority",
        "settlement_truth",
        "outside_data_availability_truth",
        "outside_chain_truth",
    ] {
        let mut poisoned =
            serde_json::from_value::<RewarderDaFallbackEvidenceReport>(clean.clone())
                .expect("clean report should deserialize");

        match field {
            "direct_reward_eligibility" => poisoned.direct_reward_eligibility = true,
            "rewarder_side_effect" => poisoned.rewarder_side_effect = true,
            "wallet_side_effect" => poisoned.wallet_side_effect = true,
            "ledger_side_effect" => poisoned.ledger_side_effect = true,
            "payout_side_effect" => poisoned.payout_side_effect = true,
            "reward_truth" => poisoned.reward_truth = true,
            "carrier_reward_authority" => poisoned.carrier_reward_authority = true,
            "archive_reward_authority" => poisoned.archive_reward_authority = true,
            "paid_unlock_authority" => poisoned.paid_unlock_authority = true,
            "pruning_authority" => poisoned.pruning_authority = true,
            "settlement_truth" => poisoned.settlement_truth = true,
            "outside_data_availability_truth" => poisoned.outside_data_availability_truth = true,
            "outside_chain_truth" => poisoned.outside_chain_truth = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "DA fallback authority flag must reject when enabled: {field}"
        );
    }

    for field in [
        "wallet_issue",
        "ledger_commit",
        "wallet_receipt",
        "carrier_reward_receipt",
        "archive_reward_receipt",
        "automatic_reward",
        "pay_from_archive_challenge",
        "paid_unlock",
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
            .insert(field.to_string(), json!("smuggled-da-authority"));

        assert!(
            serde_json::from_value::<RewarderDaFallbackEvidenceReport>(poisoned).is_err(),
            "DA fallback report must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn rewarder_da_fallback_report_rejects_bad_hashes_and_empty_context() {
    let mut bad_checkpoint_report = report();
    bad_checkpoint_report.checkpoint_hash = "not-a-b3-hash".to_string();
    assert!(
        bad_checkpoint_report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );

    let mut bad_da_root_report = report();
    bad_da_root_report.data_availability_root = "b3:ABC".to_string();
    assert!(
        bad_da_root_report.validate().is_err(),
        "DA root must be canonical lowercase b3"
    );

    let mut bad_commitment_report = report();
    bad_commitment_report.reward_manifest_commitment = format!("b3:{}", "g".repeat(64));
    assert!(
        bad_commitment_report.validate().is_err(),
        "manifest commitment must be lowercase hex"
    );

    let mut bad_chunk_report = report();
    bad_chunk_report.challenged_chunk_id = Some("bad chunk id with spaces".to_string());
    assert!(
        bad_chunk_report.validate().is_err(),
        "challenged chunk id must be bounded visible token"
    );

    let mut empty_manifest_report = report();
    empty_manifest_report.payout_count = 0;
    assert!(
        empty_manifest_report.validate().is_err(),
        "report must not claim an empty manifest as reward evidence"
    );
}

#[test]
fn rewarder_da_fallback_evidence_cannot_drive_second_payout_or_carrier_reward() {
    let manifest = manifest();
    let report = RewarderDaFallbackEvidenceReport::for_manifest(&manifest);
    let encoded_manifest = serde_json::to_string(&manifest).expect("manifest should serialize");
    let encoded_report = serde_json::to_string(&report).expect("report should serialize");

    assert!(
        !encoded_report.contains("wallet_receipt"),
        "DA fallback report must not contain wallet receipt authority"
    );
    assert!(
        !encoded_report.contains("ledger_receipt"),
        "DA fallback report must not contain ledger receipt authority"
    );
    assert!(
        !encoded_report.contains("carrier_reward_receipt"),
        "DA fallback report must not contain carrier reward receipts"
    );
    assert!(
        !encoded_report.contains("archive_reward_receipt"),
        "DA fallback report must not contain archive reward receipts"
    );

    for forbidden in [
        "pay_from_missing_data_challenge",
        "mint_from_archive_restore",
        "issue_from_carrier_proof",
        "direct_archive_reward",
        "direct_carrier_reward",
        "pruning_allowed",
        "outside_da_reward_truth",
    ] {
        assert!(
            !encoded_manifest.contains(forbidden),
            "reward manifest must not grow DA fallback direct-payout vocabulary: {forbidden}"
        );
    }
}

#[test]
fn rewarder_source_does_not_implement_da_fallback_reward_runtime_authority() {
    let source = strip_line_comments(&read_sources(&[
        "src/core/compute.rs",
        "src/core/invariants.rs",
        "src/inputs/accounting.rs",
        "src/inputs/policy.rs",
        "src/outputs/intents.rs",
        "src/outputs/manifest.rs",
        "src/outputs/wallet.rs",
        "src/http/dto.rs",
        "src/http/handlers.rs",
    ]))
    .to_ascii_lowercase();

    for forbidden in [
        "pay_from_da_fallback",
        "reward_from_da_fallback",
        "issue_from_da_fallback",
        "mint_from_archive_restore",
        "pay_from_missing_data_challenge",
        "direct_carrier_reward",
        "direct_archive_reward",
        "archive_reward_receipt",
        "carrier_reward_receipt",
        "da_fallback_wallet_issue",
        "da_fallback_ledger_commit",
        "pruning_authority: true",
        "outside_data_availability_truth: true",
        "outside_chain_truth: true",
        "bridge_settlement",
        "external_settlement",
        "solana_runtime",
        "rox_runtime",
    ] {
        assert!(
            !source.contains(forbidden),
            "svc-rewarder source must not implement DA fallback reward/runtime authority via `{forbidden}`"
        );
    }
}
