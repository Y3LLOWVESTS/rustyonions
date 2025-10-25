//! RO:WHAT
//!   Capacity autotune helper for the bounded bus ring.
//!
//! RO:WHY
//!   Picking a too-large cap is cache-hostile; too-small can increase churn/drops.
//!   This feature-gated helper chooses a sweet-spot cap from expected subscriber
//!   count with guardrails and observability (Prometheus counters/gauges).
//!
//! RO:INTERACTS
//!   - Used by examples (kernel_demo) and callers that want reasonable defaults.
//!   - Exposes Prometheus metrics via the *default* registry.
//!
//! RO:INVARIANTS
//!   - Public kernel API remains frozen; this is an internal helper.
//!   - When feature `bus_autotune_cap` is OFF, callers should not reference it.
//!   - Caps returned are restricted to {64, 128, 256} unless explicitly overridden.
//!
//! RO:TESTS
//!   - Unit: mapping for key N (0,1,4,5,16,17,64,128)
//!   - Property: monotone in N; override respected; warnings on >256
//!
//! RO:SAFETY
//!   - No `unsafe`. Pure computation + metrics.
//!
//! RO:METRICS
//!   - `bus_autotune_warn_total{reason="cap_gt_256"}`
//!   - `bus_cap_selected` (gauge)

#![cfg(feature = "bus_autotune_cap")]

use once_cell::sync::Lazy;
use prometheus::{opts, register_gauge, register_int_counter_vec, Gauge, IntCounterVec};

/// Warn counter for guardrails.
static AUTOTUNE_WARN_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        opts!("bus_autotune_warn_total", "Autotune guardrail warnings"),
        &["reason"]
    )
    .expect("register bus_autotune_warn_total")
});

/// Last selected cap (observability).
static BUS_CAP_SELECTED: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "bus_cap_selected",
        "Current bus capacity selected by autotune"
    )
    .expect("register bus_cap_selected")
});

/// Choose a cache-friendly capacity from an expected subscriber count `expected_subs`.
/// If `override_cap` is provided, it always wins (and may be any power-of-two the caller desires).
/// Guardrail: when the *effective* cap exceeds 256, we emit a warning counter.
///
/// Mapping (when `override_cap` is `None`):
///   - N ≤ 4   → 64
///   - N ≤ 16  → 128
///   - else    → 256 (warn if caller later uses >256)
pub fn autotune_capacity(expected_subs: usize, override_cap: Option<usize>) -> usize {
    let cap = match override_cap {
        Some(c) => c,
        None => {
            if expected_subs <= 4 {
                64
            } else if expected_subs <= 16 {
                128
            } else {
                256
            }
        }
    };

    if cap > 256 {
        AUTOTUNE_WARN_TOTAL.with_label_values(&["cap_gt_256"]).inc();
    }

    BUS_CAP_SELECTED.set(cap as f64);
    cap
}

/// Helper for tests/benches to reset gauge (kept for internal use).
#[cfg(test)]
pub fn __test_reset_metrics() {
    BUS_CAP_SELECTED.set(0.0);
}
