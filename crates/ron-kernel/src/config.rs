// crates/ron-kernel/src/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Kernel/runtime configuration (basic fields; expand as needed).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub data_dir: Option<PathBuf>,
    pub amnesia: Option<Amnesia>,
    pub transport: Option<TransportSection>,
    pub overlay_addr: Option<String>,
    pub idle_timeout: Option<Duration>,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Amnesia {
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransportSection {
    pub max_conns: Option<usize>,
    pub read_timeout_secs: Option<u64>,
    pub write_timeout_secs: Option<u64>,
    pub idle_timeout_secs: Option<u64>,
}

/// Submodule path used by some bins: `ron_kernel::config::spawn_config_watcher`.
/// We provide a no-op watcher to satisfy imports; wire it up later to `notify` if desired.
pub async fn spawn_config_watcher() -> JoinHandle<()> {
    tokio::spawn(async move {
        // no-op watcher
    })
}
