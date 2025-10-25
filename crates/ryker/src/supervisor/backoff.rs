//! RO:WHAT — Decorrelated jitter backoff calculator.
//! RO:WHY  — Avoid lockstep thundering herds; Concerns: RES.
//! RO:INVARIANTS — cap respected; base>0; deterministic bounds.

use rand::{rngs::StdRng, Rng, SeedableRng};

pub fn decorrelated_jitter(base_ms: u64, cap_ms: u64, prev_ms: u64, seed: u64) -> u64 {
    let mut rng = StdRng::seed_from_u64(seed ^ prev_ms);
    let next = (base_ms as f64).max((prev_ms as f64 * 3.0).min(cap_ms as f64));
    let jitter = rng.random_range(base_ms..=next as u64);
    jitter.min(cap_ms)
}
