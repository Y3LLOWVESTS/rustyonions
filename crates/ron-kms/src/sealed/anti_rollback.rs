#![cfg(feature = "soft-seal")]
#![forbid(unsafe_code)]

// RO:WHAT  Placeholder anti-rollback policy hook.
// RO:NOW   Always returns true (accept). Wire to a caller-provided policy later.
pub fn check_ts(_ts_ms: i64) -> bool {
    true
}
