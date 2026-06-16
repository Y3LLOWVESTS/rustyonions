//! RO:WHAT — Projects sealed accounting slices into rewarder-compatible contribution snapshots.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/RES. Bridges metering output into svc-rewarder input.
//! RO:INTERACTS — accounting::{SealedSlice, SliceRow, RewardSnapshotExport}, svc-rewarder.
//! RO:INVARIANTS — deterministic account order; integer-only accumulation; no ledger mutation.
//! RO:METRICS — callers may increment accounting_reward_projection_* counters/gauges.
//! RO:CONFIG — RewardProjectionConfig controls account grouping and reward pool string.
//! RO:SECURITY — account IDs are derived from normalized labels; no ROX/external-chain behavior.
//! RO:TEST — unit: reward_projection_tests.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    accounting::{
        Dimension, LabelSet, RewardContributionExport, RewardSnapshotExport, SealedSlice,
    },
    errors::{Error, Result},
};

/// Account grouping policy for reward projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardAccountMode {
    /// Group contributions by `(tenant, service)`.
    TenantService,

    /// Group contributions by `(tenant, service, region)`.
    TenantServiceRegion,
}

impl Default for RewardAccountMode {
    fn default() -> Self {
        Self::TenantService
    }
}

/// Projection configuration for converting sealed slices into reward snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardProjectionConfig {
    /// Reward pool in ROC minor units, string-encoded for JSON boundary safety.
    pub pool_minor_units: String,

    /// Account grouping policy.
    pub account_mode: RewardAccountMode,
}

impl RewardProjectionConfig {
    /// Create a projection config with a reward pool and the default account mode.
    pub fn new(pool_minor_units: impl Into<String>) -> Self {
        Self {
            pool_minor_units: pool_minor_units.into(),
            account_mode: RewardAccountMode::default(),
        }
    }

    /// Set account grouping mode.
    pub fn with_account_mode(mut self, account_mode: RewardAccountMode) -> Self {
        self.account_mode = account_mode;
        self
    }
}

impl Default for RewardProjectionConfig {
    fn default() -> Self {
        Self {
            pool_minor_units: "0".to_string(),
            account_mode: RewardAccountMode::TenantService,
        }
    }
}

/// Projection report for observability and tests.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardProjectionReport {
    /// Number of input sealed slices inspected.
    pub input_slices: usize,

    /// Number of rows inspected across all slices.
    pub input_rows: usize,

    /// Number of contribution accounts emitted.
    pub projected_accounts: usize,

    /// Total stored bytes projected.
    pub bytes_stored: u64,

    /// Total served bytes projected.
    pub bytes_served: u64,

    /// Total uptime seconds projected.
    pub uptime_seconds: u64,

    /// Rows intentionally ignored because they do not map into rewarder contribution counters.
    pub ignored_rows: usize,
}

/// Result of projecting sealed slices into a reward snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedRewardSnapshot {
    /// Canonical reward snapshot export.
    pub snapshot: RewardSnapshotExport,

    /// Canonical `b3:<hex>` CID over the snapshot bytes.
    pub snapshot_cid: String,

    /// Projection report.
    pub report: RewardProjectionReport,
}

/// Project sealed accounting slices into a canonical rewarder-compatible snapshot.
///
/// Mapping rules are intentionally conservative:
///
/// - `Dimension::Bytes` + `GET|HEAD` or served/download/egress-looking route => `bytes_served`
/// - `Dimension::Bytes` + everything else => `bytes_stored`
/// - `Dimension::Requests` + uptime-looking method/route => `uptime_seconds`
/// - all other rows are ignored
pub fn project_reward_snapshot_from_slices(
    produced_at_millis: u64,
    config: &RewardProjectionConfig,
    slices: &[SealedSlice],
) -> Result<ProjectedRewardSnapshot> {
    let mut accounts: BTreeMap<String, ContributionAccumulator> = BTreeMap::new();
    let mut report = RewardProjectionReport {
        input_slices: slices.len(),
        ..RewardProjectionReport::default()
    };

    for slice in slices {
        for row in &slice.rows {
            report.input_rows = report
                .input_rows
                .checked_add(1)
                .ok_or_else(|| Error::schema("projection row counter overflow"))?;

            let Some(counter) = classify_reward_counter(row.dimension, &row.labels) else {
                report.ignored_rows = report
                    .ignored_rows
                    .checked_add(1)
                    .ok_or_else(|| Error::schema("projection ignored row counter overflow"))?;
                continue;
            };

            let account = account_from_labels(&row.labels, config.account_mode);
            let accumulator = accounts.entry(account).or_default();

            match counter {
                RewardCounterKind::BytesStored => {
                    accumulator.bytes_stored =
                        checked_add(accumulator.bytes_stored, row.value, "bytes_stored")?;
                    report.bytes_stored =
                        checked_add(report.bytes_stored, row.value, "report.bytes_stored")?;
                }
                RewardCounterKind::BytesServed => {
                    accumulator.bytes_served =
                        checked_add(accumulator.bytes_served, row.value, "bytes_served")?;
                    report.bytes_served =
                        checked_add(report.bytes_served, row.value, "report.bytes_served")?;
                }
                RewardCounterKind::UptimeSeconds => {
                    accumulator.uptime_seconds =
                        checked_add(accumulator.uptime_seconds, row.value, "uptime_seconds")?;
                    report.uptime_seconds =
                        checked_add(report.uptime_seconds, row.value, "report.uptime_seconds")?;
                }
            }
        }
    }

    let contributions = accounts
        .into_iter()
        .map(|(account, acc)| {
            RewardContributionExport::new(
                account,
                acc.bytes_stored,
                acc.bytes_served,
                acc.uptime_seconds,
            )
        })
        .collect::<Vec<_>>();

    report.projected_accounts = contributions.len();

    let snapshot = RewardSnapshotExport::new(
        produced_at_millis,
        config.pool_minor_units.clone(),
        contributions,
    )?;
    let snapshot_cid = snapshot.canonical_cid()?;

    Ok(ProjectedRewardSnapshot {
        snapshot,
        snapshot_cid,
        report,
    })
}

/// Build a canonical reward account string from normalized labels.
pub fn account_from_labels(labels: &LabelSet, account_mode: RewardAccountMode) -> String {
    match account_mode {
        RewardAccountMode::TenantService => {
            format!("t:{}/svc:{}", labels.tenant, labels.service)
        }
        RewardAccountMode::TenantServiceRegion => {
            format!(
                "t:{}/svc:{}/r:{}",
                labels.tenant, labels.service, labels.region
            )
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RewardCounterKind {
    BytesStored,
    BytesServed,
    UptimeSeconds,
}

#[derive(Debug, Clone, Copy, Default)]
struct ContributionAccumulator {
    bytes_stored: u64,
    bytes_served: u64,
    uptime_seconds: u64,
}

fn classify_reward_counter(dimension: Dimension, labels: &LabelSet) -> Option<RewardCounterKind> {
    match dimension {
        Dimension::Bytes => {
            if looks_like_served_bytes(labels) {
                Some(RewardCounterKind::BytesServed)
            } else {
                Some(RewardCounterKind::BytesStored)
            }
        }
        Dimension::Requests if looks_like_uptime(labels) => Some(RewardCounterKind::UptimeSeconds),
        _ => None,
    }
}

fn looks_like_served_bytes(labels: &LabelSet) -> bool {
    let method = labels.method.as_str();
    let route = labels.route.as_str();

    matches!(method, "GET" | "HEAD")
        || route.contains("serve")
        || route.contains("served")
        || route.contains("download")
        || route.contains("egress")
        || route.contains("read")
}

fn looks_like_uptime(labels: &LabelSet) -> bool {
    labels.method == "UPTIME"
        || labels.route.contains("uptime")
        || labels.route.contains("heartbeat")
        || labels.route.contains("health")
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| Error::schema(format!("{label} overflow during reward projection")))
}
