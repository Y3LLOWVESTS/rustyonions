#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle boundary tests for ron-accounting.
//! RO:WHY — Accounting may produce derivative snapshots and reward-planning artifacts, but validator rotation, revocation, equivocation, replay challenges, downtime, and governance parameter updates must not become balance, wallet, ledger, payout, finality, staking, slashing, or bridge authority.
//! RO:INTERACTS — RewardSnapshotExport, ProjectedRewardSnapshot, source/Cargo boundary.
//! RO:INVARIANTS — accounting remains derivative metering/snapshot infrastructure; no validator lifecycle decision can mutate balances, produce receipts, unlock content, or execute payouts.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects Phase 3 Round 2 lifecycle authority smuggling and preserves accounting non-authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase3_validator_lifecycle_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    ProjectedRewardSnapshot, RewardContributionExport, RewardProjectionReport, RewardSnapshotExport,
};
use serde_json::{json, Map, Value};

const PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS: &[&str] = &[
    "validator_rotation",
    "validator_rotation_epoch",
    "validator_rotation_decision",
    "validator_revocation",
    "validator_revocation_reason",
    "validator_revocation_decision",
    "validator_lifecycle_decision",
    "validator_lifecycle_status",
    "validator_lifecycle_rejection_code",
    "equivocation_evidence",
    "double_attestation_evidence",
    "split_brain_evidence",
    "replay_challenge_evidence",
    "invalid_attestation_evidence",
    "validator_downtime_status",
    "validator_degraded_status",
    "downtime_report",
    "governance_parameter_update",
    "validator_set_parameter_update",
    "quorum_parameter_update",
    "checkpoint_parameter_update",
    "slash_evidence",
    "slashing",
    "staking_power",
    "validator_bond",
    "bonded_economics",
    "validator_reward",
    "bridge_settlement",
    "external_settlement",
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

fn assert_no_key(value: &Value, forbidden_key: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(forbidden_key),
                "accounting JSON value must not contain lifecycle authority key `{forbidden_key}`: {value}"
            );

            for nested in map.values() {
                assert_no_key(nested, forbidden_key);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key(nested, forbidden_key);
            }
        }
        _ => {}
    }
}

fn reward_snapshot_body_with_top_level(extra_key: &str) -> Value {
    let mut contribution = Map::new();
    contribution.insert("account".to_owned(), json!("acct_a"));
    contribution.insert("bytes_stored".to_owned(), json!(100_u64));
    contribution.insert("bytes_served".to_owned(), json!(50_u64));
    contribution.insert("uptime_seconds".to_owned(), json!(10_u64));

    let mut body = Map::new();
    body.insert("produced_at_millis".to_owned(), json!(1_u64));
    body.insert("pool_minor_units".to_owned(), json!("1000"));
    body.insert(
        "contributions".to_owned(),
        Value::Array(vec![Value::Object(contribution)]),
    );
    body.insert(extra_key.to_owned(), json!("client-must-not-supply"));

    Value::Object(body)
}

fn reward_snapshot_body_with_contribution(extra_key: &str) -> Value {
    let mut contribution = Map::new();
    contribution.insert("account".to_owned(), json!("acct_a"));
    contribution.insert("bytes_stored".to_owned(), json!(100_u64));
    contribution.insert("bytes_served".to_owned(), json!(50_u64));
    contribution.insert("uptime_seconds".to_owned(), json!(10_u64));
    contribution.insert(extra_key.to_owned(), json!("client-must-not-supply"));

    json!({
        "produced_at_millis": 1_u64,
        "pool_minor_units": "1000",
        "contributions": [Value::Object(contribution)]
    })
}

fn reward_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1,
        "1000",
        vec![
            RewardContributionExport::new("acct_b", 200, 0, 20),
            RewardContributionExport::new("acct_a", 100, 50, 10),
        ],
    )
    .expect("valid reward snapshot should canonicalize")
}

#[test]
fn accounting_reward_snapshot_rejects_validator_lifecycle_authority_fields() {
    for extra_key in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert!(
            serde_json::from_value::<RewardSnapshotExport>(
                reward_snapshot_body_with_top_level(extra_key)
            )
            .is_err(),
            "RewardSnapshotExport must reject top-level Phase 3 Round 2 lifecycle authority field: {extra_key}"
        );

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(
                reward_snapshot_body_with_contribution(extra_key)
            )
            .is_err(),
            "RewardSnapshotExport contribution rows must reject Phase 3 Round 2 lifecycle authority field: {extra_key}"
        );
    }
}

#[test]
fn canonical_accounting_snapshot_bytes_do_not_contain_lifecycle_authority_vocabulary() {
    let snapshot = reward_snapshot();
    let canonical_bytes = snapshot
        .canonical_bytes()
        .expect("snapshot canonical bytes should compute");
    let canonical_json =
        String::from_utf8(canonical_bytes).expect("canonical snapshot bytes should be UTF-8 JSON");

    for forbidden in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert!(
            !canonical_json.contains(forbidden),
            "accounting canonical bytes must not contain Phase 3 Round 2 lifecycle authority vocabulary: {forbidden}"
        );
    }

    for forbidden in [
        "wallet_receipt",
        "balance_minor",
        "available_balance",
        "settlement_status",
        "finality",
        "checkpoint_hash",
        "state_root",
        "receipt_root",
        "payout_executed",
        "paid_unlock",
    ] {
        assert!(
            !canonical_json.contains(forbidden),
            "accounting canonical bytes must remain derivative artifact data, not economic authority: {forbidden}"
        );
    }
}

#[test]
fn projected_reward_snapshot_is_not_a_lifecycle_decision_or_payout_execution() {
    let snapshot = reward_snapshot();
    let snapshot_cid = snapshot
        .canonical_cid()
        .expect("snapshot CID should compute");
    let projected = ProjectedRewardSnapshot {
        snapshot,
        snapshot_cid,
        report: RewardProjectionReport {
            input_slices: 1,
            input_rows: 2,
            projected_accounts: 2,
            bytes_stored: 300,
            bytes_served: 50,
            uptime_seconds: 30,
            ignored_rows: 0,
        },
    };

    let projected_json =
        serde_json::to_value(&projected).expect("projected reward snapshot should serialize");

    for forbidden_key in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert_no_key(&projected_json, forbidden_key);
    }

    for forbidden_key in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "anchored",
    ] {
        assert_no_key(&projected_json, forbidden_key);
    }
}

#[test]
fn accounting_source_does_not_construct_validator_lifecycle_authority() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden_dependency in [
        "ron-proto",
        "ron_proto",
        "ron-ledger",
        "ron_ledger",
        "svc-wallet",
        "svc_wallet",
    ] {
        assert!(
            !manifest.contains(forbidden_dependency),
            "ron-accounting must not gain validator/wallet/ledger authority dependency: {forbidden_dependency}"
        );
    }

    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "rotate_validator",
            "revoke_validator",
            "validator_lifecycle_decision",
            "validator_lifecycle_status",
            "equivocation_evidence",
            "double_attestation_evidence",
            "split_brain_evidence",
            "replay_challenge_evidence",
            "downtime_report",
            "validator_degraded_status",
            "governance_parameter_update",
            "slash_validator",
            "stake_validator",
            "validator_reward",
            "wallet_mutation",
            "ledger_mutation",
            "issue_from_attestation",
            "settle_from_validator",
            "payout_from_lifecycle",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting production source must not construct Phase 3 Round 2 validator lifecycle authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
