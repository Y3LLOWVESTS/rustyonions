//! Bus module index.
//!
//! Layout:
//! - `bounded.rs`          : tokio::broadcast/AoS backend (default)
//! - `soa.rs`              : SoA ring backend (feature: bus_soa)
//! - `mog_edge_notify.rs`  : A1/A5 helpers (edge-triggered notify + disciplined drain)
//! - `capacity.rs`         : A3 autotune helper (feature: bus_autotune_cap)
//!
//! RO:WHAT
//!   Central selector + feature-gated helpers for the kernel bus.
//!
//! RO:WHY
//!   Keep call-sites stable while we experiment with a SoA backend.
//!   When `bus_soa` is enabled, we re-export SoA *under* a `bounded`-shaped
//!   module so existing paths (`crate::bus::bounded::Bus`) remain valid.
//!
//! RO:INVARIANTS
//!   - Public API stays stable; features are OFF-by-default.
//!   - No `unsafe` here.
//!   - `crate::bus::bounded::Bus` always exists and compiles.
//!
//! RO:INTERACTS
//!   - `bounded.rs` (default AoS) or `soa.rs` (feature=bus_soa) as the active backend.
//!   - `mog_edge_notify.rs` when feature `bus_edge_notify` is on.
//!   - `capacity.rs` when feature `bus_autotune_cap` is on.

#![allow(clippy::module_inception)] // for the bounded re-export wrapper when bus_soa is on

/// A3: Capacity autotune helper (feature-gated).
#[cfg(feature = "bus_autotune_cap")]
pub mod capacity;

#[cfg(feature = "bus_autotune_cap")]
pub use capacity::autotune_capacity;

/// Default backend (`bounded`) when SoA is NOT enabled.
#[cfg(not(feature = "bus_soa"))]
pub mod bounded;

/// Optional SoA backend (feature: bus_soa).
#[cfg(feature = "bus_soa")]
pub mod soa;

/// When `bus_soa` is ON, re-export SoA items under a `bounded`-shaped module
/// so `crate::bus::bounded::Bus` remains valid at existing call-sites.
/// We keep `pub mod soa;` above so advanced users can still import `soa::*`
/// explicitly if they want to.
#[cfg(feature = "bus_soa")]
pub mod bounded {
    pub use super::soa::*;
}

/// A1/A5 helpers (edge-triggered notify + disciplined drain).
#[cfg(feature = "bus_edge_notify")]
pub mod mog_edge_notify;

// ---------------------------------------------------------------------------
// Convenience re-exports (non-breaking):
//   - These make `use crate::bus::Bus;` work regardless of backend.
//   - Purely additive; they do not remove or rename any existing items.
// ---------------------------------------------------------------------------

pub use bounded::{Bus, Receiver};

#[cfg(feature = "bus_edge_notify")]
pub use bounded::EdgeReceiver;
