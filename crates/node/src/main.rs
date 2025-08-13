//! Binary entrypoint for `ronode`.
use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use clap::{Parser, Subcommand};
use common::Config;
use overlay::{
    client_get, client_get_via, client_put, client_put_via, run_overlay_listener,
    run_overlay_listener_with_transport, Store,
};
use std::fs;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;
use transport::{SmallMsgTransport, Transport};

#[derive(Parser, Debug)]
#[command(name="ronode", version, about="RustyOnions node")]
struct Args {
    /// Path to config (JSON or TOML)
    #[arg(long, default_value = "config.json")]
    config: String,

    /// Log level (error|warn|info|debug|trace). Env `RUST_LOG` also honored.
    #[arg(long, default_value = "info")]
    log: String,

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Serve the overlay store (TCP or Tor HS).
    Serve {
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
    },

    /// Put a file (TCP by default; pass --transport tor + --to .onion:1777 to use Tor).
    Put {
        /// Path to file to upload.
        path: String,
        /// Optional override target like "host:port" or "xxxx.onion:1777"
        #[arg(long)]
        to: Option<String>,
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
    },

    /// Get a hash (TCP by default; pass --transport tor + --to .onion:1777 to use Tor).
    Get {
        /// Content hash to retrieve.
        key: String,
        /// Output file path.
        out: String,
        /// Optional override target like "host:port" or "xxxx.onion:1777"
        #[arg(long)]
        to: Option<String>,
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
    },

    /// Demo: use dev TCP to send a small message.
    DevSend {
        /// Address like 127.0.0.1:2888
        to: String,
        /// Message
        msg: String,
    },

    /// Smoke-test an outbound Tor TCP dial via SOCKS5.
    TorDial {
        /// Destination like example.com:80 or <onion>.onion:80
        to: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    // logging
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(args.log.clone()))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_max_level(Level::TRACE)
        .init();

    // config
    let cfg_path = Path::new(&args.config);
    let cfg = if cfg_path.exists() {
        Config::load(cfg_path)?
    } else {
        info!("config not found at {}, using defaults", cfg_path.display());
        Config::default()
    };

    match args.cmd {
        Some(Cmd::Serve { transport }) => {
            let store = Store::open(&cfg.data_dir, cfg.chunk_size)?;
            match transport.as_str() {
                "tcp" => {
                    let addr: SocketAddr = cfg.overlay_addr;
                    run_overlay_listener(addr, store)?;
                    info!("serving TCP on {}; Ctrl+C to exit", addr);
                }
                "tor" => {
                    let arti = ArtiTransport::new(
                        cfg.socks5_addr.clone(),
                        cfg.tor_ctrl_addr.clone(),
                        Duration::from_millis(cfg.connect_timeout_ms),
                    );
                    run_overlay_listener_with_transport(&arti, store)?;
                    info!("serving via Tor HS; watch logs for *.onion address");
                }
                other => return Err(anyhow!("unknown transport {other}")),
            }
            // park the main thread
            loop {
                std::thread::park();
            }
        }

        Some(Cmd::Put { path, to, transport }) => {
            let bytes = fs::read(&path)
                .with_context(|| format!("reading input {}", path))?;

            match transport.as_str() {
                "tcp" => {
                    let addr: SocketAddr = to
                        .as_deref()
                        .unwrap_or(&format!("{}", cfg.overlay_addr))
                        .parse()
                        .context("parsing --to host:port")?;
                    let key = client_put(addr, &bytes)?;
                    println!("{key}");
                }
                "tor" => {
                    let to = to.ok_or_else(|| anyhow!("--to <onion:port> required with --transport tor"))?;
                    let arti = ArtiTransport::new(
                        cfg.socks5_addr.clone(),
                        cfg.tor_ctrl_addr.clone(),
                        Duration::from_millis(cfg.connect_timeout_ms),
                    );
                    let key = client_put_via(&arti, &to, &bytes)?;
                    println!("{key}");
                }
                other => return Err(anyhow!("unknown transport {other}")),
            }
        }

        Some(Cmd::Get { key, out, to, transport }) => {
            match transport.as_str() {
                "tcp" => {
                    let addr: SocketAddr = to
                        .as_deref()
                        .unwrap_or(&format!("{}", cfg.overlay_addr))
                        .parse()
                        .context("parsing --to host:port")?;
                    if let Some(bytes) = client_get(addr, &key)? {
                        fs::write(&out, &bytes)
                            .with_context(|| format!("writing {}", out))?;
                        info!("wrote {}", out);
                    } else {
                        return Err(anyhow!("Key {key} not found at {}", addr));
                    }
                }
                "tor" => {
                    let to = to.ok_or_else(|| anyhow!("--to <onion:port> required with --transport tor"))?;
                    let arti = ArtiTransport::new(
                        cfg.socks5_addr.clone(),
                        cfg.tor_ctrl_addr.clone(),
                        Duration::from_millis(cfg.connect_timeout_ms),
                    );
                    if let Some(bytes) = client_get_via(&arti, &to, &key)? {
                        fs::write(&out, &bytes)
                            .with_context(|| format!("writing {}", out))?;
                        info!("wrote {}", out);
                    } else {
                        return Err(anyhow!("Key {key} not found at {}", to));
                    }
                }
                other => return Err(anyhow!("unknown transport {other}")),
            }
        }

        Some(Cmd::DevSend { to, msg }) => {
            let dev = SmallMsgTransport::new(cfg.dev_inbox_addr.to_string());
            let _ = dev.listen(std::sync::Arc::new(|mut s| {
                let mut buf = [0u8; 1024];
                if let Ok(n) = s.read(&mut buf) {
                    let _ = s.write_all(&buf[..n]);
                    let _ = s.flush();
                }
            }));
            dev.send_small(&to, msg.as_bytes())?;
            info!("dev msg sent to {to}");
        }

        Some(Cmd::TorDial { to }) => {
            let arti = ArtiTransport::new(
                cfg.socks5_addr.clone(),
                cfg.tor_ctrl_addr.clone(),
                Duration::from_millis(cfg.connect_timeout_ms),
            );
            let mut s = arti.connect(&to)?;
            s.write_all(b"HEAD / HTTP/1.1\r\nHost: example\r\n\r\n")?;
            s.flush()?;
            info!("tor dial success to {}", to);
        }

        None => {
            eprintln!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}
