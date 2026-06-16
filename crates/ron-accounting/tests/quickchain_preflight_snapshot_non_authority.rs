//! RO:WHAT — QuickChain Phase-0 snapshot non-authority tests for ron-accounting.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Proves reward snapshots are planning artifacts only.
//! RO:INTERACTS — RewardSnapshotExport, RewardContributionExport, canonical snapshot bytes/CID.
//! RO:INVARIANTS — snapshot CID is not a root; no balance, receipt, finality, or mutation fields.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no fake balances, fake receipts, roots, validators, anchors, or payout authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_snapshot_non_authority.

use ron_accounting::{
    canonical_snapshot_bytes, canonical_snapshot_cid, RewardContributionExport,
    RewardSnapshotExport,
};
use serde_json::Value;

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1,
        "1000",
        vec![
            RewardContributionExport::new("acct_b", 200, 0, 20),
            RewardContributionExport::new("acct_a", 100, 50, 10),
        ],
    )
    .expect("sample snapshot")
}

fn assert_b3(value: &str) {
    assert_eq!(value.len(), 67, "expected b3:<64 lowercase hex>");
    assert!(value.starts_with("b3:"), "expected b3 prefix");
    assert!(
        value.as_bytes()[3..]
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "expected lowercase hex only"
    );
}

fn assert_no_forbidden_keys(value: &Value) {
    const FORBIDDEN_KEYS: &[&str] = &[
        "balance",
        "balance_minor",
        "available_balance",
        "spendable_balance",
        "receipt",
        "receipt_hash",
        "operation_id",
        "account_sequence",
        "hold_id",
        "issue",
        "burn",
        "transfer",
        "hold",
        "capture",
        "release",
        "mint",
        "settlement_status",
        "finality",
        "finalized",
        "state_root",
        "receipt_root",
        "accounting_root",
        "reward_root",
        "checkpoint",
        "checkpoint_root",
        "validator",
        "anchor",
        "bridge",
        "staking",
        "liquidity",
        "payout_authorized",
        "ledger_mutation",
        "wallet_mutation",
    ];

    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let normalized = key.to_ascii_lowercase();

                assert!(
                    !FORBIDDEN_KEYS.contains(&normalized.as_str()),
                    "forbidden authority key leaked into snapshot: {key}"
                );
                assert!(
                    !normalized.ends_with("_root"),
                    "root-like key leaked into snapshot: {key}"
                );

                assert_no_forbidden_keys(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_forbidden_keys(item);
            }
        }
        _ => {}
    }
}

#[test]
fn reward_snapshot_json_shape_is_contribution_artifact_only() {
    let snapshot = sample_snapshot();
    let value = serde_json::to_value(&snapshot).expect("snapshot JSON");
    let object = value.as_object().expect("snapshot object");

    assert_eq!(object.len(), 3);
    assert!(object.contains_key("produced_at_millis"));
    assert!(object.contains_key("pool_minor_units"));
    assert!(object.contains_key("contributions"));

    assert_no_forbidden_keys(&value);
}

#[test]
fn pool_minor_units_is_integer_string_not_float_or_json_number() {
    let snapshot = sample_snapshot();
    let value = serde_json::to_value(&snapshot).expect("snapshot JSON");

    assert_eq!(
        value.get("pool_minor_units"),
        Some(&Value::String("1000".to_string())),
        "pool_minor_units must remain an integer minor-unit string"
    );

    assert!(
        RewardSnapshotExport::new(
            1,
            "10.5",
            vec![RewardContributionExport::new("acct_a", 1, 0, 0)]
        )
        .is_err(),
        "float-looking pools must reject"
    );

    assert!(
        RewardSnapshotExport::new(
            1,
            "",
            vec![RewardContributionExport::new("acct_a", 1, 0, 0)]
        )
        .is_err(),
        "empty pool strings must reject"
    );
}

#[test]
fn canonical_snapshot_cid_is_artifact_hash_not_root_field() {
    let snapshot = sample_snapshot();

    let cid = canonical_snapshot_cid(&snapshot).expect("snapshot CID");
    assert_b3(&cid);

    let value = serde_json::to_value(&snapshot).expect("snapshot JSON");
    let object = value.as_object().expect("snapshot object");

    assert!(
        !object.contains_key("snapshot_cid"),
        "CID may be returned beside the artifact, but must not become embedded authority"
    );
    assert!(
        !object.keys().any(|key| key.ends_with("_root")),
        "snapshot export must not expose root fields"
    );
}

#[test]
fn canonical_snapshot_bytes_do_not_contain_finality_or_mutation_vocabulary() {
    let snapshot = sample_snapshot();
    let bytes = canonical_snapshot_bytes(&snapshot).expect("canonical bytes");
    let canonical_json = String::from_utf8(bytes).expect("canonical JSON UTF-8");

    for forbidden in [
        "root",
        "finality",
        "finalized",
        "settlement",
        "validator",
        "checkpoint",
        "anchor",
        "bridge",
        "operation_id",
        "account_sequence",
        "receipt",
        "balance",
        "wallet_mutation",
        "ledger_mutation",
        "payout_authorized",
    ] {
        assert!(
            !canonical_json.contains(forbidden),
            "canonical snapshot bytes must not contain forbidden authority vocabulary: {forbidden}"
        );
    }
}

#[test]
fn duplicate_accounts_and_invalid_account_strings_reject() {
    let duplicate = RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "1000".to_string(),
        contributions: vec![
            RewardContributionExport::new("acct_a", 1, 0, 0),
            RewardContributionExport::new(" acct_a ", 2, 0, 0),
        ],
    };

    assert!(
        duplicate.canonicalized().is_err(),
        "duplicate accounts must reject after canonical trimming"
    );

    let invalid_account = RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "1000".to_string(),
        contributions: vec![RewardContributionExport::new("acct a", 1, 0, 0)],
    };

    assert!(
        invalid_account.validate().is_err(),
        "account strings must stay bounded and canonical"
    );
}
