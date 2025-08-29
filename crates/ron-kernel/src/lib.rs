#![forbid(unsafe_code)]

pub mod bus;
pub mod metrics;
pub mod transport;

pub use bus::{Bus, KernelEvent};
pub use metrics::{HealthState, Metrics};

/// Await Ctrl-C for graceful shutdown.
pub async fn wait_for_ctrl_c() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!(%err, "ctrl_c signal error");
    }
}
