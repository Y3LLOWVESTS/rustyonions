//! Binary entrypoint for `ronode`.
use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use clap::{Parser, Subcommand};
use common::Config;
use overlay::{client_get, client_put, run_overlay_listener, Store};
use std::fs;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;
use transport::{SmallMsgTransport, Transport};

/// CLI for running a node, storing bytes, and moving data across transports.
#[derive(Parser, Debug)]
#[command(name = "ronode", version = "0.1.0", about = "RustyOnions node")]
struct Args {
    /// Path to config (JSON or TOML)
    #[arg(long, default_value = "config.json")]
    config: String,

    /// Log level (error|warn|info|debug|trace). Env `RUST_LOG` also honored.
    #[arg(long, default_value = "info")]
    log: String,

    #[command(subcommand)]
    /// Subcommands map to common workflows used during dev/testing.
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
/// Subcommands map to common workflows used during dev/testing.
enum Cmd {
    /// Serve the overlay store at configured address.
    Serve,
    /// Put a file into the overlay; prints the content hash.
    Put {
        /// Path to file to upload.
        path: String,
    },
    /// Get a hash from the overlay to a local path.
    Get {
        /// Content hash to retrieve.
        key: String,
        /// Output file path.
        out: String,
    },
    /// Demo: use dev TCP to send a small message.
    DevSend {
        /// Address like 127.0.0.1:2888
        to: String,
        /// Message bytes to send (utf8).
        msg: String,
    },
    /// Demo: use Tor/Arti SOCKS to connect to an addr (e.g., hostname:port).
    TorDial {
        /// Address to connect via SOCKS5 (e.g., example.com:80 or onion:port).
        to: String,
    },
}

fn init_tracing(level: &str) {
    let env = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    tracing_subscriber::fmt()
        .with_env_filter(env)
        .with_max_level(Level::TRACE)
        .compact()
        .init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(&args.log);

    let cfg = if Path::new(&args.config).exists() {
        Config::load(&args.config).context("loading config")?
    } else {
        info!("config not found, using defaults");
        Config::default()
    };

    match args.cmd {
        Some(Cmd::Serve) => {
            // Overlay store server
            let store = Store::open(&cfg.data_dir, cfg.chunk_size)?;
            let addr: SocketAddr = cfg.overlay_addr;
            run_overlay_listener(addr, store)?;
            info!("serving; press Ctrl+C to exit");
            // park the main thread
            loop {
                std::thread::park();
            }
        }

        Some(Cmd::Put { path }) => {
            let bytes = fs::read(&path).with_context(|| format!("reading {path}"))?;
            let hash = client_put(cfg.overlay_addr, &bytes)?;
            println!("{hash}");
        }

        Some(Cmd::Get { key, out }) => {
            if let Some(bytes) = client_get(cfg.overlay_addr, &key)? {
                fs::write(&out, &bytes)?;
                info!("OK wrote {} bytes to {}", bytes.len(), out);
            } else {
                return Err(anyhow!("Key {key} not found at {}", cfg.overlay_addr));
            }
        }

        Some(Cmd::DevSend { to, msg }) => {
            // Start a tiny echo listener on our dev inbox, then send a small msg.
            let dev = SmallMsgTransport::new(cfg.dev_inbox_addr.to_string());
            let _ = dev.listen(Arc::new(|mut s| {
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
