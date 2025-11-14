//! Metrics wiring for audit operations.
//!
//! This is deliberately a no-op placeholder for the first seed. A later
//! pass can integrate with `ron-metrics` / `prometheus` without changing
//! the public type name.

/// Placeholder metrics handle for audit operations.
///
/// In its current form this does not record anything; it exists so host
/// crates can thread a metrics handle through without feature gating.
#[derive(Debug, Clone, Default)]
pub struct AuditMetrics;

impl AuditMetrics {
    /// Create a new, no-op metrics handle.
    pub fn new() -> Self {
        Self
    }

    /// Record a successful append operation (no-op).
    pub fn on_append_ok(&self) {}

    /// Record a failed append operation (no-op).
    pub fn on_append_err(&self) {}
}
