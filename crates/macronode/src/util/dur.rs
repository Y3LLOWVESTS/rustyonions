//! RO:WHAT — Duration helpers for Macronode.
//! RO:WHY  — Avoid sprinkling raw `Duration::from_secs` calls and magic
//!           numbers (like 1000) throughout the codebase.
//! RO:INVARIANTS —
//!   - Helpers are thin wrappers over `std::time::Duration`.
//!   - Parsing helpers never panic; they return `Result`.

#![allow(dead_code)]

use std::num::ParseIntError;
use std::time::Duration;

/// Construct a duration from whole milliseconds.
#[must_use]
pub const fn millis(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

/// Construct a duration from whole seconds.
#[must_use]
pub const fn seconds(secs: u64) -> Duration {
    Duration::from_secs(secs)
}

/// Construct a duration from whole minutes.
#[must_use]
pub const fn minutes(mins: u64) -> Duration {
    Duration::from_secs(mins * 60)
}

/// Parse a duration expressed as whole seconds (e.g. from an env var).
///
/// Whitespace is trimmed; invalid inputs yield a `ParseIntError`.
pub fn parse_seconds(input: &str) -> Result<Duration, ParseIntError> {
    let secs: u64 = input.trim().parse()?;
    Ok(Duration::from_secs(secs))
}
