//! RO:WHAT — Unit tests for the stable accounting↔rewarder interop vector.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Locks the snapshot shape used by svc-rewarder.
//! RO:INTERACTS — accounting::interop, reward_snapshot canonicalization.
//! RO:INVARIANTS — canonical JSON is stable; CID matches canonical bytes; score/count match.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — vector uses synthetic accounts only.
//! RO:TEST — cargo test -p ron-accounting --test unit.

use ron_accounting::{
    canonical_json_for_snapshot, reward_snapshot_interop_vector_v1,
    REWARD_SNAPSHOT_VECTOR_EPOCH_ID, REWARD_SNAPSHOT_VECTOR_SCHEMA,
};

#[test]
fn reward_snapshot_interop_vector_validates() {
    let vector = reward_snapshot_interop_vector_v1().expect("interop vector");

    vector.validate().expect("vector validates");
    assert_eq!(vector.schema, REWARD_SNAPSHOT_VECTOR_SCHEMA);
    assert_eq!(vector.epoch_id, REWARD_SNAPSHOT_VECTOR_EPOCH_ID);
    assert_eq!(vector.expected_contribution_count, 2);
    assert_eq!(vector.expected_total_score, 342);
    assert_eq!(vector.usage_events.len(), 5);
}

#[test]
fn reward_snapshot_interop_vector_has_stable_canonical_json() {
    let vector = reward_snapshot_interop_vector_v1().expect("interop vector");

    assert_eq!(
        vector.canonical_snapshot_json,
        r#"{"produced_at_millis":1,"pool_minor_units":"1000","contributions":[{"account":"acct_a","bytes_stored":100,"bytes_served":50,"uptime_seconds":10},{"account":"acct_b","bytes_stored":200,"bytes_served":0,"uptime_seconds":20}]}"#
    );

    assert_eq!(
        canonical_json_for_snapshot(&vector.snapshot).expect("canonical json"),
        vector.canonical_snapshot_json
    );
}

#[test]
fn reward_snapshot_interop_vector_cid_matches_snapshot_bytes() {
    let vector = reward_snapshot_interop_vector_v1().expect("interop vector");

    assert!(vector.snapshot_cid.starts_with("b3:"));
    assert_eq!(vector.snapshot_cid.len(), 67);
    assert_eq!(
        vector.snapshot.canonical_cid().expect("snapshot cid"),
        vector.snapshot_cid
    );
}

#[test]
fn reward_snapshot_interop_vector_snapshot_is_rewarder_shape() {
    let vector = reward_snapshot_interop_vector_v1().expect("interop vector");
    let snapshot = vector.snapshot;

    assert_eq!(snapshot.produced_at_millis, 1);
    assert_eq!(snapshot.pool_minor_units, "1000");
    assert_eq!(snapshot.contributions.len(), 2);

    assert_eq!(snapshot.contributions[0].account, "acct_a");
    assert_eq!(snapshot.contributions[0].bytes_stored, 100);
    assert_eq!(snapshot.contributions[0].bytes_served, 50);
    assert_eq!(snapshot.contributions[0].uptime_seconds, 10);

    assert_eq!(snapshot.contributions[1].account, "acct_b");
    assert_eq!(snapshot.contributions[1].bytes_stored, 200);
    assert_eq!(snapshot.contributions[1].bytes_served, 0);
    assert_eq!(snapshot.contributions[1].uptime_seconds, 20);
}
