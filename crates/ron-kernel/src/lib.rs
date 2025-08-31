#![forbid(unsafe_code)]

pub mod bus;
pub mod config;
pub mod metrics;
pub mod transport;   // transport available at the crate root
pub mod cancel;      // shutdown token wrapper
pub mod supervisor;  // <— NEW: supervisor module

// Re-export stable surface (kept compatible with existing bins)
pub use crate::config::Config;
pub use crate::metrics::{HealthState, Metrics};
pub use bus::Bus;

/// Kernel-wide event type (public at crate root).
#[derive(Clone, Debug)]
pub enum KernelEvent {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    // NOTE: existing tree uses a `reason` field; we keep it for compatibility.
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}

/// Graceful Ctrl-C helper.
pub async fn wait_for_ctrl_c() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}
