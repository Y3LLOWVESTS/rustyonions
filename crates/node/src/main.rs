//! Binary entrypoint for `ronode`.
use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use clap::{Parser, Subcommand};
use common::Config;
use overlay::{client_get_via, client_put_via, run_overlay_listener_with_transport, Store};
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;
use transport::{TcpTransport, Transport};

#[derive(Parser, Debug)]
#[command(name = "ronode", version, about = "RustyOnions node")]
struct Args {
    /// Path to config (JSON/TOML)
    #[arg(long, default_value = "config.json")]
    config: String,

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
        /// Optional HS key file for persistent onion (Tor only).
        #[arg(long)]
        hs_key_file: Option<PathBuf>,
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

    /// Smoke-test an outbound Tor TCP dial via SOCKS5.
    TorDial {
        /// Destination like example.com:80 or <onion>.onion:80
        to: String,
    },
}

fn main() -> Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_max_level(Level::INFO)
        .init();

    // Config
    let args = Args::parse();
    let cfg = Config::load(&args.config).context("loading config")?;

    match args.cmd {
        Some(Cmd::Serve {
            transport,
            hs_key_file,
        }) => {
            let store = Store::open(&cfg.data_dir, cfg.chunk_size)?;
            match transport.as_str() {
                "tcp" => {
                    let addr: SocketAddr = cfg.overlay_addr;
                    let tcp = TcpTransport::with_bind_addr(addr.to_string());
                    let ctrs = tcp.counters();
                    run_overlay_listener_with_transport(&tcp, store)?;
                    info!("serving TCP on {}; Ctrl+C to exit", addr);
                    // periodic stats
                    thread::spawn(move || loop {
                        std::thread::sleep(Duration::from_secs(10));
                        let s = ctrs.snapshot();
                        info!(
                            "stats/tcp total_in={} total_out={} last_min_in={} last_min_out={}",
                            s.total_in, s.total_out, s.per_min_in[59], s.per_min_out[59]
                        );
                    });
                }
                "tor" => {
                    if let Some(p) = hs_key_file {
                        std::env::set_var("RO_HS_KEY_FILE", p);
                    }
                    let arti = ArtiTransport::new(
                        cfg.socks5_addr.clone(),
                        cfg.tor_ctrl_addr.clone(),
                        Duration::from_millis(cfg.connect_timeout_ms),
                    );
                    let ctrs = arti.counters();
                    run_overlay_listener_with_transport(&arti, store)?;
                    info!("serving via Tor HS; watch logs for *.onion address");
                    thread::spawn(move || loop {
                        std::thread::sleep(Duration::from_secs(10));
                        let s = ctrs.snapshot();
                        info!(
                            "stats/tor total_in={} total_out={} last_min_in={} last_min_out={}",
                            s.total_in, s.total_out, s.per_min_in[59], s.per_min_out[59]
                        );
                    });
                }
                other => return Err(anyhow!("unknown transport {other}")),
            }
            // park main thread
            loop {
                std::thread::park();
            }
        }

        Some(Cmd::Put {
            path,
            to,
            transport,
        }) => {
            let bytes = fs::read(&path).with_context(|| format!("reading input {}", path))?;

            match transport.as_str() {
                "tcp" => {
                    let addr: SocketAddr = to
                        .as_deref()
                        .unwrap_or(&format!("{}", cfg.overlay_addr))
                        .parse()
                        .context("parsing --to host:port")?;
                    let tcp = TcpTransport::new();
                    let before = tcp.counters().snapshot();
                    let key = client_put_via(&tcp, &addr.to_string(), &bytes)?;
                    let after = tcp.counters().snapshot();
                    println!("{key}");
                    eprintln!(
                        "stats put tcp: +in={} +out={}",
                        after.total_in.saturating_sub(before.total_in),
                        after.total_out.saturating_sub(before.total_out),
                    );
                }
                "tor" => {
                    let to = to.ok_or_else(|| {
                        anyhow!("--to <onion:port> required with --transport tor")
                    })?;
                    let arti = ArtiTransport::new(
                        cfg.socks5_addr.clone(),
                        cfg.tor_ctrl_addr.clone(),
                        Duration::from_millis(cfg.connect_timeout_ms),
                    );
                    let before = arti.counters().snapshot();
                    let key = client_put_via(&arti, &to, &bytes)?;
                    let after = arti.counters().snapshot();
                    println!("{key}");
                    eprintln!(
                        "stats put tor: +in={} +out={}",
                        after.total_in.saturating_sub(before.total_in),
                        after.total_out.saturating_sub(before.total_out),
                    );
                }
                other => return Err(anyhow!("unknown transport {other}")),
            }
        }

        Some(Cmd::Get {
            key,
            out,
            to,
            transport,
        }) => match transport.as_str() {
            "tcp" => {
                let addr: SocketAddr = to
                    .as_deref()
                    .unwrap_or(&format!("{}", cfg.overlay_addr))
                    .parse()
                    .context("parsing --to host:port")?;
                let tcp = TcpTransport::new();
                let before = tcp.counters().snapshot();
                let maybe = client_get_via(&tcp, &addr.to_string(), &key)?;
                let after = tcp.counters().snapshot();
                match maybe {
                    Some(bytes) => {
                        let mut f =
                            fs::File::create(&out).with_context(|| format!("creating {}", out))?;
                        f.write_all(&bytes)?;
                        eprintln!(
                            "stats get tcp: +in={} +out={}",
                            after.total_in.saturating_sub(before.total_in),
                            after.total_out.saturating_sub(before.total_out),
                        );
                    }
                    None => eprintln!("NOT FOUND"),
                }
            }
            "tor" => {
                let to =
                    to.ok_or_else(|| anyhow!("--to <onion:port> required with --transport tor"))?;
                let arti = ArtiTransport::new(
                    cfg.socks5_addr.clone(),
                    cfg.tor_ctrl_addr.clone(),
                    Duration::from_millis(cfg.connect_timeout_ms),
                );
                let before = arti.counters().snapshot();
                let maybe = client_get_via(&arti, &to, &key)?;
                let after = arti.counters().snapshot();
                match maybe {
                    Some(bytes) => {
                        let mut f =
                            fs::File::create(&out).with_context(|| format!("creating {}", out))?;
                        f.write_all(&bytes)?;
                        eprintln!(
                            "stats get tor: +in={} +out={}",
                            after.total_in.saturating_sub(before.total_in),
                            after.total_out.saturating_sub(before.total_out),
                        );
                    }
                    None => eprintln!("NOT FOUND"),
                }
            }
            other => return Err(anyhow!("unknown transport {other}")),
        },

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
