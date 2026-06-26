#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 1 bond planning boundary tests for svc-rewarder.
//! RO:WHY — Bond reports and bond-like future inputs may inform planning later,
//! but rewarder must not become bond truth, slash truth, validator reward
//! authority, staking authority, liquidity authority, wallet mutation authority,
//! ledger mutation authority, bridge authority, or external settlement authority.
//! RO:INTERACTS — ComputeEpochRequest, AccountingSnapshot, RewardPolicy,
//! RewardManifest, SettlementBatch, WalletIssueRequest, source boundary.
//! RO:INVARIANTS — svc-rewarder remains deterministic payout planning only;
//! svc-wallet remains mutation front-door; ron-ledger remains economic truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects bond/slash/stake/liquidity authority smuggling.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary.

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
    outputs::{IntentResult, RewardManifest, SettlementBatch, WalletIssueRequest},
};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const INPUTS_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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

fn cid() -> ContentCid {
    ContentCid::parse(INPUTS_CID).expect("test CID should be canonical b3")
}

fn policy() -> RewardPolicy {
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

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_phase4_plan_b".to_owned(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_phase4_plan_a".to_owned(),
                bytes_stored: 100,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    }
}

fn compute_input() -> ComputeInput {
    ComputeInput {
        epoch_id: "epoch-phase4-r1-reward-planning".to_owned(),
        inputs_cid: cid(),
        policy: policy(),
        snapshot: snapshot(),
        dry_run: false,
        idempotency_salt: "svc-rewarder|phase4-round1-reward-planning".to_owned(),
    }
}

fn manifest() -> RewardManifest {
    compute_manifest(compute_input(), IntentResult::Accepted)
        .expect("valid planning input should compute manifest")
}

fn compute_request_json() -> Value {
    json!({
        "inputs_cid": INPUTS_CID,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true
    })
}

fn insert_top_level(mut value: Value, field: &str) -> Value {
    value
        .as_object_mut()
        .expect("JSON value should be object")
        .insert(
            field.to_owned(),
            json!("client-supplied-phase4-bond-authority"),
        );
    value
}

fn assert_no_forbidden_keys_recursive(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                assert!(
                    !PHASE4_ROUND1_BOND_AUTHORITY_KEYS.contains(&key.as_str()),
                    "Phase 4 bond/slash/stake/liquidity authority key leaked into rewarder artifact: {key}"
                );
                assert_no_forbidden_keys_recursive(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_forbidden_keys_recursive(item);
            }
        }
        _ => {}
    }
}

#[test]
fn rewarder_manifest_and_wallet_handoff_remain_planning_only_not_bond_enforcement() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest)
        .expect("reward manifest should produce wallet handoff batch");
    let wallet_batch = settlement.to_wallet_issue_batch();

    assert!(
        !manifest.payouts.is_empty(),
        "test manifest should plan payouts"
    );
    assert_eq!(
        settlement.total_minor_units,
        manifest.totals.payout_minor_units
    );
    assert_eq!(
        wallet_batch.total_minor_units,
        manifest.totals.payout_minor_units.get().to_string()
    );

    let manifest_value = serde_json::to_value(&manifest).expect("manifest serializes");
    let settlement_value = serde_json::to_value(&settlement).expect("settlement batch serializes");
    let wallet_value = serde_json::to_value(&wallet_batch).expect("wallet batch serializes");

    assert_no_forbidden_keys_recursive(&manifest_value);
    assert_no_forbidden_keys_recursive(&settlement_value);
    assert_no_forbidden_keys_recursive(&wallet_value);

    let encoded_wallet = serde_json::to_string(&wallet_batch).expect("wallet batch serializes");
    assert!(encoded_wallet.contains(r#""amount_minor":""#));
}

#[test]
fn rewarder_compute_inputs_reject_phase4_bond_authority_fields() {
    for field in PHASE4_ROUND1_BOND_AUTHORITY_KEYS {
        let request = insert_top_level(compute_request_json(), field);
        assert!(
            serde_json::from_value::<ComputeEpochRequest>(request).is_err(),
            "ComputeEpochRequest must reject Phase 4 bond authority field: {field}"
        );

        let snapshot = insert_top_level(
            serde_json::to_value(snapshot()).expect("snapshot serializes"),
            field,
        );
        assert!(
            serde_json::from_value::<AccountingSnapshot>(snapshot).is_err(),
            "AccountingSnapshot must reject Phase 4 bond authority field: {field}"
        );

        let policy = insert_top_level(
            serde_json::to_value(policy()).expect("policy serializes"),
            field,
        );
        assert!(
            serde_json::from_value::<RewardPolicy>(policy).is_err(),
            "RewardPolicy must reject Phase 4 bond authority field: {field}"
        );
    }
}

#[test]
fn rewarder_outputs_reject_phase4_bond_authority_fields() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest)
        .expect("reward manifest should produce settlement batch");
    let wallet_batch = settlement.to_wallet_issue_batch();
    let wallet_request = wallet_batch
        .requests
        .first()
        .expect("test should have at least one wallet issue request")
        .clone();

    for field in PHASE4_ROUND1_BOND_AUTHORITY_KEYS {
        let manifest_value = insert_top_level(
            serde_json::to_value(&manifest).expect("manifest serializes"),
            field,
        );
        assert!(
            serde_json::from_value::<RewardManifest>(manifest_value).is_err(),
            "RewardManifest must reject Phase 4 bond authority field: {field}"
        );

        let settlement_value = insert_top_level(
            serde_json::to_value(&settlement).expect("settlement serializes"),
            field,
        );
        assert!(
            serde_json::from_value::<SettlementBatch>(settlement_value).is_err(),
            "SettlementBatch must reject Phase 4 bond authority field: {field}"
        );

        let wallet_value = insert_top_level(
            serde_json::to_value(&wallet_request).expect("wallet request serializes"),
            field,
        );
        assert!(
            serde_json::from_value::<WalletIssueRequest>(wallet_value).is_err(),
            "WalletIssueRequest must reject Phase 4 bond authority field: {field}"
        );
    }
}

#[test]
fn bond_like_replay_is_still_rewarder_dedupe_not_second_payout_or_slash_authority() {
    let manifest = manifest();
    let settlement = SettlementBatch::from_manifest(&manifest)
        .expect("reward manifest should produce settlement batch");
    let store = svc_rewarder::outputs::IntentStore::default();

    assert_eq!(store.emit_batch_once(&settlement, true).as_str(), "dry_run");
    assert_eq!(
        store.emit_batch_once(&settlement, false).as_str(),
        "accepted"
    );
    assert_eq!(store.emit_batch_once(&settlement, false).as_str(), "dup");

    let encoded = serde_json::to_string(&settlement).expect("settlement serializes");

    for forbidden in [
        "bond_account_id",
        "slash_evidence",
        "automatic_slash",
        "validator_reward_receipt",
        "staking_power",
        "public_staking_market",
        "liquidity_pool",
        "bridge_settlement",
        "external_settlement",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "rewarder replay/dedupe seam must not become Phase 4 bond authority: {forbidden}"
        );
    }
}

#[test]
fn rewarder_source_does_not_implement_phase4_bond_runtime_authority() {
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
        "bridge_settlement",
        "external_settlement",
        "solana",
        "rox",
        "ron_ledger::",
    ] {
        assert!(
            !source.contains(forbidden),
            "svc-rewarder source must not implement Phase 4 bond runtime authority via `{forbidden}`"
        );
    }
}
