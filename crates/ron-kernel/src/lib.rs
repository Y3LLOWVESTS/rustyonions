// FILE: crates/ron-kernel/src/lib.rs
#![forbid(unsafe_code)]
#![doc = include_str!("../docs/kernel_events.md")]

pub mod bus;
pub mod config;
pub mod metrics;
pub mod transport;
pub mod cancel;
pub mod supervisor;
pub mod overlay;

use serde::{Deserialize, Serialize};

// Re-export the stable surface (no self re-export of wait_for_ctrl_c)
pub use crate::config::Config;
pub use crate::metrics::{HealthState, Metrics};
pub use bus::Bus;

/// Kernel-wide event type (public at crate root).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelEvent {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    // Keep 'reason' for compatibility and test snapshots.
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}

/// Graceful Ctrl-C helper.
pub async fn wait_for_ctrl_c() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}
