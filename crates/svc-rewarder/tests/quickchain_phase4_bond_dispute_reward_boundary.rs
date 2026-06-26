#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 2 disputed-bond reward boundary tests for svc-rewarder.
//! RO:WHY — Challenge/freeze/appeal/slash simulation must not become reward,
//! penalty, payout, wallet, ledger, staking, liquidity, bridge, or external
//! settlement authority.
//! RO:INTERACTS — ComputeEpochRequest, AccountingSnapshot, RewardPolicy,
//! RewardManifest, SettlementBatch, WalletIssueRequest, source boundary.
//! RO:INVARIANTS — rewarder remains deterministic payout planning only;
//! dispute simulation cannot mint, slash, freeze, capture, or reward by itself.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects Phase 4 Round 2 dispute/slash/challenge authority smuggling.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    http::dto::ComputeEpochRequest,
    inputs::{
        AccountContribution, AccountingSnapshot, ContentCid, RewardFundingSource, RewardPolicy,
    },
    outputs::{
        plan_settlement_intents, IntentResult, RewardManifest, SettlementBatch, WalletIssueRequest,
    },
};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const INPUTS_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
    "slash_reward",
    "slash_reward_recipient",
    "slash_penalty_minor",
    "automatic_slash",
    "auto_slash_now",
    "execute_slash",
    "commit_slash_decision",
    "capture_disputed_bond",
    "bond_forfeiture",
    "bond_penalty",
    "bond_dispute_payout",
    "dispute_reward",
    "reward_from_dispute",
    "penalty_reward",
    "validator_reward",
    "validator_reward_receipt",
    "wallet_receipt",
    "ledger_receipt",
    "wallet_mutation",
    "ledger_mutation",
    "payout_execution",
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

fn valid_snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_777_314_000_000,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_reward_plan_alice".to_owned(),
                bytes_stored: 100,
                bytes_served: 50,
                uptime_seconds: 10,
            },
            AccountContribution {
                account: "acct_reward_plan_bob".to_owned(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 20,
            },
        ],
    }
}

fn zero_score_snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_777_314_000_000,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![AccountContribution {
            account: "acct_reward_plan_zero".to_owned(),
            bytes_stored: 0,
            bytes_served: 0,
            uptime_seconds: 0,
        }],
    }
}

fn valid_policy() -> RewardPolicy {
    RewardPolicy {
        id: POLICY_ID.to_owned(),
        hash: POLICY_HASH.to_owned(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_owned(),
    }
}

fn valid_manifest() -> RewardManifest {
    compute_manifest(
        ComputeInput {
            epoch_id: "epoch_phase4_r2_reward_plan".to_owned(),
            inputs_cid: ContentCid::parse(INPUTS_CID).expect("valid input cid"),
            policy: valid_policy(),
            snapshot: valid_snapshot(),
            dry_run: true,
            idempotency_salt: "svc-rewarder|phase4-r2|reward-plan".to_owned(),
        },
        IntentResult::DryRun,
    )
    .expect("valid manifest should compute")
}

fn clean_compute_request_json() -> Value {
    json!({
        "inputs_cid": INPUTS_CID,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true
    })
}

fn assert_no_key_recursive(value: &Value, forbidden_key: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(forbidden_key),
                "serialized rewarder artifact must not expose forbidden key: {forbidden_key}"
            );
            for nested in map.values() {
                assert_no_key_recursive(nested, forbidden_key);
            }
        }
        Value::Array(items) => {
            for nested in items {
                assert_no_key_recursive(nested, forbidden_key);
            }
        }
        _ => {}
    }
}

#[test]
fn dispute_simulation_fields_reject_at_rewarder_input_boundaries() {
    for field in PHASE4_ROUND2_DISPUTE_AUTHORITY_KEYS {
        let mut request = clean_compute_request_json();
        request
            .as_object_mut()
            .expect("request JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("client-supplied-dispute-authority"),
            );
        assert!(
            serde_json::from_value::<ComputeEpochRequest>(request).is_err(),
            "ComputeEpochRequest must reject Phase 4 Round 2 dispute authority field: {field}"
        );

        let mut snapshot =
            serde_json::to_value(valid_snapshot()).expect("snapshot should serialize");
        snapshot
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("snapshot-supplied-dispute-authority"),
            );
        assert!(
            serde_json::from_value::<AccountingSnapshot>(snapshot).is_err(),
            "AccountingSnapshot must reject Phase 4 Round 2 dispute authority field: {field}"
        );

        let mut policy = serde_json::to_value(valid_policy()).expect("policy should serialize");
        policy
            .as_object_mut()
            .expect("policy JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("policy-supplied-dispute-authority"),
            );
        assert!(
            serde_json::from_value::<RewardPolicy>(policy).is_err(),
            "RewardPolicy must reject Phase 4 Round 2 dispute authority field: {field}"
        );
    }
}

#[test]
fn dispute_simulation_fields_reject_at_rewarder_output_and_wallet_handoff_boundaries() {
    let manifest = valid_manifest();
    let batch = plan_settlement_intents(&manifest).expect("settlement planning should succeed");
    let wallet_request = batch
        .intents
        .first()
        .expect("manifest should produce at least one settlement intent")
        .to_wallet_issue_request();

    for field in PHASE4_ROUND2_DISPUTE_AUTHORITY_KEYS {
        let mut manifest_json = serde_json::to_value(&manifest).expect("manifest should serialize");
        manifest_json
            .as_object_mut()
            .expect("manifest JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("manifest-supplied-dispute-authority"),
            );
        assert!(
            serde_json::from_value::<RewardManifest>(manifest_json).is_err(),
            "RewardManifest must reject Phase 4 Round 2 dispute authority field: {field}"
        );

        let mut batch_json = serde_json::to_value(&batch).expect("batch should serialize");
        batch_json
            .as_object_mut()
            .expect("batch JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("batch-supplied-dispute-authority"),
            );
        assert!(
            serde_json::from_value::<SettlementBatch>(batch_json).is_err(),
            "SettlementBatch must reject Phase 4 Round 2 dispute authority field: {field}"
        );

        let mut wallet_json =
            serde_json::to_value(&wallet_request).expect("wallet issue request should serialize");
        wallet_json
            .as_object_mut()
            .expect("wallet issue JSON should be object")
            .insert(
                (*field).to_owned(),
                json!("wallet-supplied-dispute-authority"),
            );
        assert!(
            serde_json::from_value::<WalletIssueRequest>(wallet_json).is_err(),
            "WalletIssueRequest must reject Phase 4 Round 2 dispute authority field: {field}"
        );
    }
}

#[test]
fn rewarder_does_not_emit_dispute_reward_when_scores_are_zero_or_dry_run() {
    let manifest = compute_manifest(
        ComputeInput {
            epoch_id: "epoch_phase4_r2_zero_score".to_owned(),
            inputs_cid: ContentCid::parse(INPUTS_CID).expect("valid input cid"),
            policy: valid_policy(),
            snapshot: zero_score_snapshot(),
            dry_run: true,
            idempotency_salt: "svc-rewarder|phase4-r2|zero-score".to_owned(),
        },
        IntentResult::DryRun,
    )
    .expect("zero-score dry-run manifest should compute");

    assert!(
        manifest.payouts.is_empty(),
        "zero-score inputs must not create any payout, including dispute/penalty rewards"
    );
    assert_eq!(manifest.totals.payout_minor_units.get(), 0);
    assert!(!manifest.ledger.emitted);
    assert_eq!(manifest.ledger.result, "dry_run");

    let manifest_json = serde_json::to_value(&manifest).expect("manifest should serialize");
    for forbidden in [
        "dispute_id",
        "slash_reward",
        "slash_penalty_minor",
        "dispute_reward",
        "reward_from_dispute",
        "penalty_reward",
        "validator_reward",
        "wallet_receipt",
        "ledger_receipt",
        "wallet_mutation",
        "ledger_mutation",
        "payout_execution",
        "public_staking_market",
        "liquidity_pool",
        "bridge_settlement",
        "external_settlement",
    ] {
        assert_no_key_recursive(&manifest_json, forbidden);
    }
}

#[test]
fn rewarder_source_does_not_construct_phase4_round2_dispute_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-rewarder Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "reward_from_dispute",
            "dispute_reward",
            "slash_reward",
            "penalty_reward",
            "bond_dispute_payout",
            "payout_from_dispute",
            "payout_from_slash",
            "slash_penalty_minor",
            "execute_slash",
            "commit_slash_decision",
            "capture_disputed_bond",
            "bond_forfeiture",
            "wallet_slash",
            "ledger_slash",
            "auto_slash_now",
            "freeze_bond_payout",
            "validator_reward_receipt",
            "public_staking_market",
            "liquidity_pool",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-rewarder source must not construct Phase 4 Round 2 dispute reward/slash authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
