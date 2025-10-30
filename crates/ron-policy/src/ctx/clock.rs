//! RO:WHAT — Clock trait for deterministic tests and prod time access.
//!
//! RO:WHY  — Keep engine free of direct time deps; easy to mock.

pub trait Clock {
    fn now_ms(&self) -> u64;
}

#[derive(Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        u64::try_from(ms).unwrap_or(u64::MAX)
    }
}
