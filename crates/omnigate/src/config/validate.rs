//! RO:WHAT — Validate Config invariants (caps, limits).
//! RO:WHY  — Prevent misconfig from violating protocol/HTTP limits; Concerns: GOV/SEC.
//! RO:INVARIANTS — OAP max_frame=1MiB; body cap ≤1MiB; decompression ≤10x (to be added when body handling lands).

use super::Config;

pub fn validate(_cfg: &Config) -> anyhow::Result<()> {
    // Add concrete checks as data-plane routes land (body caps, timeouts, inflight).
    Ok(())
}
