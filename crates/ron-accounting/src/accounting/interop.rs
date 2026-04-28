//! RO:WHAT — Stable interop fixtures for ron-accounting ↔ svc-rewarder handoff.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Provides deterministic cross-crate vectors.
//! RO:INTERACTS — UsageEvent, RewardSnapshotExport, svc-rewarder AccountingSnapshot contract.
//! RO:INVARIANTS — canonical JSON bytes; b3 CID over canonical bytes; no ledger mutation.
//! RO:METRICS — none; tests/examples may report vector bytes and CID.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — sample accounts are synthetic; no secrets or PII.
//! RO:TEST — unit: interop_vector_tests.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::{MetricKind, RewardContributionExport, RewardSnapshotExport, UsageEvent},
    errors::{Error, Result},
};

/// Current fixture schema name for the reward snapshot interop vector.
pub const REWARD_SNAPSHOT_VECTOR_SCHEMA: &str = "ron-accounting.reward-snapshot.interop.v1";

/// Canonical fixture epoch ID used by rewarder/accounting integration tests.
pub const REWARD_SNAPSHOT_VECTOR_EPOCH_ID: &str = "interop-epoch-1";

/// Stable interop vector used to verify `ron-accounting` can feed `svc-rewarder`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardSnapshotInteropVector {
    /// Fixture schema name.
    pub schema: String,

    /// Epoch ID expected by rewarder integration tests.
    pub epoch_id: String,

    /// Example raw usage events that could produce the contribution shape.
    pub usage_events: Vec<UsageEvent>,

    /// Canonical rewarder-compatible snapshot.
    pub snapshot: RewardSnapshotExport,

    /// Canonical `b3:<hex>` CID over `snapshot.canonical_bytes()`.
    pub snapshot_cid: String,

    /// Canonical compact JSON snapshot bytes represented as UTF-8.
    pub canonical_snapshot_json: String,

    /// Total score according to the v1 simple reward scoring helper.
    pub expected_total_score: u64,

    /// Expected number of contribution accounts.
    pub expected_contribution_count: usize,
}

impl RewardSnapshotInteropVector {
    /// Validate internal vector consistency.
    pub fn validate(&self) -> Result<()> {
        if self.schema != REWARD_SNAPSHOT_VECTOR_SCHEMA {
            return Err(Error::schema("unexpected reward snapshot vector schema"));
        }

        if self.epoch_id.trim().is_empty() {
            return Err(Error::schema("reward snapshot vector epoch_id is empty"));
        }

        let canonical_cid = self.snapshot.canonical_cid()?;
        if self.snapshot_cid != canonical_cid {
            return Err(Error::schema("reward snapshot vector CID mismatch"));
        }

        let canonical_json = canonical_json_for_snapshot(&self.snapshot)?;
        if self.canonical_snapshot_json != canonical_json {
            return Err(Error::schema(
                "reward snapshot vector canonical JSON mismatch",
            ));
        }

        if self.expected_total_score != self.snapshot.total_score()? {
            return Err(Error::schema("reward snapshot vector score mismatch"));
        }

        if self.expected_contribution_count != self.snapshot.contribution_count() {
            return Err(Error::schema(
                "reward snapshot vector contribution count mismatch",
            ));
        }

        Ok(())
    }
}

/// Build the stable v1 reward snapshot interop vector.
///
/// This vector intentionally mirrors the `svc-rewarder` dev snapshot shape:
///
/// ```text
/// produced_at_millis = 1
/// pool_minor_units = "1000"
/// acct_a: bytes_stored=100, bytes_served=50, uptime_seconds=10
/// acct_b: bytes_stored=200, bytes_served=0,  uptime_seconds=20
/// ```
pub fn reward_snapshot_interop_vector_v1() -> Result<RewardSnapshotInteropVector> {
    let snapshot = RewardSnapshotExport::new(
        1,
        "1000",
        vec![
            RewardContributionExport::new("acct_b", 200, 0, 20),
            RewardContributionExport::new("acct_a", 100, 50, 10),
        ],
    )?;

    let usage_events = vec![
        UsageEvent::new(1_000, 1, "acct_a", MetricKind::BytesStored, 100)
            .with_source_service("svc-storage"),
        UsageEvent::new(1_001, 1, "acct_a", MetricKind::BytesServed, 50)
            .with_source_service("svc-edge"),
        UsageEvent::new(1_002, 1, "acct_a", MetricKind::UptimeSeconds, 10)
            .with_source_service("svc-overlay"),
        UsageEvent::new(1_003, 1, "acct_b", MetricKind::BytesStored, 200)
            .with_source_service("svc-storage"),
        UsageEvent::new(1_004, 1, "acct_b", MetricKind::UptimeSeconds, 20)
            .with_source_service("svc-overlay"),
    ];

    let vector = RewardSnapshotInteropVector {
        schema: REWARD_SNAPSHOT_VECTOR_SCHEMA.to_string(),
        epoch_id: REWARD_SNAPSHOT_VECTOR_EPOCH_ID.to_string(),
        usage_events,
        snapshot_cid: snapshot.canonical_cid()?,
        canonical_snapshot_json: canonical_json_for_snapshot(&snapshot)?,
        expected_total_score: snapshot.total_score()?,
        expected_contribution_count: snapshot.contribution_count(),
        snapshot,
    };

    vector.validate()?;
    Ok(vector)
}

/// Return compact canonical JSON for a reward snapshot.
pub fn canonical_json_for_snapshot(snapshot: &RewardSnapshotExport) -> Result<String> {
    String::from_utf8(snapshot.canonical_bytes()?)
        .map_err(|err| Error::schema(format!("canonical snapshot JSON is not UTF-8: {err}")))
}
