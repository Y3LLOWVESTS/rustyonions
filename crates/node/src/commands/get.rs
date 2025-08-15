use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use overlay::client_get_via;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Duration;
use transport::TcpTransport;

pub fn get(
    config_path: &str,
    data_dir_override: Option<&str>,
    key: &str,
    out: &str,
    to: Option<&str>,
    transport: &str,
) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }

    match transport {
        "tcp" => {
            let addr: SocketAddr = to
                .unwrap_or(&format!("{}", cfg.overlay_addr))
                .parse()
                .context("parsing --to host:port")?;
            let tcp = TcpTransport::new();
            let before = tcp.counters().snapshot();
            let maybe = client_get_via(&tcp, &addr.to_string(), key)?;
            let after = tcp.counters().snapshot();
            match maybe {
                Some(bytes) => {
                    let mut f = fs::File::create(out).with_context(|| format!("creating {}", out))?;
                    f.write_all(&bytes)?;
                    eprintln!(
                        "stats get tcp: +in={} +out={}",
                        after.total_in.saturating_sub(before.total_in),
                        after.total_out.saturating_sub(before.total_out),
                    );
                }
                None => eprintln!("NOT FOUND"),
            }
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
            let maybe = client_get_via(&arti, target, key)?;
            let after = arti.counters().snapshot();
            match maybe {
                Some(bytes) => {
                    let mut f = fs::File::create(out).with_context(|| format!("creating {}", out))?;
                    f.write_all(&bytes)?;
                    eprintln!(
                        "stats get tor: +in={} +out={}",
                        after.total_in.saturating_sub(before.total_in),
                        after.total_out.saturating_sub(before.total_out),
                    );
                }
                None => eprintln!("NOT FOUND"),
            }
            Ok(())
        }
        other => Err(anyhow!("unknown transport {other}")),
    }
}
