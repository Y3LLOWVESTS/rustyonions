//! RO:WHAT — Compact label and account-key types used by accounting counters and slices.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/SEC. Labels define who/what usage is attributed to.
//! RO:INTERACTS — normalize, recorder, slice rows, metrics label values.
//! RO:INVARIANTS — tenant is explicit; labels normalized; account rows use saturating increments.
//! RO:METRICS — service/region/method/route labels are derived from LabelSet.
//! RO:CONFIG — no direct config; caps live in normalize.rs.
//! RO:SECURITY — PII-looking route segments are templated/redacted.
//! RO:TEST — unit: recording_tests; prop: labels_prop.

use serde::{Deserialize, Serialize};

use crate::normalize::{normalize_component, normalize_method, normalize_route};

/// Tenant identifier for accounting streams.
pub type TenantId = u128;

/// Namespace for account-style rows used by downstream consumers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Namespace {
    /// Internal service namespace.
    Service,
    /// Route or endpoint namespace.
    Route,
    /// Account namespace when a caller has already mapped usage to an account.
    Account,
    /// Explicit future extension point.
    Custom(String),
}

/// Stable key for simple account-style row exports.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountKey {
    /// Namespace for this row.
    pub ns: Namespace,
    /// Stable integer key inside the namespace.
    pub id: u128,
}

impl AccountKey {
    /// Construct a new account key.
    pub fn new(ns: Namespace, id: u128) -> Self {
        Self { ns, id }
    }
}

/// Simple stable row shape from the public API spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// Row key.
    pub key: AccountKey,
    /// Non-negative increment; adders use saturating arithmetic.
    pub inc: u64,
}

/// Normalized metric/usage labels for a counter row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LabelSet {
    /// Tenant that owns this usage row.
    pub tenant: TenantId,
    /// Producing service, such as `svc_storage` or `svc_gateway`.
    pub service: String,
    /// Region or locality label, such as `local` or `us_central`.
    pub region: String,
    /// Method/source label, such as `GET`, `PUT`, or `TICK`.
    pub method: String,
    /// Normalized route or operation label.
    pub route: String,
}

impl LabelSet {
    /// Construct and normalize a label set.
    pub fn new(
        tenant: TenantId,
        service: impl AsRef<str>,
        region: impl AsRef<str>,
        method: impl AsRef<str>,
        route: impl AsRef<str>,
    ) -> Self {
        Self {
            tenant,
            service: normalize_component(service),
            region: normalize_component(region),
            method: normalize_method(method),
            route: normalize_route(route),
        }
    }
}
