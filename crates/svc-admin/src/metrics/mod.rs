// crates/svc-admin/src/metrics/mod.rs
//
// RO:WHAT — Metrics/observability modules for svc-admin.
// RO:WHY  — Group Prometheus integration and in-memory facet sampling behind
//          a small, well-namespaced surface.
// RO:INTERACTS — prometheus, router, server bootstrap, nodes::registry.
// RO:INVARIANTS —
//   - Submodules own their metric registration (OnceLock + default registry).
//   - No business logic lives here; only observability plumbing.

pub mod actions;
pub mod facet;
pub mod prometheus_bridge;
pub mod sampler;
