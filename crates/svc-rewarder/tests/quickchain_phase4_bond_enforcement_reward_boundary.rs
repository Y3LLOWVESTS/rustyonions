#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 3 controlled bond-enforcement reward boundary tests.
//! RO:WHY — Controlled internal bond enforcement must not become rewarder payout,
//! penalty, wallet, ledger, staking, liquidity, bridge, or public-market authority.
//! RO:INTERACTS — ComputeEpochRequest, AccountingSnapshot, RewardPolicy,
//! RewardManifest, SettlementBatch, WalletIssueBatch, source boundary.
//! RO:INVARIANTS — svc-rewarder remains deterministic payout planning only;
//! controlled enforcement cannot mint, slash, capture, release, or reward by itself.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects Phase 4 Round 3 enforcement authority smuggling.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase4_bond_enforcement_reward_boundary.

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
        plan_settlement_intents, IntentResult, IntentStore, RewardManifest, SettlementBatch,
    },
};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const INPUTS_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS: &[&str] = &[
    "enforcement_id",
    "enforcement_status",
    "enforcement_decision",
    "enforcement_action",
    "bond_enforcement_decision",
    "bond_enforcement_confirmation",
    "bond_enforcement_receipt",
    "bond_account_id",
    "bond_account_status",
    "bond_operator_subject",
    "slash_reserved_minor",
    "slash_reserve_minor",
    "reserve_slash",
    "release_slash_reserve",
    "capture_slash_reserve",
    "slash_capture",
    "slash_release",
    "slash_reserve",
    "controlled_bond_enforcement",
    "apply_controlled_bond_enforcement",
    "execute_bond_enforcement",
    "commit_bond_enforcement",
    "policy_gated_slash",
    "governance_approval_ref",
    "operator_approval_ref",
    "wallet_receipt",
    "ledger_receipt",
    "wallet_mutation",
    "ledger_mutation",
    "balance_side_effect",
    "payout_executed",
    "payout_execution",
    "mint_from_enforcement",
    "reward_from_enforcement",
    "validator_reward",
    "validator_reward_receipt",
    "staking_power",
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

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_alpha".to_string(),
                bytes_stored: 1_000,
                bytes_served: 500,
                uptime_seconds: 60,
            },
            AccountContribution {
                account: "acct_beta".to_string(),
                bytes_stored: 500,
                bytes_served: 250,
                uptime_seconds: 60,
            },
        ],
    }
}

fn policy() -> RewardPolicy {
    RewardPolicy {
        id: POLICY_ID.to_string(),
        hash: POLICY_HASH.to_string(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_string(),
    }
}

fn manifest() -> RewardManifest {
    compute_manifest(
        ComputeInput {
            epoch_id: "epoch-phase4-round3".to_string(),
            inputs_cid: ContentCid::parse(INPUTS_CID).expect("valid cid"),
            policy: policy(),
            snapshot: snapshot(),
            dry_run: true,
            idempotency_salt: "svc-rewarder|phase4-round3".to_string(),
        },
        IntentResult::DryRun,
    )
    .expect("reward manifest should compute")
}

fn settlement_batch() -> SettlementBatch {
    plan_settlement_intents(&manifest()).expect("settlement planning should succeed")
}

fn compute_request_value() -> Value {
    serde_json::to_value(ComputeEpochRequest {
        inputs_cid: INPUTS_CID.to_string(),
        policy_id: POLICY_ID.to_string(),
        policy_hash: POLICY_HASH.to_string(),
        dry_run: true,
        notes: Some("phase4 round3 rewarder remains planning only".to_string()),
        snapshot: Some(snapshot()),
        policy: Some(policy()),
    })
    .expect("request should serialize")
}

fn assert_no_key_recursive(value: &Value, forbidden: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(forbidden),
                "JSON object must not expose forbidden authority key `{forbidden}`: {value}"
            );
            for child in map.values() {
                assert_no_key_recursive(child, forbidden);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_no_key_recursive(child, forbidden);
            }
        }
        _ => {}
    }
}

#[test]
fn rewarder_compute_request_rejects_round3_enforcement_authority_fields() {
    serde_json::from_value::<ComputeEpochRequest>(compute_request_value())
        .expect("clean compute request should deserialize");

    for field in PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS {
        let mut top_level = compute_request_value();
        top_level.as_object_mut().expect("request object").insert(
            (*field).to_string(),
            json!("client-supplied-enforcement-authority"),
        );

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(top_level).is_err(),
            "ComputeEpochRequest must reject top-level Phase 4 Round 3 authority field: {field}"
        );

        let mut nested_policy = compute_request_value();
        nested_policy
            .get_mut("policy")
            .and_then(Value::as_object_mut)
            .expect("policy object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-enforcement-authority"),
            );

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(nested_policy).is_err(),
            "RewardPolicy must reject nested Phase 4 Round 3 authority field: {field}"
        );

        let mut nested_snapshot = compute_request_value();
        nested_snapshot
            .get_mut("snapshot")
            .and_then(Value::as_object_mut)
            .expect("snapshot object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-enforcement-authority"),
            );

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(nested_snapshot).is_err(),
            "AccountingSnapshot must reject nested Phase 4 Round 3 authority field: {field}"
        );
    }
}

#[test]
fn rewarder_manifest_and_wallet_issue_batch_remain_planning_not_enforcement_truth() {
    let manifest = manifest();
    let settlement = plan_settlement_intents(&manifest).expect("settlement plan");
    let wallet_batch = settlement.to_wallet_issue_batch();

    let manifest_json = serde_json::to_value(&manifest).expect("manifest serializes");
    let settlement_json = serde_json::to_value(&settlement).expect("settlement serializes");
    let wallet_batch_json = serde_json::to_value(&wallet_batch).expect("wallet batch serializes");

    assert_eq!(wallet_batch.wallet_path, "/v1/issue");
    assert_eq!(
        wallet_batch.funding_source,
        RewardFundingSource::ProtocolPool
    );
    assert!(
        !wallet_batch.requests.is_empty(),
        "reward plan should contain wallet issue previews"
    );

    for request in &wallet_batch.requests {
        assert_eq!(request.asset, "roc");
        assert!(
            request.amount_minor.parse::<u128>().is_ok(),
            "wallet issue amount must remain an integer minor-unit string"
        );
        assert!(
            request
                .idempotency_key
                .as_deref()
                .is_some_and(|key| key.starts_with("b3:")),
            "wallet handoff remains idempotency-key based, not enforcement-operation authority"
        );
    }

    for forbidden in PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS {
        assert_no_key_recursive(&manifest_json, forbidden);
        assert_no_key_recursive(&settlement_json, forbidden);
        assert_no_key_recursive(&wallet_batch_json, forbidden);
    }
}

#[test]
fn bond_enforcement_like_replay_is_still_rewarder_dedupe_not_second_payout_or_slash() {
    let settlement = settlement_batch();
    let store = IntentStore::default();

    assert_eq!(
        store.emit_batch_once(&settlement, true),
        IntentResult::DryRun
    );
    assert_eq!(
        store.emit_batch_once(&settlement, false),
        IntentResult::Accepted
    );
    assert_eq!(store.emit_batch_once(&settlement, false), IntentResult::Dup);

    let encoded = serde_json::to_string(&settlement).expect("settlement serializes");

    for forbidden in PHASE4_ROUND3_ENFORCEMENT_AUTHORITY_KEYS {
        assert!(
            !encoded.contains(forbidden),
            "rewarder replay/dedupe seam must not become Phase 4 Round 3 enforcement authority: {forbidden}"
        );
    }
}

#[test]
fn rewarder_source_does_not_implement_phase4_round3_bond_enforcement_authority() {
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
    let lower = source.to_ascii_lowercase();

    for forbidden in [
        "quickchainbondenforcement",
        "bond_enforcement_decision",
        "bond_enforcement_confirmation",
        "bond_enforcement_route",
        "controlled_bond_enforcement",
        "apply_controlled_bond_enforcement",
        "execute_bond_enforcement",
        "commit_bond_enforcement",
        "reserve_slash",
        "release_slash_reserve",
        "capture_slash_reserve",
        "slash_reserved",
        "slash_capture",
        "slash_release",
        "policy_gated_slash",
        "governance_approval_ref",
        "operator_approval_ref",
        "mint_from_enforcement",
        "reward_from_enforcement",
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
            !lower.contains(forbidden),
            "svc-rewarder source must not implement Phase 4 Round 3 bond enforcement authority via `{forbidden}`"
        );
    }
}
