//! RO:WHAT — Unit tests for sealed-slice to reward-snapshot projection.
//! RO:WHY — Pillar 12; Concerns: ECON/DX. Proves accounting can feed svc-rewarder deterministically.
//! RO:INTERACTS — reward_projection, reward_snapshot, SealedSlice.
//! RO:INVARIANTS — stable CID independent of input order; checked integer aggregation.
//! RO:METRICS — none.
//! RO:CONFIG — RewardProjectionConfig.
//! RO:SECURITY — no secrets; account IDs derived from normalized labels.
//! RO:TEST — cargo test -p ron-accounting --test unit.

use ron_accounting::{
    account_from_labels, project_reward_snapshot_from_slices, Dimension, LabelSet,
    RewardAccountMode, RewardProjectionConfig, SealedSlice, SliceId, SliceMeta, SliceRow, Window,
};

fn meta() -> SliceMeta {
    SliceMeta::new(
        Window::for_timestamp_ms(300_000, 300).expect("window"),
        300_001,
        None,
        true,
    )
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

#[test]
fn projection_maps_put_bytes_to_stored_and_get_bytes_to_served() {
    let slices = vec![slice(
        1,
        vec![
            row(
                1,
                "svc-storage",
                "local",
                "PUT",
                "/objects",
                Dimension::Bytes,
                1_000,
            ),
            row(
                1,
                "svc-storage",
                "local",
                "GET",
                "/objects",
                Dimension::Bytes,
                250,
            ),
        ],
    )];

    let projected =
        project_reward_snapshot_from_slices(42, &RewardProjectionConfig::new("500"), &slices)
            .expect("projected snapshot");

    assert_eq!(projected.report.input_slices, 1);
    assert_eq!(projected.report.input_rows, 2);
    assert_eq!(projected.report.projected_accounts, 1);
    assert_eq!(projected.report.bytes_stored, 1_000);
    assert_eq!(projected.report.bytes_served, 250);
    assert_eq!(projected.snapshot.pool_minor_units, "500");

    let contribution = &projected.snapshot.contributions[0];
    assert_eq!(contribution.account, "t:1/svc:svc-storage");
    assert_eq!(contribution.bytes_stored, 1_000);
    assert_eq!(contribution.bytes_served, 250);
    assert_eq!(contribution.uptime_seconds, 0);
}

#[test]
fn projection_can_group_by_tenant_service_region() {
    let labels = LabelSet::new(7, "svc-storage", "us-central", "PUT", "/objects");

    assert_eq!(
        account_from_labels(&labels, RewardAccountMode::TenantService),
        "t:7/svc:svc-storage"
    );
    assert_eq!(
        account_from_labels(&labels, RewardAccountMode::TenantServiceRegion),
        "t:7/svc:svc-storage/r:us-central"
    );
}

#[test]
fn projection_cid_is_stable_when_input_slice_order_changes() {
    let first = slice(
        1,
        vec![row(
            1,
            "svc-storage",
            "local",
            "PUT",
            "/objects",
            Dimension::Bytes,
            100,
        )],
    );
    let second = slice(
        2,
        vec![row(
            1,
            "svc-gateway",
            "local",
            "GET",
            "/objects",
            Dimension::Bytes,
            25,
        )],
    );

    let cfg = RewardProjectionConfig::new("1000");
    let left = project_reward_snapshot_from_slices(99, &cfg, &[first.clone(), second.clone()])
        .expect("left");
    let right = project_reward_snapshot_from_slices(99, &cfg, &[second, first]).expect("right");

    assert_eq!(left.snapshot_cid, right.snapshot_cid);
    assert_eq!(left.snapshot.contributions, right.snapshot.contributions);
}

#[test]
fn projection_maps_uptime_request_rows_to_uptime_seconds() {
    let slices = vec![slice(
        1,
        vec![row(
            1,
            "svc-storage",
            "local",
            "UPTIME",
            "/health",
            Dimension::Requests,
            60,
        )],
    )];

    let projected =
        project_reward_snapshot_from_slices(42, &RewardProjectionConfig::new("500"), &slices)
            .expect("projected snapshot");

    assert_eq!(projected.report.uptime_seconds, 60);
    assert_eq!(projected.snapshot.contributions[0].uptime_seconds, 60);
}

#[test]
fn projection_ignores_unmapped_request_rows() {
    let slices = vec![slice(
        1,
        vec![row(
            1,
            "svc-storage",
            "local",
            "POST",
            "/objects",
            Dimension::Requests,
            3,
        )],
    )];

    let result =
        project_reward_snapshot_from_slices(42, &RewardProjectionConfig::new("500"), &slices);

    assert!(result.is_err());
}

#[test]
fn projection_rejects_overflowing_aggregates() {
    let slices = vec![slice(
        1,
        vec![
            row(
                1,
                "svc-storage",
                "local",
                "PUT",
                "/objects",
                Dimension::Bytes,
                u64::MAX,
            ),
            row(
                1,
                "svc-storage",
                "local",
                "PUT",
                "/objects",
                Dimension::Bytes,
                1,
            ),
        ],
    )];

    let result =
        project_reward_snapshot_from_slices(42, &RewardProjectionConfig::new("500"), &slices);

    assert!(result.is_err());
}
