//! RO:WHAT — UTC-aligned fixed-window time helpers for accounting rollover.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. All counters share deterministic boundaries.
//! RO:INTERACTS — accounting::window, accounting::rollover, config::validate.
//! RO:INVARIANTS — fixed windows 60s..3600s; unix milliseconds; no local timezone math.
//! RO:METRICS — boundary tick callers update sealed-slice counters.
//! RO:CONFIG — accounting.window_len_s.
//! RO:SECURITY — no secrets.
//! RO:TEST — unit: rollover_tests.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::{Error, Result};

/// Minimum supported accounting window length.
pub const MIN_WINDOW_LEN_S: u32 = 60;

/// Maximum supported accounting window length.
pub const MAX_WINDOW_LEN_S: u32 = 3_600;

/// Maximum tolerated host clock skew used by boundary callers.
pub const SKEW_TOLERANCE_MS: u64 = 500;

/// Return current Unix time in milliseconds.
pub fn now_unix_ms() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::other(format!("system clock before unix epoch: {err}")))?;
    Ok(duration.as_millis() as u64)
}

/// Validate a fixed window length.
pub fn validate_window_len_s(window_len_s: u32) -> Result<()> {
    if !(MIN_WINDOW_LEN_S..=MAX_WINDOW_LEN_S).contains(&window_len_s) {
        return Err(Error::schema(format!(
            "window_len_s must be in [{MIN_WINDOW_LEN_S}, {MAX_WINDOW_LEN_S}]"
        )));
    }
    Ok(())
}

/// Return the UTC-aligned window start in milliseconds for a timestamp.
pub fn aligned_window_start_ms(timestamp_ms: u64, window_len_s: u32) -> Result<u64> {
    validate_window_len_s(window_len_s)?;
    let window_ms = u64::from(window_len_s) * 1_000;
    Ok((timestamp_ms / window_ms) * window_ms)
}

/// Return the UTC-aligned exclusive window end in milliseconds for a timestamp.
pub fn aligned_window_end_ms(timestamp_ms: u64, window_len_s: u32) -> Result<u64> {
    Ok(aligned_window_start_ms(timestamp_ms, window_len_s)? + u64::from(window_len_s) * 1_000)
}
