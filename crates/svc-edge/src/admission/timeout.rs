//! Admission guard: per-request timeout (stub).

use std::time::Duration;

/// Timeout configuration (placeholder).
#[derive(Debug, Clone, Copy)]
pub struct Timeout {
    /// Maximum request processing time.
    pub duration: Duration,
}

impl Default for Timeout {
    fn default() -> Self {
        Self { duration: Duration::from_secs(5) }
    }
}
