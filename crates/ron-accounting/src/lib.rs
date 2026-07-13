//! RO:WHAT — Library façade for transient ROC metering, sealing, ordered export, and lightweight ingest.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/RES. Keeps counters separate from ledger truth.
//! RO:INTERACTS — accounting, exporter, config, metrics, readiness, svc-storage, svc-rewarder, svc-wallet.
//! RO:INVARIANTS — not a ledger; bounded counters; deterministic sealed slices; no unsafe code.
//! RO:METRICS — exposes accounting_* handles when the `metrics` feature is enabled.
//! RO:CONFIG — reads ron-accounting Config via config::{load, validate}; HTTP ingest reads RON_ACCOUNTING_ADDR.
//! RO:SECURITY — no secrets; labels are normalized to avoid PII/cardinality blowups.
//! RO:TEST — unit tests under tests/unit plus examples/minimal.rs and live storage→accounting smoke.

#![forbid(unsafe_code)]
#![deny(clippy::await_holding_lock)]

pub mod accounting;
pub mod config;
pub mod errors;
pub mod exporter;
pub mod http_ingest;
pub mod metrics;
pub mod normalize;
pub mod readiness;
pub mod utils;

#[cfg(feature = "wal")]
pub mod wal;

pub use accounting::{
    account_from_labels, canonical_json_for_snapshot, canonical_snapshot_bytes,
    canonical_snapshot_cid, classify_metric_for_internal_roc, classify_service_evidence,
    classify_service_evidence_batch, economic_receipt_decision_from_source,
    project_reward_snapshot_from_slices, record_usage_event, record_usage_events,
    reward_snapshot_interop_vector_v1, AccountKey, CounterRow, Dimension, EventIngestPolicy,
    EventIngestReport, EventSubjectMode, InternalRocEconomicsConfigLabel, InternalRocEventClass,
    InternalRocEventClassDecision, LabelSet, MetricKind, Namespace, ProjectedRewardSnapshot,
    QuickChainAnchorReport, QuickChainBondDisputeReport, QuickChainBondDisputeReportStatus,
    QuickChainBondEnforcementReport, QuickChainBondEnforcementReportAction, QuickChainBondReport,
    QuickChainDaFallbackReport, QuickChainExternalPostureReport,
    QuickChainExternalPostureReportStatus, Recorder, RecorderConfig, RewardAccountMode,
    RewardContributionExport, RewardProjectionConfig, RewardProjectionReport, RewardSnapshotExport,
    RewardSnapshotInteropVector, Row, SealedSlice, ServiceEvidenceAccountingClassV1,
    ServiceEvidenceAccountingDecisionV1, ServiceEvidenceClassificationBatchV1, SliceId, SliceMeta,
    SliceRow, TenantId, UsageCounterInput, UsageEvent, Window,
    MAX_SERVICE_EVIDENCE_CLASSIFICATION_ITEMS, REWARD_SNAPSHOT_VECTOR_EPOCH_ID,
    REWARD_SNAPSHOT_VECTOR_SCHEMA, RON_ACCOUNTING_INTERNAL_ROC_ECONOMICS_CONFIG_LABEL_SCHEMA,
    RON_ACCOUNTING_QUICKCHAIN_ANCHOR_REPORT_SCHEMA,
    RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA,
    RON_ACCOUNTING_QUICKCHAIN_BOND_ENFORCEMENT_REPORT_SCHEMA,
    RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA,
    RON_ACCOUNTING_QUICKCHAIN_DA_FALLBACK_REPORT_SCHEMA,
    RON_ACCOUNTING_QUICKCHAIN_EXTERNAL_POSTURE_REPORT_SCHEMA,
    SERVICE_EVIDENCE_CLASSIFICATION_BATCH_SCHEMA, SERVICE_EVIDENCE_CLASSIFICATION_BATCH_VERSION,
};
pub use errors::{Error, Result};
pub use exporter::{Ack, AckLru, BoxExportFuture, Exporter, ExporterRouter};
pub use readiness::{Readiness, ReadyKey};

#[cfg(feature = "wal")]
pub use wal::{Wal, WalConfig, WalStats};

pub use accounting::user_verification::{
    classify_user_verification, classify_user_verification_batch,
    UserVerificationAccountingClassV1, UserVerificationAccountingDecisionV1,
    UserVerificationClassificationBatchV1, MAX_USER_VERIFICATION_CLASSIFICATION_ITEMS,
    USER_VERIFICATION_CLASSIFICATION_BATCH_SCHEMA, USER_VERIFICATION_CLASSIFICATION_BATCH_VERSION,
};

pub use accounting::epoch_snapshot::{
    build_accounting_epoch_snapshot, canonical_accounting_epoch_snapshot_artifact_cid,
    canonical_accounting_epoch_snapshot_bytes, AccountingEconomicsConfigBindingV1,
    AccountingEconomicsProfileV1, AccountingEpochSnapshotV1, AccountingEpochWindowV1,
    ServiceAccountingSnapshotRowV1, UserVerificationAccountingSnapshotRowV1,
    ACCOUNTING_ECONOMICS_CONFIG_BINDING_SCHEMA, ACCOUNTING_EPOCH_SNAPSHOT_SCHEMA,
    ACCOUNTING_EPOCH_SNAPSHOT_VERSION,
};

pub use accounting::node_evidence_wire::{
    EvidenceContentId, ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingKindV1,
    UserVerificationAccountingInputV1, UserVerificationEvidenceKindV1,
    UserVerificationFailureReasonV1, UserVerificationResultV1,
    SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA, SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
    USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA, USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
};
