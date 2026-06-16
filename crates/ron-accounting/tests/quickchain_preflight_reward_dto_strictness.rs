//! RO:WHAT — QuickChain Phase-0 strict DTO tests for ron-accounting reward artifacts.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/GOV. Reward planning artifacts must reject authority drift.
//! RO:INTERACTS — RewardSnapshotExport, RewardContributionExport, ProjectedRewardSnapshot.
//! RO:INVARIANTS — reward snapshots are planning inputs only; no roots, finality, validators, or mutation authority.
//! RO:METRICS — none.
//! RO:CONFIG — RewardProjectionConfig only.
//! RO:SECURITY — rejects unknown economic-authority fields at snapshot/projection boundaries.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_reward_dto_strictness.

use ron_accounting::{
    canonical_snapshot_bytes, canonical_snapshot_cid, ProjectedRewardSnapshot,
    RewardContributionExport, RewardProjectionConfig, RewardProjectionReport, RewardSnapshotExport,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn contribution(account: &str) -> RewardContributionExport {
    RewardContributionExport::new(account, 100, 50, 10)
}

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1,
        "1000",
        vec![contribution("acct_b"), contribution("acct_a")],
    )
    .expect("sample snapshot")
}

fn sample_projected_snapshot() -> ProjectedRewardSnapshot {
    let snapshot = sample_snapshot();
    let snapshot_cid = canonical_snapshot_cid(&snapshot).expect("snapshot cid");

    ProjectedRewardSnapshot {
        snapshot,
        snapshot_cid,
        report: RewardProjectionReport {
            input_slices: 1,
            input_rows: 3,
            projected_accounts: 2,
            bytes_stored: 200,
            bytes_served: 100,
            uptime_seconds: 20,
            ignored_rows: 1,
        },
    }
}

fn with_unknown_field(mut value: Value, field: &str) -> Value {
    value
        .as_object_mut()
        .expect("test value must be a JSON object")
        .insert(field.to_string(), json!("client-smuggled-authority"));
    value
}

fn assert_rejects_unknown<T>(value: Value, field: &str)
where
    T: DeserializeOwned,
{
    let poisoned = with_unknown_field(value, field);

    assert!(
        serde_json::from_value::<T>(poisoned).is_err(),
        "DTO must reject unknown authority-looking field: {field}"
    );
}

#[test]
fn reward_snapshot_export_rejects_unknown_authority_fields() {
    for field in [
        "balance",
        "receipt_hash",
        "operation_id",
        "account_sequence",
        "settlement_status",
        "finality",
        "finalized",
        "state_root",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "checkpoint_root",
        "validator",
        "anchor",
        "bridge",
        "staking",
        "liquidity",
        "payout_authorized",
        "ledger_mutation",
        "wallet_mutation",
    ] {
        let value = serde_json::to_value(sample_snapshot()).expect("snapshot JSON");
        assert_rejects_unknown::<RewardSnapshotExport>(value, field);
    }
}

#[test]
fn reward_contribution_export_rejects_unknown_authority_fields() {
    for field in [
        "balance_minor",
        "receipt",
        "hold_id",
        "mint",
        "transfer",
        "capture",
        "release",
        "operation_id",
        "reward_root",
        "payout_authorized",
    ] {
        let value = serde_json::to_value(contribution("acct_a")).expect("contribution JSON");
        assert_rejects_unknown::<RewardContributionExport>(value, field);
    }
}

#[test]
fn nested_reward_contribution_rejects_unknown_authority_fields() {
    let mut value = serde_json::to_value(sample_snapshot()).expect("snapshot JSON");

    value
        .get_mut("contributions")
        .and_then(Value::as_array_mut)
        .and_then(|items| items.get_mut(0))
        .and_then(Value::as_object_mut)
        .expect("first contribution object")
        .insert(
            "operation_id".to_string(),
            json!("client-supplied-ledger-operation"),
        );

    assert!(
        serde_json::from_value::<RewardSnapshotExport>(value).is_err(),
        "nested contribution must reject client-supplied operation identity"
    );
}

#[test]
fn reward_projection_config_rejects_unknown_authority_fields() {
    for field in [
        "validator",
        "checkpoint_root",
        "settlement_status",
        "payout_authorized",
        "wallet_mutation",
    ] {
        let value = serde_json::to_value(RewardProjectionConfig::new("1000")).expect("config JSON");
        assert_rejects_unknown::<RewardProjectionConfig>(value, field);
    }
}

#[test]
fn reward_projection_report_rejects_unknown_authority_fields() {
    for field in [
        "balance",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "finalized",
        "ledger_mutation",
    ] {
        let value = serde_json::to_value(RewardProjectionReport {
            input_slices: 1,
            input_rows: 2,
            projected_accounts: 1,
            bytes_stored: 100,
            bytes_served: 0,
            uptime_seconds: 10,
            ignored_rows: 1,
        })
        .expect("report JSON");

        assert_rejects_unknown::<RewardProjectionReport>(value, field);
    }
}

#[test]
fn projected_reward_snapshot_rejects_unknown_top_level_authority_fields() {
    for field in [
        "state_root",
        "reward_root",
        "checkpoint",
        "checkpoint_root",
        "finality",
        "validator",
        "anchor",
        "bridge",
        "settlement_status",
        "payout_authorized",
    ] {
        let value =
            serde_json::to_value(sample_projected_snapshot()).expect("projected snapshot JSON");
        assert_rejects_unknown::<ProjectedRewardSnapshot>(value, field);
    }
}

#[test]
fn projected_reward_snapshot_rejects_unknown_nested_snapshot_authority_fields() {
    let mut value =
        serde_json::to_value(sample_projected_snapshot()).expect("projected snapshot JSON");

    value
        .get_mut("snapshot")
        .and_then(Value::as_object_mut)
        .expect("nested snapshot object")
        .insert("reward_root".to_string(), json!("client-smuggled-root"));

    assert!(
        serde_json::from_value::<ProjectedRewardSnapshot>(value).is_err(),
        "nested snapshot must reject root-looking authority drift"
    );
}

#[test]
fn strict_reward_dtos_still_roundtrip_valid_current_shapes() {
    let snapshot = sample_snapshot();
    let snapshot_json = serde_json::to_value(&snapshot).expect("snapshot JSON");

    let decoded_snapshot: RewardSnapshotExport =
        serde_json::from_value(snapshot_json).expect("strict snapshot should still roundtrip");

    assert_eq!(
        canonical_snapshot_bytes(&snapshot).expect("original bytes"),
        canonical_snapshot_bytes(&decoded_snapshot).expect("decoded bytes")
    );

    let config = RewardProjectionConfig::new("1000");
    let config_json = serde_json::to_value(&config).expect("config JSON");
    let decoded_config: RewardProjectionConfig =
        serde_json::from_value(config_json).expect("strict config should still roundtrip");

    assert_eq!(decoded_config, config);

    let projected = sample_projected_snapshot();
    let projected_json = serde_json::to_value(&projected).expect("projected JSON");
    let decoded_projected: ProjectedRewardSnapshot =
        serde_json::from_value(projected_json).expect("strict projection should still roundtrip");

    assert_eq!(decoded_projected, projected);
}
