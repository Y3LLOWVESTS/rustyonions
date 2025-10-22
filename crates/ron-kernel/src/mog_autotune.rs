/*!
MOG A3 — Capacity Autotune + Guardrails (feature: `bus_autotune_cap`)

Purpose:
- Provide a safe, side-effect free helper to pick a ring/channel capacity from the expected load.
- This does NOT mutate global state; callers must opt-in to use it.

Integration:
- Call `autotune_capacity(expected_subs, override_cap)` from your Bus builder.
- If `override_cap` is `Some`, it wins (after normalization).
- Otherwise (feature ON) we choose plateaus: <=4 → 64, <=16 → 128, else → 256.
- Feature OFF: conservative 128 default.

Observability:
- Warn if finalized capacity >256 (cache-hostile territory for typical workloads).

Safety:
- No panics. Always returns >= 2. Overrides are rounded to power-of-two and clamped.
*/

#![allow(dead_code)] // until wired in by a builder

use core::cmp::{max, min};

/// Guardrail constants.
const MIN_CAP: usize = 2;
const MAX_CAP: usize = 65_536;

// Plateau levels (power-of-two, cache-friendly)
const PLATEAU_SMALL: usize = 64;
const PLATEAU_MED:   usize = 128;
const PLATEAU_LARGE: usize = 256;

/// Returns a recommended capacity given the expected subscriber count
/// and an optional explicit override.
///
/// Feature gating:
/// - `bus_autotune_cap` **enabled**: use plateau heuristic when `override_cap` is None.
/// - Feature **disabled**: honor override (normalized) or fall back to 128.
///
/// Invariants:
/// - Never returns < 2.
/// - Final result is rounded to the next power-of-two and clamped to [2, 65_536].
#[allow(unused_variables)]
pub fn autotune_capacity(expected_subs: usize, override_cap: Option<usize>) -> usize {
    // 1) If caller provides override, it wins after normalization & guardrails.
    if let Some(c) = override_cap {
        return finalize_cap(c);
    }

    // 2) Otherwise pick via heuristic (if feature enabled), or a conservative default.
    let raw = {
        cfg_if::cfg_if! {
            if #[cfg(feature = "bus_autotune_cap")] {
                if expected_subs <= 4 {
                    PLATEAU_SMALL
                } else if expected_subs <= 16 {
                    PLATEAU_MED
                } else {
                    PLATEAU_LARGE
                }
            } else {
                // Feature disabled: predictable default before guardrails.
                128
            }
        }
    };

    // 3) Apply guardrails uniformly (also handles any future changes safely).
    finalize_cap(raw)
}

/// Apply guardrails to any incoming capacity (from override or heuristic):
/// - minimum of 2
/// - clamped to 65_536
/// - rounded up to next power-of-two
/// - warns if result > 256 (likely cache-hostile for typical workloads)
#[inline(always)]
fn finalize_cap(cap: usize) -> usize {
    let bounded = min(max(cap, MIN_CAP), MAX_CAP);
    let pow2 = next_pow2(bounded);

    if pow2 > PLATEAU_LARGE {
        tracing::warn!(
            cap = pow2,
            "bus_autotune_cap: capacity >256 is likely cache-hostile; consider 64/128/256 unless proven otherwise"
        );
    }
    pow2
}

/// Round up to the next power-of-two with a floor of 2.
#[inline(always)]
fn next_pow2(n: usize) -> usize {
    if n <= MIN_CAP { return MIN_CAP; }
    // `next_power_of_two` on usize is well-defined for n>0 and will not overflow
    // under our clamp (MAX_CAP = 65_536).
    n.next_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_is_normalized_pow2_and_clamped() {
        assert_eq!(autotune_capacity(0, Some(1)), 2);
        assert_eq!(autotune_capacity(0, Some(3)), 4);
        assert_eq!(autotune_capacity(0, Some(64)), 64);
        assert_eq!(autotune_capacity(0, Some(65_000)), 65_536);
        assert_eq!(autotune_capacity(0, Some(100_000)), 65_536);
    }

    #[test]
    fn default_when_disabled_is_reasonable() {
        // This holds regardless of feature; with feature ON, values are >=64,
        // with feature OFF default is 128 before guardrails.
        let cap = autotune_capacity(8, None);
        assert!(cap >= 64);
        assert!(cap <= MAX_CAP);
    }

    #[test]
    fn heuristic_plateaus_are_expected_when_enabled() {
        // These assertions hold for feature-enabled builds; for feature-off they
        // still validate general bounds.
        let small = autotune_capacity(1, None);
        let mid   = autotune_capacity(8, None);
        let big   = autotune_capacity(32, None);

        assert!(small >= PLATEAU_SMALL, "small expected ≥64, got {}", small);
        assert!(mid   >= PLATEAU_SMALL && mid <= PLATEAU_LARGE, "mid in [64,256], got {}", mid);
        assert!(big   >= PLATEAU_MED, "big expected ≥128, got {}", big);
        assert!(big   <= MAX_CAP);
    }
}
