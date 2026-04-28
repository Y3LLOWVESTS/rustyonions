//! RO:WHAT — Accounting domain module for labels, dimensions, recorder, windows, slices, rollover.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/RES. Converts transient counters into sealed usage snapshots.
//! RO:INTERACTS — exporter, config, metrics, utils, svc-rewarder input adapters.
//! RO:INVARIANTS — not a ledger; non-negative counters; monotone window rows; deterministic seal.
//! RO:METRICS — recorder and rollover callers update accounting_* metrics.
//! RO:CONFIG — RecorderConfig and Window use accounting.* config values.
//! RO:SECURITY — labels must be normalized and PII-safe.
//! RO:TEST — unit: recording_tests, rollover_tests, event_ingest_tests, interop_vector_tests; benches: record, seal.

pub mod dimensions;
pub mod events;
pub mod interop;
pub mod labels;
pub mod recorder;
pub mod reward_projection;
pub mod reward_snapshot;
pub mod rollover;
pub mod slice;
pub mod window;

pub use dimensions::{Dimension, BYTES, CPU_UNITS, REQUESTS};
pub use events::{
    record_usage_event, record_usage_events, EventIngestPolicy, EventIngestReport,
    EventSubjectMode, MetricKind, UsageCounterInput, UsageEvent,
};
pub use interop::{
    canonical_json_for_snapshot, reward_snapshot_interop_vector_v1, RewardSnapshotInteropVector,
    REWARD_SNAPSHOT_VECTOR_EPOCH_ID, REWARD_SNAPSHOT_VECTOR_SCHEMA,
};
pub use labels::{AccountKey, LabelSet, Namespace, Row, TenantId};
pub use recorder::{CounterKey, CounterRow, Recorder, RecorderConfig};
pub use reward_projection::{
    account_from_labels, project_reward_snapshot_from_slices, ProjectedRewardSnapshot,
    RewardAccountMode, RewardProjectionConfig, RewardProjectionReport,
};
pub use reward_snapshot::{
    canonical_snapshot_bytes, canonical_snapshot_cid, RewardContributionExport,
    RewardSnapshotExport,
};
pub use rollover::{RolloverDecision, RolloverHandle};
pub use slice::{SealedSlice, SliceId, SliceMeta, SliceRow};
pub use window::Window;
