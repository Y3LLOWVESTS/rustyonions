//! RO:WHAT — Byte and size parsing helpers for svc-rewarder caps.
//! RO:WHY — Pillar 12; Concerns: HARDENING/PERF. Centralizes 1MiB body and decompression cap validation.
//! RO:INTERACTS — config::validate, http body-limit wiring, tests.
//! RO:INVARIANTS — checked arithmetic only; no allocation based on untrusted lengths.
//! RO:METRICS — rejected sizes are counted by callers.
//! RO:CONFIG — parses max_body_bytes-like strings.
//! RO:SECURITY — malformed sizes fail closed.
//! RO:TEST — config validation tests.

use crate::{Result, RewarderError};

/// Protocol default request cap for this crate: 1 MiB.
pub const DEFAULT_MAX_BODY_BYTES: u64 = 1_048_576;

/// Parse a small human size string such as `1024`, `1KiB`, `1MiB`, or `8MiB`.
pub fn parse_size_bytes(input: &str) -> Result<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RewarderError::Config("size cannot be empty".into()));
    }
    let lower = trimmed.to_ascii_lowercase();
    let (num, multiplier) = if let Some(prefix) = lower.strip_suffix("kib") {
        (prefix.trim(), 1024_u64)
    } else if let Some(prefix) = lower.strip_suffix("kb") {
        (prefix.trim(), 1000_u64)
    } else if let Some(prefix) = lower.strip_suffix("mib") {
        (prefix.trim(), 1024_u64 * 1024)
    } else if let Some(prefix) = lower.strip_suffix("mb") {
        (prefix.trim(), 1000_u64 * 1000)
    } else {
        (trimmed, 1_u64)
    };
    let value = num
        .parse::<u64>()
        .map_err(|_| RewarderError::Config(format!("invalid size: {input}")))?;
    value
        .checked_mul(multiplier)
        .ok_or_else(|| RewarderError::Config(format!("size overflows u64: {input}")))
}
