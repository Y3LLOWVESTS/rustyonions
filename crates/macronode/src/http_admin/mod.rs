//! RO:WHAT — HTTP admin/observability plane for Macronode.
//! RO:WHY  — Expose `/version`, `/healthz`, `/readyz`, `/metrics`, and basic admin APIs.
//! RO:INTERACTS —
//!   - `AppState` for config + probes.
//!   - `observability::metrics` for Prometheus encoding.

pub mod handlers;
pub mod middleware;
pub mod router;
