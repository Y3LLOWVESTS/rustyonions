//! RO:WHAT — Metrics module index and re-exports for RON-CORE.
//! RO:WHY  — Centralize exporter + health + readiness; expose optional TLS buffering at `metrics::buffer`.

pub mod exporter;
pub mod health;
pub mod readiness;

// Declare the submodule *unconditionally* so the name `crate::metrics::buffer` always exists.
// The file itself is feature-gated internally, so this is safe in all builds.
pub mod buffer;

// Re-export the primary metrics type so call-sites can use `crate::metrics::Metrics`.
pub use exporter::Metrics;

// Convenience re-exports (common call-sites).
pub use health::HealthState;
pub use readiness::Readiness;
