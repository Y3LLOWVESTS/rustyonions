//! RO:WHAT — Low-cardinality lifecycle events for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/GOV. Reward runs need auditable event breadcrumbs without high-cardinality metrics.
//! RO:INTERACTS — bus::RewarderBus, http handlers, future audit integration.
//! RO:INVARIANTS — no raw payloads or secrets; event names are stable.
//! RO:METRICS — events are separate from metrics; future bridge may count them.
//! RO:CONFIG — none.
//! RO:SECURITY — no auth tokens or raw snapshots in events.
//! RO:TEST — compile coverage through handlers.

use serde::{Deserialize, Serialize};

/// Rewarder event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type")]
pub enum RewarderEvent {
    /// Run started.
    RunStarted { epoch_id: String, run_key: String },
    /// Run completed.
    RunCompleted {
        epoch_id: String,
        run_key: String,
        status: String,
    },
    /// Run quarantined.
    RunQuarantined {
        epoch_id: String,
        run_key: String,
        reason: String,
    },
}
