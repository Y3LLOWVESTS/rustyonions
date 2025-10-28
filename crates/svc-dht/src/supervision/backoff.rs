//! RO:WHAT — Exponential backoff with jitter
//! RO:WHY — Prevents stampedes; Concerns: RES/PERF
pub fn next(prev_ms: u64) -> u64 {
    (prev_ms.saturating_mul(2)).min(30_000)
}
