//! Parse human-readable sizes like "64k", "10MiB" (stub).
//!
//! Accepted suffixes (case-insensitive; decimal only for now):
//! - k, m, g, t  → 10^3 steps.

/// Parse a simple size string. Returns bytes on success.
///
/// Examples: "0", "64k", "10m". Binary units and MiB/GiB will arrive later.
pub fn parse_decimal_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, suf) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let mut n: u64 = num.parse().ok()?;
    let suffix = suf.trim().to_ascii_lowercase();
    n *= match suffix.as_str() {
        "" => 1,
        "k" => 1_000,
        "m" => 1_000_000,
        "g" => 1_000_000_000,
        "t" => 1_000_000_000_000,
        _ => return None,
    };
    Some(n)
}
