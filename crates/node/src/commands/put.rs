use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use overlay::client_put_via;
use std::fs;
use std::net::SocketAddr;
use std::time::Duration;
use transport::TcpTransport;

pub fn put(
    config_path: &str,
    data_dir_override: Option<&str>,
    path: &str,
    to: Option<&str>,
    transport: &str,
) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }
    let bytes = fs::read(path).with_context(|| format!("reading input {}", path))?;

    match transport {
        "tcp" => {
            let addr: SocketAddr = to
                .unwrap_or(&format!("{}", cfg.overlay_addr))
                .parse()
                .context("parsing --to host:port")?;
            let tcp = TcpTransport::new();
            let before = tcp.counters().snapshot();
            let hash = client_put_via(&tcp, &addr.to_string(), &bytes)?;
            let after = tcp.counters().snapshot();

            eprintln!(
                "stats put tcp: +in={} +out={}",
                after.total_in.saturating_sub(before.total_in),
                after.total_out.saturating_sub(before.total_out),
            );
            println!("{hash}");
            Ok(())
        }
        "tor" => {
            let arti = ArtiTransport::new(
                cfg.socks5_addr.clone(),
                cfg.tor_ctrl_addr.clone(),
                Duration::from_millis(cfg.connect_timeout_ms),
            );
            let target = to.ok_or_else(|| anyhow!("--to <onion:port> required for tor"))?;
            let before = arti.counters().snapshot();
            let hash = client_put_via(&arti, target, &bytes)?;
            let after = arti.counters().snapshot();

            eprintln!(
                "stats put tor: +in={} +out={}",
                after.total_in.saturating_sub(before.total_in),
                after.total_out.saturating_sub(before.total_out),
            );
            println!("{hash}");
            Ok(())
        }
        other => Err(anyhow!("unknown transport {other}")),
    }
}
