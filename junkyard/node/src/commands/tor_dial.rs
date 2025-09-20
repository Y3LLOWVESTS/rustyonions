use anyhow::{Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use std::io::Write;
use std::time::Duration;
use tracing::info;
use transport::Transport; // <- bring the trait into scope so `.connect()` works

pub fn tor_dial(config_path: &str, to: &str) -> Result<()> {
    let cfg = Config::load(config_path).context("loading config")?;
    let arti = ArtiTransport::new(
        cfg.socks5_addr.clone(),
        cfg.tor_ctrl_addr.clone(),
        Duration::from_millis(cfg.connect_timeout_ms),
    );
    let mut s = arti.connect(to)?;
    s.write_all(b"HEAD / HTTP/1.1\r\nHost: example\r\n\r\n")?;
    s.flush()?;
    info!("tor dial success to {}", to);
    Ok(())
}
