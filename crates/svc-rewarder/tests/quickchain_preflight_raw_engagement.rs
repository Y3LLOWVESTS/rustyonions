//! RO:WHAT — QuickChain Phase-0 raw-engagement rejection tests for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Bot-prone engagement must not become protocol ROC authority.
//! RO:INTERACTS — inputs::AccountingSnapshot, inputs::RewardPolicy, http::dto::ComputeEpochRequest.
//! RO:INVARIANTS — raw views/likes/clicks/watch seconds are not payout-basis fields.
//! RO:METRICS — none; pure DTO drift tests.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects fakeable engagement formula smuggling through strict DTOs.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_raw_engagement.

use serde_json::{json, Value};
use svc_rewarder::http::dto::ComputeEpochRequest;
use svc_rewarder::inputs::{canonical_snapshot_cid, AccountingSnapshot, RewardPolicy};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn valid_snapshot_value() -> Value {
    json!({
        "produced_at_millis": 1,
        "pool_minor_units": "1000",
        "contributions": [
            {
                "account": "acct_a",
                "bytes_stored": 100,
                "bytes_served": 40,
                "uptime_seconds": 10
            }
        ]
    })
}

fn valid_inputs_cid(snapshot: &Value) -> String {
    let parsed = serde_json::from_value::<AccountingSnapshot>(snapshot.clone())
        .expect("valid snapshot parses");
    canonical_snapshot_cid(parsed).expect("snapshot cid")
}

fn valid_compute_body() -> Value {
    let snapshot = valid_snapshot_value();
    let inputs_cid = valid_inputs_cid(&snapshot);

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": snapshot,
        "policy": {
            "id": POLICY_ID,
            "hash": POLICY_HASH,
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units": "1000",
            "min_payout_minor_units": "1",
            "weight_bps": 10000,
            "rounding": "floor"
        }
    })
}

#[test]
fn accounting_snapshot_rejects_raw_engagement_contribution_fields() {
    for raw_field in [
        "raw_views",
        "raw_likes",
        "raw_comments",
        "raw_impressions",
        "raw_watch_seconds",
        "raw_clicks",
        "raw_active_users",
    ] {
        let snapshot = json!({
            "produced_at_millis": 1,
            "pool_minor_units": "1000",
            "contributions": [
                {
                    "account": "acct_a",
                    "bytes_stored": 100,
                    "bytes_served": 40,
                    "uptime_seconds": 10,
                    raw_field: 999
                }
            ]
        });

        let err = serde_json::from_value::<AccountingSnapshot>(snapshot)
            .expect_err("raw engagement field must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "{raw_field} should fail as unknown, got {err}"
        );
    }
}

#[test]
fn compute_request_rejects_top_level_raw_engagement_payout_fields() {
    for raw_field in [
        "raw_views",
        "raw_likes",
        "raw_comments",
        "raw_impressions",
        "raw_watch_seconds",
        "raw_clicks",
        "raw_active_users",
        "engagement_reward_minor_units",
        "mint_from_views",
    ] {
        let mut body = valid_compute_body();
        body[raw_field] = json!(777);

        let err = serde_json::from_value::<ComputeEpochRequest>(body)
            .expect_err("top-level raw engagement authority must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "{raw_field} should fail as unknown, got {err}"
        );
    }
}

#[test]
fn reward_policy_rejects_raw_engagement_formula_fields() {
    for raw_field in [
        "reward_formula",
        "raw_engagement_weight",
        "watch_seconds_weight",
        "views_to_roc_ratio",
        "mint_authorized",
        "payout_authorized",
    ] {
        let policy = json!({
            "id": POLICY_ID,
            "hash": POLICY_HASH,
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units": "1000",
            "min_payout_minor_units": "1",
            "weight_bps": 10000,
            "rounding": "floor",
            raw_field: "raw_watch_seconds"
        });

        let err = serde_json::from_value::<RewardPolicy>(policy)
            .expect_err("raw engagement formula field must be rejected");

        assert!(
            err.to_string().contains("unknown field"),
            "{raw_field} should fail as unknown, got {err}"
        );
    }
}

#[test]
fn current_allowed_contribution_counters_are_storage_egress_and_uptime_only() {
    let snapshot = serde_json::from_value::<AccountingSnapshot>(valid_snapshot_value())
        .expect("current allowed contribution shape parses");

    assert_eq!(snapshot.contributions.len(), 1);
    assert_eq!(snapshot.contributions[0].account, "acct_a");
    assert_eq!(snapshot.contributions[0].bytes_stored, 100);
    assert_eq!(snapshot.contributions[0].bytes_served, 40);
    assert_eq!(snapshot.contributions[0].uptime_seconds, 10);
    assert_eq!(snapshot.contributions[0].score().expect("score"), 120);
}
