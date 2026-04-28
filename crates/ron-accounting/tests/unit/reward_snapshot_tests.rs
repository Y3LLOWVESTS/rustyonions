//! RO:WHAT — Unit tests for reward snapshot canonicalization and CID generation.
//! RO:WHY — Pillar 12; Concerns: ECON/DX. Snapshot export must match svc-rewarder expectations.
//! RO:INTERACTS — accounting::reward_snapshot, utils::hashing, serde_json.
//! RO:INVARIANTS — deterministic bytes; b3 CID; duplicate accounts rejected; integer-only pool.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — account strings bounded and charset-checked.
//! RO:TEST — cargo test -p ron-accounting --test unit.

use ron_accounting::{canonical_snapshot_cid, RewardContributionExport, RewardSnapshotExport};

fn sample_snapshot_reversed() -> RewardSnapshotExport {
    RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "1000".to_string(),
        contributions: vec![
            RewardContributionExport::new("acct_b", 200, 0, 20),
            RewardContributionExport::new("acct_a", 100, 50, 10),
        ],
    }
}

#[test]
fn reward_snapshot_canonicalizes_account_order() {
    let snapshot = sample_snapshot_reversed();
    let canonical = snapshot.canonicalized().expect("canonical snapshot");

    assert_eq!(canonical.contributions.len(), 2);
    assert_eq!(canonical.contributions[0].account, "acct_a");
    assert_eq!(canonical.contributions[1].account, "acct_b");
}

#[test]
fn reward_snapshot_canonical_bytes_match_expected_shape() {
    let snapshot = sample_snapshot_reversed();
    let bytes = snapshot.canonical_bytes().expect("canonical bytes");
    let json = String::from_utf8(bytes).expect("utf8 json");

    assert_eq!(
        json,
        r#"{"produced_at_millis":1,"pool_minor_units":"1000","contributions":[{"account":"acct_a","bytes_stored":100,"bytes_served":50,"uptime_seconds":10},{"account":"acct_b","bytes_stored":200,"bytes_served":0,"uptime_seconds":20}]}"#
    );
}

#[test]
fn reward_snapshot_cid_is_stable_after_canonicalization() {
    let unsorted = sample_snapshot_reversed();
    let sorted = RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "1000".to_string(),
        contributions: vec![
            RewardContributionExport::new("acct_a", 100, 50, 10),
            RewardContributionExport::new("acct_b", 200, 0, 20),
        ],
    };

    let left = canonical_snapshot_cid(&unsorted).expect("left cid");
    let right = canonical_snapshot_cid(&sorted).expect("right cid");

    assert_eq!(left, right);
    assert!(left.starts_with("b3:"));
    assert_eq!(left.len(), 67);
}

#[test]
fn reward_snapshot_rejects_duplicate_accounts_after_trim() {
    let snapshot = RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "1000".to_string(),
        contributions: vec![
            RewardContributionExport::new("acct_a", 1, 0, 0),
            RewardContributionExport::new(" acct_a ", 2, 0, 0),
        ],
    };

    assert!(snapshot.canonicalized().is_err());
}

#[test]
fn reward_snapshot_rejects_non_integer_pool() {
    let snapshot = RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "10.5".to_string(),
        contributions: vec![RewardContributionExport::new("acct_a", 1, 0, 0)],
    };

    assert!(snapshot.validate().is_err());
}

#[test]
fn reward_snapshot_total_score_is_checked() {
    let snapshot = RewardSnapshotExport::new(
        1,
        "1000",
        vec![
            RewardContributionExport::new("acct_a", 100, 40, 10),
            RewardContributionExport::new("acct_b", 200, 0, 20),
        ],
    )
    .expect("snapshot");

    assert_eq!(snapshot.total_score().expect("score"), 340);
}

#[test]
fn reward_contribution_rejects_invalid_account_chars() {
    let snapshot = RewardSnapshotExport {
        produced_at_millis: 1,
        pool_minor_units: "1000".to_string(),
        contributions: vec![RewardContributionExport::new("acct a", 1, 0, 0)],
    };

    assert!(snapshot.validate().is_err());
}
