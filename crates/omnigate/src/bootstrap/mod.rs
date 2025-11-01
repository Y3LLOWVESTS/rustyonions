//! RO:WHAT — Bootstrap modules: API server, metrics server (via kernel), health probe helpers.
//! RO:WHY  — Keep main.rs tiny; Concerns: RES/PERF (clean layering, quick start/stop).
//! RO:INTERACTS — server.rs (axum serve), metrics_server.rs (delegates to ron-kernel), health_probe.rs.
//! RO:INVARIANTS — single writer per listener; truthful readiness; no blocking in async.

pub mod health_probe;
pub mod metrics_server;
pub mod server;
