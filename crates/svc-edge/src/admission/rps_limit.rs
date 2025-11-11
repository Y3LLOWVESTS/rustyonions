//! Admission guard: requests-per-second limiter (stub).

/// Simple RPS limiter configuration (placeholder).
#[derive(Debug, Clone, Copy)]
pub struct RpsLimit {
    /// Approximate target request rate (per second).
    pub rps: u64,
}

impl Default for RpsLimit {
    fn default() -> Self {
        Self { rps: 1000 }
    }
}
