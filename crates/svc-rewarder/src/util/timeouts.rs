//! RO:WHAT — Duration parsing and deadline helpers for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF. All outbound and compute paths use bounded time budgets.
//! RO:INTERACTS — config validation, main bootstrap, future wallet/accounting clients.
//! RO:INVARIANTS — invalid durations fail closed; zero timeouts are rejected by validation.
//! RO:METRICS — timeout errors are counted by callers.
//! RO:CONFIG — parses read_timeout/write_timeout/idle_timeout and rewarder durations.
//! RO:SECURITY — no secret handling.
//! RO:TEST — config validation tests.

use std::time::Duration;

use crate::{Result, RewarderError};

/// Parse simple duration strings like `500ms`, `5s`, `2m`, `1h`, or a raw millisecond number.
pub fn parse_duration(input: &str) -> Result<Duration> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RewarderError::Config("duration cannot be empty".into()));
    }
    let lower = trimmed.to_ascii_lowercase();
    let (num, multiplier_ms) = if let Some(prefix) = lower.strip_suffix("ms") {
        (prefix.trim(), 1_u64)
    } else if let Some(prefix) = lower.strip_suffix('s') {
        (prefix.trim(), 1_000_u64)
    } else if let Some(prefix) = lower.strip_suffix('m') {
        (prefix.trim(), 60_000_u64)
    } else if let Some(prefix) = lower.strip_suffix('h') {
        (prefix.trim(), 3_600_000_u64)
    } else {
        (trimmed, 1_u64)
    };
    let value = num
        .parse::<u64>()
        .map_err(|_| RewarderError::Config(format!("invalid duration: {input}")))?;
    let millis = value
        .checked_mul(multiplier_ms)
        .ok_or_else(|| RewarderError::Config(format!("duration overflows u64: {input}")))?;
    Ok(Duration::from_millis(millis))
}
