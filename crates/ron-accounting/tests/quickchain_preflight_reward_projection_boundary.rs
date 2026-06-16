//! RO:WHAT — QuickChain Phase-0 reward projection boundary tests for ron-accounting.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Proves projection remains payout planning input only.
//! RO:INTERACTS — UsageEvent, MetricKind, SealedSlice, RewardProjectionConfig, ProjectedRewardSnapshot.
//! RO:INVARIANTS — raw engagement does not allocate protocol ROC; projection emits artifact CID, not roots.
//! RO:METRICS — none.
//! RO:CONFIG — RewardProjectionConfig only.
//! RO:SECURITY — no payout execution, no ledger/wallet mutation, no finality/validator/checkpoint authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_preflight_reward_projection_boundary.

use ron_accounting::{
    canonical_snapshot_bytes, project_reward_snapshot_from_slices, Dimension, EventIngestPolicy,
    LabelSet, MetricKind, RewardProjectionConfig, SealedSlice, SliceId, SliceMeta, SliceRow,
    UsageEvent, Window,
};
use serde_json::Value;

fn meta() -> SliceMeta {
    SliceMeta::new(
        Window::for_timestamp_ms(300_000, 300).expect("window"),
        300_001,
        None,
        true,
    )
}

fn slice(seq: u64, rows: Vec<SliceRow>) -> SealedSlice {
    SealedSlice::new(
        SliceId {
            tenant: 1,
            dimension: Dimension::Bytes,
            seq,
        },
        meta(),
        rows,
    )
    .expect("sealed slice")
}

fn row(
    tenant: u128,
    service: &str,
    region: &str,
    method: &str,
    route: &str,
    dimension: Dimension,
    value: u64,
) -> SliceRow {
    SliceRow {
        labels: LabelSet::new(tenant, service, region, method, route),
        dimension,
        value,
    }
}

fn row_from_event(event: UsageEvent) -> SliceRow {
    let counter = event
        .to_counter(&EventIngestPolicy::default())
        .expect("usage event should convert to counter");

    SliceRow {
        labels: counter.labels,
        dimension: counter.dimension,
        value: counter.value,
    }
}

fn storage_reward_rows_for_account(account: &str) -> Vec<SliceRow> {
    vec![
        row_from_event(
            UsageEvent::new(1_000, 7, account, MetricKind::BytesStored, 100)
                .with_source_service("svc-storage"),
        ),
        row_from_event(
            UsageEvent::new(1_001, 7, account, MetricKind::BytesServed, 40)
                .with_source_service("svc-gateway"),
        ),
        row_from_event(
            UsageEvent::new(1_002, 7, account, MetricKind::UptimeSeconds, 30)
                .with_source_service("svc-overlay"),
        ),
    ]
}

fn assert_b3_artifact_cid(value: &str) {
    assert_eq!(
        value.len(),
        67,
        "artifact CID must be b3:<64 lowercase hex>"
    );
    assert!(value.starts_with("b3:"), "artifact CID must use b3: prefix");
    assert!(
        value.as_bytes()[3..]
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "artifact CID must use lowercase hex only"
    );
}

fn assert_no_authority_key_fragments(value: &Value) {
    const FORBIDDEN_KEY_FRAGMENTS: &[&str] = &[
        "root",
        "receipt",
        "balance",
        "operation",
        "sequence",
        "settlement",
        "finality",
        "finalized",
        "validator",
        "checkpoint",
        "anchor",
        "bridge",
        "staking",
        "liquidity",
        "payout",
        "ledger",
        "wallet",
        "mint",
        "burn",
        "transfer",
        "hold",
        "capture",
        "release",
    ];

    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let normalized = key.to_ascii_lowercase();

                for forbidden in FORBIDDEN_KEY_FRAGMENTS {
                    assert!(
                        !normalized.contains(forbidden),
                        "projection artifact key must not imply authority: {key}"
                    );
                }

                assert_no_authority_key_fragments(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_authority_key_fragments(item);
            }
        }
        _ => {}
    }
}

#[test]
fn projection_accepts_storage_served_and_uptime_but_ignores_analytics() {
    let mut rows = storage_reward_rows_for_account("provider-a");

    rows.push(row_from_event(
        UsageEvent::new(
            1_003,
            7,
            "provider-a",
            MetricKind::Custom("views".to_string()),
            999,
        )
        .with_source_service("omnigate")
        .with_route("/analytics/views"),
    ));

    rows.push(row_from_event(
        UsageEvent::new(1_004, 7, "provider-a", MetricKind::RequestOk, 1)
            .with_source_service("svc-gateway")
            .with_route("/site/view"),
    ));

    let projected = project_reward_snapshot_from_slices(
        42,
        &RewardProjectionConfig::new("1000"),
        &[slice(1, rows)],
    )
    .expect("projection should accept only reward-eligible counters");

    assert_b3_artifact_cid(&projected.snapshot_cid);

    assert_eq!(projected.report.input_slices, 1);
    assert_eq!(projected.report.input_rows, 5);
    assert_eq!(projected.report.ignored_rows, 2);
    assert_eq!(projected.report.projected_accounts, 1);
    assert_eq!(projected.report.bytes_stored, 100);
    assert_eq!(projected.report.bytes_served, 40);
    assert_eq!(projected.report.uptime_seconds, 30);

    assert_eq!(projected.snapshot.contributions.len(), 1);
    assert_eq!(
        projected.snapshot.contributions[0].account,
        "t:7/svc:provider-a"
    );
    assert_eq!(projected.snapshot.contributions[0].bytes_stored, 100);
    assert_eq!(projected.snapshot.contributions[0].bytes_served, 40);
    assert_eq!(projected.snapshot.contributions[0].uptime_seconds, 30);
}

#[test]
fn raw_engagement_and_non_reward_metrics_alone_cannot_project_rewards() {
    let non_reward_events = vec![
        UsageEvent::new(
            1_000,
            7,
            "creator-a",
            MetricKind::Custom("views".to_string()),
            1,
        )
        .with_source_service("omnigate")
        .with_route("/analytics/views"),
        UsageEvent::new(
            1_001,
            7,
            "creator-a",
            MetricKind::Custom("likes".to_string()),
            1,
        )
        .with_source_service("omnigate")
        .with_route("/analytics/likes"),
        UsageEvent::new(1_002, 7, "provider-a", MetricKind::RequestOk, 1)
            .with_source_service("svc-gateway")
            .with_route("/site/view"),
        UsageEvent::new(1_003, 7, "provider-a", MetricKind::PinSeconds, 60)
            .with_source_service("svc-storage"),
        UsageEvent::new(1_004, 7, "provider-a", MetricKind::CpuUnits, 25)
            .with_source_service("svc-worker"),
    ];

    for event in non_reward_events {
        let metric_kind = event.metric_kind.clone();
        let rows = vec![row_from_event(event)];

        let result = project_reward_snapshot_from_slices(
            42,
            &RewardProjectionConfig::new("1000"),
            &[slice(1, rows)],
        );

        assert!(
            result.is_err(),
            "non-reward metric must not produce reward projection output: {metric_kind:?}"
        );
    }
}

#[test]
fn current_event_class_doctrine_is_narrow_and_explicit() {
    assert!(MetricKind::BytesStored.is_reward_projection_input());
    assert!(MetricKind::BytesServed.is_reward_projection_input());
    assert!(MetricKind::UptimeSeconds.is_reward_projection_input());

    assert!(!MetricKind::RequestOk.is_reward_projection_input());
    assert!(!MetricKind::PinSeconds.is_reward_projection_input());
    assert!(!MetricKind::CpuUnits.is_reward_projection_input());
    assert!(!MetricKind::Custom("views".to_string()).is_reward_projection_input());
    assert!(!MetricKind::Custom("likes".to_string()).is_reward_projection_input());
    assert!(!MetricKind::Custom("impressions".to_string()).is_reward_projection_input());
}

#[test]
fn projection_artifact_does_not_expose_payout_or_chain_authority_fields() {
    let projected = project_reward_snapshot_from_slices(
        42,
        &RewardProjectionConfig::new("1000"),
        &[slice(1, storage_reward_rows_for_account("provider-a"))],
    )
    .expect("projection");

    let value = serde_json::to_value(&projected).expect("projected JSON");

    assert_b3_artifact_cid(&projected.snapshot_cid);
    assert_no_authority_key_fragments(&value);

    let object = value.as_object().expect("projected object");
    assert!(
        object.contains_key("snapshot_cid"),
        "projection may expose artifact CID"
    );
    assert!(
        !object.contains_key("reward_root"),
        "projection must not expose a reward root in Phase 0"
    );
    assert!(
        !object.contains_key("accounting_root"),
        "projection must not expose an accounting root in Phase 0"
    );
}

#[test]
fn projection_order_is_deterministic_but_still_not_a_root() {
    let left = slice(
        1,
        vec![
            row(
                7,
                "provider-b",
                "local",
                "PUT",
                "/objects",
                Dimension::Bytes,
                200,
            ),
            row(
                7,
                "provider-a",
                "local",
                "GET",
                "/objects",
                Dimension::Bytes,
                50,
            ),
        ],
    );

    let right = slice(
        2,
        vec![
            row(
                7,
                "provider-a",
                "local",
                "PUT",
                "/objects",
                Dimension::Bytes,
                100,
            ),
            row(
                7,
                "provider-b",
                "local",
                "UPTIME",
                "/usage/uptime",
                Dimension::Requests,
                10,
            ),
        ],
    );

    let config = RewardProjectionConfig::new("1000");

    let projected_a =
        project_reward_snapshot_from_slices(42, &config, &[left.clone(), right.clone()])
            .expect("projection a");
    let projected_b =
        project_reward_snapshot_from_slices(42, &config, &[right, left]).expect("projection b");

    assert_eq!(projected_a.snapshot_cid, projected_b.snapshot_cid);
    assert_eq!(
        canonical_snapshot_bytes(&projected_a.snapshot).expect("snapshot a bytes"),
        canonical_snapshot_bytes(&projected_b.snapshot).expect("snapshot b bytes")
    );

    assert_b3_artifact_cid(&projected_a.snapshot_cid);
    assert!(
        !serde_json::to_value(&projected_a)
            .expect("projected JSON")
            .as_object()
            .expect("projected object")
            .keys()
            .any(|key| key.ends_with("_root")),
        "deterministic artifact CID must not be promoted into a root field"
    );
}

#[test]
fn projection_rejects_float_money_and_empty_reward_inputs() {
    let float_pool = RewardProjectionConfig::new("1000.5");

    assert!(
        project_reward_snapshot_from_slices(
            42,
            &float_pool,
            &[slice(1, storage_reward_rows_for_account("provider-a"))],
        )
        .is_err(),
        "pool_minor_units must remain integer minor-unit string only"
    );

    let analytics_only = vec![row_from_event(
        UsageEvent::new(
            1_000,
            7,
            "creator-a",
            MetricKind::Custom("views".to_string()),
            100,
        )
        .with_source_service("omnigate")
        .with_route("/analytics/views"),
    )];

    assert!(
        project_reward_snapshot_from_slices(
            42,
            &RewardProjectionConfig::new("1000"),
            &[slice(1, analytics_only)],
        )
        .is_err(),
        "analytics-only rows must not create an empty reward snapshot"
    );
}
