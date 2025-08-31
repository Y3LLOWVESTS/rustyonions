#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::info;

/// Minimal kernel Config per blueprint (extend later).
#[derive(Clone, Debug)]
pub struct Config {
    /// Version increments on any config change that affects behavior.
    pub version: u64,

    /// Address for the admin server (/metrics, /healthz, /readyz).
    pub admin_addr: SocketAddr,

    /// Default idle timeout for transports.
    pub idle_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            admin_addr: "127.0.0.1:9090".parse().unwrap(),
            idle_timeout: Duration::from_secs(300),
        }
    }
}

/// Spawn a config watcher (stub for M0).
///
/// Per blueprint, a future version validates new config, emits
/// `KernelEvent::ConfigUpdated{version}`, and rolls back on failure.
/// For M0, we provide a harmless, running stub to satisfy callers.
pub fn spawn_config_watcher() -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("config watcher (stub) started");
        // No-op loop to keep the task alive in M0. Replace with real FS watch/HUP later.
        // We purposely avoid busy loops.
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    })
}
