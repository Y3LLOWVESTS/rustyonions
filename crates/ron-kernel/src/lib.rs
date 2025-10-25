//! # ron-kernel — microkernel core
//!
//! RO:WHAT
//!   Crate root for the RustyOnions microkernel. Exposes the frozen public API:
//!   `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, and `wait_for_ctrl_c()`.
//!
//! RO:WHY
//!   Provide lifecycle/supervision, readiness gates, config hot-reload, bounded event bus,
//!   and canonical observability surfaces (/metrics, /healthz, /readyz) for nodes.
//!
//! RO:INVARIANTS
//!   - Public API is semver-guarded; perf toggles live behind features and default OFF.
//!   - Readiness contract: `/readyz` returns 503 until BOTH (config_loaded && services_healthy).
//!   - Concurrency: no locks across `.await`; bounded channels; one receiver per task.
//!   - Amnesia mode surfaced via metrics (`amnesia_mode` gauge) and events.
//!
//! See: `examples/kernel_demo` for an integration sanity run.
#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// A3 helper — capacity autotune — re-exported at crate root for stable imports in tests/benches.
#[cfg(feature = "bus_autotune_cap")]
pub use crate::bus::autotune_capacity;

// -----------------------------------------------------------------------------
// Internal structure
// -----------------------------------------------------------------------------

pub mod internal {
    pub mod types;
}

pub mod amnesia;

pub mod events; // KernelEvent enum
pub mod shutdown; // wait_for_ctrl_c()

// IMPORTANT: use the directory module so we pick up `bus/mod.rs` and its feature wiring.
// (Previously this was an inline `pub mod bus { ... }`, which prevented `bus/mod.rs` from loading.)
pub mod bus;

pub mod metrics;

// Use your existing config module (which itself may declare submodules like watcher/validation)
pub mod config;

// Supervision
pub mod supervisor {
    pub mod backoff;
    pub mod child;
    pub mod lifecycle;
}

// -----------------------------------------------------------------------------
// Frozen public API re-exports (SemVer-guarded)
// -----------------------------------------------------------------------------
pub use crate::bus::bounded::Bus;
pub use crate::config::Config;
pub use crate::events::KernelEvent;
pub use crate::metrics::exporter::Metrics;
pub use crate::metrics::health::HealthState;
pub use crate::shutdown::wait_for_ctrl_c;

// If you maintain an experimental MOG helper module at crate root, keep this.
// If it doesn't exist in your tree, comment/remove the next line to avoid compile errors.
pub mod mog_autotune;
