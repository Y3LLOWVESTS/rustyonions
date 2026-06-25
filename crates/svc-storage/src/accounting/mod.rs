//! RO:WHAT — Accounting DTOs and exporter seams for svc-storage usage events.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Storage emits metering signals without becoming accounting truth.
//! RO:INTERACTS — http::routes::paid_object, accounting::exporter, future ron-accounting HTTP ingest.
//! RO:INVARIANTS — usage only; no balances; no ledger mutation; integer counters only.
//! RO:METRICS — export outcomes are observed by metrics when enabled.
//! RO:CONFIG — exporter mode/base URL/bearer/timeout live in config env helpers.
//! RO:SECURITY — no account balances/secrets; labels must stay bounded and low-cardinality.
//! RO:TEST — tests/paid_write_accounting_export.rs and web3_paid_storage_loop.rs.

pub mod exporter;

use serde::{Deserialize, Serialize};

/// Storage usage event DTO shaped to match `ron-accounting::UsageEvent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageEventDto {
    /// Event timestamp in Unix epoch milliseconds.
    pub timestamp_ms: u64,
    /// Tenant identifier.
    pub tenant: u128,
    /// Accounting subject/provider/service label.
    pub subject: String,
    /// Metric kind, e.g. bytes_stored, request_ok, pin_seconds.
    pub metric_kind: &'static str,
    /// Integer counter value.
    pub value: u64,
    /// Source service label.
    pub source_service: &'static str,
    /// Region label.
    pub region: String,
    /// Route label.
    pub route: &'static str,
}
