//! RO:WHAT — Shared vocabulary for backpressure queues and metrics labels.
//! RO:WHY  — Avoid stringly-typed queue names scattered across the crate;
//!           keep backpressure docs and code in sync.
//! RO:INVARIANTS —
//!   * Queue names are stable across releases.
//!   * Metrics (e.g. `queue_depth{queue=...}`) use the same labels.

/// Logical name for the primary work queue feeding Micronode handlers.
///
/// This corresponds to the “work” queue discussed in `CONCURRENCY.MD`.
pub const QUEUE_WORK: &str = "work";

/// Logical name for the broadcast bus queue (used when tracking lag/backpressure).
pub const QUEUE_BUS: &str = "bus";

/// Logical name for telemetry/export queues (best-effort, drop-oldest on overflow).
pub const QUEUE_TELEMETRY: &str = "telemetry";
