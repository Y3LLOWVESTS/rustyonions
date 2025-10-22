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
- Warn if chosen capacity >256 (cache-hostile territory for typical workloads).

Safety:
- No panics. Always returns >= 2. Overrides are rounded to power-of-two and clamped.
*/

#![allow(dead_code)] // until wired in by a builder

use core::cmp::{max, min};

/// Returns a recommended capacity given the expected subscriber count
/// and an optional explicit override.
///
/// Feature gating:
/// - `bus_autotune_cap` **enabled**: use plateau heuristic when `override_cap` is None.
/// - Feature **disabled**: honor override (normalized) or fall back to 128.
///
/// Invariants:
/// - Never returns < 2.
/// - Overrides are rounded to the next power-of-two and clamped to [2, 65_536].
#[allow(unused_variables)]
pub fn autotune_capacity(expected_subs: usize, override_cap: Option<usize>) -> usize {
    cfg_if::cfg_if! {
        if #[cfg(feature = "bus_autotune_cap")] {
            // If caller provides override, respect it after normalization.
            if let Some(c) = override_cap {
                return normalize_override(c);
            }

            // Plateau heuristic tuned for cache-local rings.
            let chosen = if expected_subs <= 4 {
                64
            } else if expected_subs <= 16 {
                128
            } else {
                256
            };

            if chosen > 256 {
                tracing::warn!(
                    cap = chosen,
                    expected_subs,
                    "bus_autotune_cap: capacity >256 is likely cache-hostile; consider 64/128/256 unless proven otherwise"
                );
            }
            chosen
        } else {
            // Feature disabled: be conservative/predictable.
            match override_cap {
                Some(c) => normalize_override(c),
                None => 128,
            }
        }
    }
}

/// Normalize a caller-provided override:
/// - minimum of 2
/// - clamped to 65_536
/// - rounded up to next power-of-two
#[inline]
fn normalize_override(cap: usize) -> usize {
    let bounded = min(max(cap, 2), 65_536);
    next_pow2(bounded)
}

#[inline]
fn next_pow2(n: usize) -> usize {
    if n <= 2 { return 2; }
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
        // with feature OFF default is 128.
        let cap = autotune_capacity(8, None);
        assert!(cap >= 64);
    }

    #[test]
    fn heuristic_plateaus_are_expected_when_enabled() {
        // These assertions hold for feature-enabled builds; for feature-off they
        // still validate general bounds.
        let small = autotune_capacity(1, None);
        let mid   = autotune_capacity(8, None);
        let big   = autotune_capacity(32, None);

        assert!(small >= 64, "small expected ≥64, got {}", small);
        assert!(mid   >= 64 && mid <= 256, "mid in [64,256], got {}", mid);
        assert!(big   >= 128, "big expected ≥128, got {}", big);
    }
}
