//! Simple exponential backoff calculator (stub).

use std::time::Duration;

/// Exponential backoff parameters (placeholder).
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    /// Base delay.
    pub base: Duration,
    /// Maximum delay.
    pub max: Duration,
    /// Multiplier per attempt.
    pub factor: f64,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(10),
            max: Duration::from_secs(2),
            factor: 2.0,
        }
    }
}

impl Backoff {
    /// Compute delay for `attempt` (0-based).
    pub fn delay(&self, attempt: u32) -> Duration {
        let ms = (self.base.as_millis() as f64) * self.factor.powi(attempt as i32);
        let d = Duration::from_millis(ms as u64);
        if d > self.max { self.max } else { d }
    }
}
