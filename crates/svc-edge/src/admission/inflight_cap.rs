//! Admission guard: cap concurrent in-flight requests (stub).

/// In-flight concurrency cap (placeholder).
#[derive(Debug, Clone, Copy)]
pub struct InflightCap {
    /// Maximum concurrent requests the service will process.
    pub max: usize,
}

impl Default for InflightCap {
    fn default() -> Self {
        Self { max: 256 }
    }
}
