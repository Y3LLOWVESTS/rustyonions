//! RO:WHAT — Library façade for transient ROC metering, sealing, and ordered export.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/RES. Keeps counters separate from ledger truth.
//! RO:INTERACTS — accounting, exporter, config, metrics, readiness, svc-rewarder, svc-wallet.
//! RO:INVARIANTS — not a ledger; bounded counters; deterministic sealed slices; no unsafe code.
//! RO:METRICS — exposes accounting_* handles when the `metrics` feature is enabled.
//! RO:CONFIG — reads ron-accounting Config via config::{load, validate}.
//! RO:SECURITY — no secrets; labels are normalized to avoid PII/cardinality blowups.
//! RO:TEST — unit tests under tests/unit plus examples/minimal.rs.

#![forbid(unsafe_code)]
#![deny(clippy::await_holding_lock)]

pub mod accounting;
pub mod config;
pub mod errors;
pub mod exporter;
pub mod metrics;
pub mod normalize;
pub mod readiness;
pub mod utils;

#[cfg(feature = "wal")]
pub mod wal;

pub use accounting::{
    account_from_labels, canonical_json_for_snapshot, canonical_snapshot_bytes,
    canonical_snapshot_cid, project_reward_snapshot_from_slices, record_usage_event,
    record_usage_events, reward_snapshot_interop_vector_v1, AccountKey, CounterRow, Dimension,
    EventIngestPolicy, EventIngestReport, EventSubjectMode, LabelSet, MetricKind, Namespace,
    ProjectedRewardSnapshot, Recorder, RecorderConfig, RewardAccountMode, RewardContributionExport,
    RewardProjectionConfig, RewardProjectionReport, RewardSnapshotExport,
    RewardSnapshotInteropVector, Row, SealedSlice, SliceId, SliceMeta, SliceRow, TenantId,
    UsageCounterInput, UsageEvent, Window, REWARD_SNAPSHOT_VECTOR_EPOCH_ID,
    REWARD_SNAPSHOT_VECTOR_SCHEMA,
};
pub use errors::{Error, Result};
pub use exporter::{Ack, AckLru, BoxExportFuture, Exporter, ExporterRouter};
pub use readiness::{Readiness, ReadyKey};

#[cfg(feature = "wal")]
pub use wal::{Wal, WalConfig, WalStats};
