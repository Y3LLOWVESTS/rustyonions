//! RO:WHAT — Host-constructed configuration for Bus
//! RO:WHY  — Capacity & observability knobs are fixed at construction (cutover by rebuild)
//! RO:INTERACTS — Used by Bus::new(cfg); host is responsible for reading env/files/flags
//! RO:INVARIANTS — capacity >= 2 and reasonable upper bound; no runtime mutation
//! RO:TEST — Unit: validate bounds; Integration: capacity_cutover
//! RO:NOTE — Marked #[non_exhaustive] to allow additive evolution without SemVer breaks.
//!           Because of this, external crates must use the builder helpers rather than
//!           struct-literal syntax.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Host-facing configuration for constructing a [`Bus`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BusConfig {
    /// Bounded broadcast buffer size (messages).
    pub capacity: u32,
    /// Host WARN throttling (per minute) for overflow logs; library does not log.
    pub overflow_warn_rate_per_min: u32,
    /// Optional namespace the host may use for metrics; library is metrics-neutral.
    pub metrics_namespace: String,
    /// If true, hosts may attach amnesia={on|off} label to metrics.
    pub emit_amnesia_label: bool,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            capacity: 256,
            overflow_warn_rate_per_min: 60,
            metrics_namespace: "ronbus".to_string(),
            emit_amnesia_label: true,
        }
    }
}

impl BusConfig {
    /// Create with default values (same as `Default::default()`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the bounded capacity (messages).
    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.capacity = capacity;
        self
    }

    /// Set the overflow warn throttle (per minute).
    pub fn with_overflow_warn_rate_per_min(mut self, rate: u32) -> Self {
        self.overflow_warn_rate_per_min = rate;
        self
    }

    /// Set the metrics namespace hint (library remains metrics-neutral).
    pub fn with_metrics_namespace<S: Into<String>>(mut self, ns: S) -> Self {
        self.metrics_namespace = ns.into();
        self
    }

    /// Toggle amnesia label emission hint.
    pub fn with_emit_amnesia_label(mut self, yes: bool) -> Self {
        self.emit_amnesia_label = yes;
        self
    }

    /// Validate bounds & basic invariants.
    pub fn validate(&self) -> Result<(), String> {
        if self.capacity < 2 {
            return Err("capacity must be >= 2".into());
        }
        if self.capacity > (1 << 20) {
            // Keep memory sane; Tokio broadcast alloc is O(capacity)
            return Err("capacity too large; must be <= 1,048,576".into());
        }
        Ok(())
    }
}
