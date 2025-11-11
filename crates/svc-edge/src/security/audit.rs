//! Security audit trail placeholder (stub).

/// Minimal audit event (placeholder).
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Category (e.g., "admission", "security", "policy").
    pub category: String,
    /// Short message.
    pub message: String,
}

/// Audit sink (no-op).
#[derive(Debug, Default, Clone)]
pub struct Auditor;

impl Auditor {
    /// Record an audit event (no-op).
    pub fn record(&self, _evt: AuditEvent) {
        // Future: write to structured log / metrics.
    }
}
