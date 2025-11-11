//! Admission guard: maximum request body bytes (stub).

/// Configuration for a future body cap guard.
#[derive(Debug, Clone, Copy)]
pub struct BodyCap {
    /// Maximum allowed bytes in the request body.
    pub max_bytes: u64,
}

impl Default for BodyCap {
    fn default() -> Self {
        Self { max_bytes: 1_048_576 } // 1 MiB
    }
}

impl BodyCap {
    /// Create a new cap with the given maximum.
    pub fn new(max_bytes: u64) -> Self {
        Self { max_bytes }
    }
}
