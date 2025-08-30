// crates/ron-kernel/src/lib.rs

#![forbid(unsafe_code)]

pub mod bus;
pub mod metrics;
pub mod transport;
pub mod config;

pub use bus::{Bus, KernelEvent};
pub use metrics::{HealthState, Metrics};
pub use config::Config;

/// Minimal capability/secret types expected by some bins.
/// These are intentionally small and can be expanded later.
#[derive(Clone, Debug)]
pub struct Capabilities {
    pub amnesia: AmnesiaMode,
}
impl Default for Capabilities {
    fn default() -> Self {
        Self { amnesia: AmnesiaMode::Disabled }
    }
}

#[derive(Clone, Debug)]
pub enum AmnesiaMode {
    Disabled,
    Enabled,
}
impl AmnesiaMode {
    pub fn new(enabled: bool) -> Self {
        if enabled { AmnesiaMode::Enabled } else { AmnesiaMode::Disabled }
    }
}
impl Default for AmnesiaMode {
    fn default() -> Self { AmnesiaMode::Disabled }
}

#[derive(Clone, Debug, Default)]
pub struct Secrets;

/// Cross-platform CTRL-C waiter used by demo binaries.
pub async fn wait_for_ctrl_c() -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await.map_err(|e| anyhow::anyhow!(e))
}
