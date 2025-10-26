//! RO:WHAT — Config loader/validator
use anyhow::{anyhow, bail, Result};
use std::{net::SocketAddr, time::Duration};

#[derive(Clone, Debug)]
pub struct Admin {
    pub http_addr: SocketAddr,
    pub metrics_addr: SocketAddr,
}

#[derive(Clone, Debug)]
pub struct TransportCfg {
    pub addr: SocketAddr,
    pub name: &'static str,
    pub read_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_conns: usize,
    // TLS/QUIC/Tor knobs can be added here and mapped to ron-transport features
}

#[derive(Clone, Debug)]
pub struct Config {
    pub admin: Admin,
    pub transport: TransportCfg,
    pub oap_max_frame: usize,
    pub send_window_frames: u32,
    pub recv_window_frames: u32,
    pub amnesia: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            admin: Admin {
                http_addr: "127.0.0.1:9600".parse().unwrap(),
                metrics_addr: "127.0.0.1:9601".parse().unwrap(),
            },
            transport: TransportCfg {
                addr: "127.0.0.1:9700".parse().unwrap(),
                name: "svc-overlay",
                read_timeout: Duration::from_secs(5),
                idle_timeout: Duration::from_secs(30),
                max_conns: 1024,
            },
            oap_max_frame: 1 << 20,
            send_window_frames: 16,
            recv_window_frames: 16,
            amnesia: false,
        }
    }
}

impl Config {
    /// Minimal env loader; expand per your CONFIG.MD later.
    pub fn from_env_and_cli() -> Result<Self> {
        let mut c = Self::default();
        if let Ok(addr) = std::env::var("SVC_OVERLAY_HTTP_ADDR") {
            c.admin.http_addr = addr.parse().map_err(|e| anyhow!("bad http addr: {e}"))?;
        }
        if let Ok(addr) = std::env::var("SVC_OVERLAY_METRICS_ADDR") {
            c.admin.metrics_addr = addr.parse().map_err(|e| anyhow!("bad metrics addr: {e}"))?;
        }
        if let Ok(addr) = std::env::var("SVC_OVERLAY_LISTEN_ADDR") {
            c.transport.addr = addr.parse().map_err(|e| anyhow!("bad listen addr: {e}"))?;
        }
        if let Ok(n) = std::env::var("SVC_OVERLAY_MAX_CONNS") {
            c.transport.max_conns = n.parse().map_err(|e| anyhow!("bad max conns: {e}"))?;
        }
        c.validate()?;
        Ok(c)
    }

    pub fn validate(&self) -> Result<()> {
        if self.oap_max_frame == 0 || self.oap_max_frame > (1 << 20) {
            bail!("oap_max_frame must be 1..=1MiB");
        }
        if self.transport.max_conns == 0 {
            bail!("transport.max_conns must be > 0");
        }
        Ok(())
    }
}
