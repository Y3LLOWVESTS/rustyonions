//! Cooperative shutdown helpers (stub).

/// A cooperative shutdown token (stub).
#[derive(Debug, Clone, Default)]
pub struct ShutdownToken {
    /// Whether shutdown was requested.
    pub requested: bool,
}

impl ShutdownToken {
    /// Request shutdown (no-op).
    pub fn request(&mut self) {
        self.requested = true;
    }
    /// Check whether shutdown was requested.
    pub fn is_requested(&self) -> bool {
        self.requested
    }
}
