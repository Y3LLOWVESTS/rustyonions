#![forbid(unsafe_code)]

use std::net::SocketAddr;
use super::types::Config;

pub fn validate(cfg: &Config) -> anyhow::Result<()> {
    let _admin: SocketAddr = cfg.admin_addr.parse()?;
    let _overlay: SocketAddr = cfg.overlay_addr.parse()?;

    let t = &cfg.transport;
    if t.max_conns.unwrap_or(2048) == 0 { anyhow::bail!("transport.max_conns must be > 0"); }
    if t.idle_timeout_ms.unwrap_or(30_000) < 1_000 { anyhow::bail!("transport.idle_timeout_ms too small"); }
    if t.read_timeout_ms.unwrap_or(5_000) < 100 { anyhow::bail!("transport.read_timeout_ms too small"); }
    if t.write_timeout_ms.unwrap_or(5_000) < 100 { anyhow::bail!("transport.write_timeout_ms too small"); }

    Ok(())
}
