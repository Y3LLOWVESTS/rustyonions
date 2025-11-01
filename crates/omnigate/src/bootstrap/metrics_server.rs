//! RO:WHAT — Placeholder to document metrics server behavior (served by ron-kernel Metrics).
//! RO:WHY  — Keep parity with TODO structure; Concerns: GOV/OBS.
//! RO:INTERACTS — ron_kernel::Metrics::serve() started in App::build().
//! RO:INVARIANTS — none here; admin plane lives in kernel exporter.

/// Metrics server is started in `App::build()` via `ron_kernel::Metrics::serve`.
pub struct MetricsServer;
