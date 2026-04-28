//! RO:WHAT — Stable usage-event DTOs and ingest helpers for recording metered activity.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/RES. Provides the minimal WEB3 event schema.
//! RO:INTERACTS — Recorder, LabelSet, Dimension, reward projection, future HTTP/OAP adapters.
//! RO:INVARIANTS — integer-only values; subject is normalized; zero-value events are no-ops.
//! RO:METRICS — callers increment accounting_events_ingested_total and reject counters.
//! RO:CONFIG — EventIngestPolicy controls subject/source attribution.
//! RO:SECURITY — no raw PII labels; subject/source/route are normalized before recording.
//! RO:TEST — unit: event_ingest_tests.

use serde::{Deserialize, Serialize};

use crate::{
    accounting::{Dimension, LabelSet, Recorder, TenantId},
    errors::{Error, Result},
    normalize::{normalize_component, normalize_route},
};

/// Stable metering event kind for the ROC value plane.
///
/// These are counters, not balances and not ledger entries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    /// Bytes stored by a provider/service.
    BytesStored,

    /// Bytes served or egressed by a provider/service.
    BytesServed,

    /// Pin duration in seconds.
    PinSeconds,

    /// Successful request count.
    RequestOk,

    /// Uptime/health contribution seconds.
    UptimeSeconds,

    /// Abstract CPU/work units.
    CpuUnits,

    /// Forward-compatible custom metric kind.
    Custom(String),
}

impl MetricKind {
    /// Return the storage dimension used by the recorder.
    pub fn dimension(&self) -> Dimension {
        match self {
            Self::BytesStored | Self::BytesServed => Dimension::Bytes,
            Self::CpuUnits => Dimension::Cpu,
            Self::PinSeconds | Self::RequestOk | Self::UptimeSeconds | Self::Custom(_) => {
                Dimension::Requests
            }
        }
    }

    /// Return the method/source label used by the recorder.
    pub fn method_label(&self) -> String {
        match self {
            Self::BytesStored => "PUT".to_string(),
            Self::BytesServed => "GET".to_string(),
            Self::PinSeconds => "PIN_SECONDS".to_string(),
            Self::RequestOk => "REQ_OK".to_string(),
            Self::UptimeSeconds => "UPTIME".to_string(),
            Self::CpuUnits => "CPU".to_string(),
            Self::Custom(value) => normalize_component(value)
                .replace(['-', '.'], "_")
                .to_ascii_uppercase(),
        }
    }

    /// Return the default route/operation label used by the recorder.
    pub fn default_route(&self) -> String {
        match self {
            Self::BytesStored => "/usage/bytes-stored".to_string(),
            Self::BytesServed => "/usage/bytes-served".to_string(),
            Self::PinSeconds => "/usage/pin-seconds".to_string(),
            Self::RequestOk => "/usage/request-ok".to_string(),
            Self::UptimeSeconds => "/usage/uptime".to_string(),
            Self::CpuUnits => "/usage/cpu-units".to_string(),
            Self::Custom(value) => format!("/usage/custom/{}", normalize_component(value)),
        }
    }

    /// Return true when this metric kind can currently feed reward projection directly.
    pub fn is_reward_projection_input(&self) -> bool {
        matches!(
            self,
            Self::BytesStored | Self::BytesServed | Self::UptimeSeconds
        )
    }
}

/// Attribution mode for converting usage events into recorder labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSubjectMode {
    /// Attribute usage to `event.subject`.
    Subject,

    /// Attribute usage to `event.source_service`.
    SourceService,
}

impl Default for EventSubjectMode {
    fn default() -> Self {
        Self::Subject
    }
}

/// Runtime policy for ingesting events into the recorder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventIngestPolicy {
    /// Which event field becomes the `LabelSet.service` attribution key.
    pub subject_mode: EventSubjectMode,

    /// Default region used when an event has no region.
    pub default_region: String,

    /// Default source service used when an event has no source service.
    pub default_source_service: String,
}

impl Default for EventIngestPolicy {
    fn default() -> Self {
        Self {
            subject_mode: EventSubjectMode::Subject,
            default_region: "local".to_string(),
            default_source_service: "unknown".to_string(),
        }
    }
}

/// Minimal usage event DTO from WEB3.
///
/// `subject` is the provider/user/service being credited or metered. In the
/// default policy, it becomes the `LabelSet.service` value so reward projection
/// can group by provider-like account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageEvent {
    /// Event timestamp in Unix milliseconds.
    pub timestamp_ms: u64,

    /// Tenant that owns this usage event.
    pub tenant: TenantId,

    /// Provider/user/service subject being metered.
    pub subject: String,

    /// Metric kind.
    pub metric_kind: MetricKind,

    /// Non-negative counter increment.
    pub value: u64,

    /// Optional service that observed/emitted the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_service: Option<String>,

    /// Optional region/locality label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Optional route/operation override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

impl UsageEvent {
    /// Construct a minimal usage event.
    pub fn new(
        timestamp_ms: u64,
        tenant: TenantId,
        subject: impl Into<String>,
        metric_kind: MetricKind,
        value: u64,
    ) -> Self {
        Self {
            timestamp_ms,
            tenant,
            subject: subject.into(),
            metric_kind,
            value,
            source_service: None,
            region: None,
            route: None,
        }
    }

    /// Attach an emitting source service.
    pub fn with_source_service(mut self, source_service: impl Into<String>) -> Self {
        self.source_service = Some(source_service.into());
        self
    }

    /// Attach a region/locality label.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Attach a route/operation label override.
    pub fn with_route(mut self, route: impl Into<String>) -> Self {
        self.route = Some(route.into());
        self
    }

    /// Validate the event shape before recording.
    pub fn validate(&self) -> Result<()> {
        if self.timestamp_ms == 0 {
            return Err(Error::schema("usage event timestamp_ms must be non-zero"));
        }

        validate_labelish_field("subject", &self.subject)?;

        if let Some(source_service) = &self.source_service {
            validate_labelish_field("source_service", source_service)?;
        }

        if let Some(region) = &self.region {
            validate_labelish_field("region", region)?;
        }

        if let MetricKind::Custom(value) = &self.metric_kind {
            validate_labelish_field("custom metric kind", value)?;
        }

        Ok(())
    }

    /// Convert this event into recorder labels and a dimension.
    pub fn to_counter(&self, policy: &EventIngestPolicy) -> Result<UsageCounterInput> {
        self.validate()?;

        let source_service = self
            .source_service
            .as_deref()
            .unwrap_or(&policy.default_source_service);
        let attribution = match policy.subject_mode {
            EventSubjectMode::Subject => self.subject.as_str(),
            EventSubjectMode::SourceService => source_service,
        };

        let region = self.region.as_deref().unwrap_or(&policy.default_region);
        let route = self
            .route
            .as_ref()
            .map(normalize_route)
            .unwrap_or_else(|| self.metric_kind.default_route());

        Ok(UsageCounterInput {
            labels: LabelSet::new(
                self.tenant,
                attribution,
                region,
                self.metric_kind.method_label(),
                route,
            ),
            dimension: self.metric_kind.dimension(),
            value: self.value,
        })
    }
}

/// Recorder-ready event conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCounterInput {
    /// Normalized labels.
    pub labels: LabelSet,

    /// Recorder dimension.
    pub dimension: Dimension,

    /// Counter increment.
    pub value: u64,
}

/// Ingest summary for batch recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventIngestReport {
    /// Events inspected.
    pub inspected: usize,

    /// Events recorded.
    pub recorded: usize,

    /// Zero-value events skipped as no-ops.
    pub skipped_zero: usize,
}

/// Record one usage event into the recorder.
pub fn record_usage_event(
    recorder: &Recorder,
    event: &UsageEvent,
    policy: &EventIngestPolicy,
) -> Result<bool> {
    let counter = event.to_counter(policy)?;

    if counter.value == 0 {
        return Ok(false);
    }

    recorder.record(counter.labels, counter.dimension, counter.value)?;
    Ok(true)
}

/// Record many usage events into the recorder.
pub fn record_usage_events(
    recorder: &Recorder,
    events: &[UsageEvent],
    policy: &EventIngestPolicy,
) -> Result<EventIngestReport> {
    let mut report = EventIngestReport {
        inspected: events.len(),
        ..EventIngestReport::default()
    };

    for event in events {
        if record_usage_event(recorder, event, policy)? {
            report.recorded = report
                .recorded
                .checked_add(1)
                .ok_or_else(|| Error::schema("event ingest recorded counter overflow"))?;
        } else {
            report.skipped_zero = report
                .skipped_zero
                .checked_add(1)
                .ok_or_else(|| Error::schema("event ingest skipped counter overflow"))?;
        }
    }

    Ok(report)
}

fn validate_labelish_field(name: &str, value: &str) -> Result<()> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(Error::schema(format!("{name} must not be empty")));
    }

    if trimmed.len() > 160 {
        return Err(Error::schema(format!("{name} exceeds 160 bytes")));
    }

    Ok(())
}
