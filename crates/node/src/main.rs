use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use common::Config;
use overlay::{client_get, client_put, run_overlay_listener, Store};
use std::fs;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;
use std::sync::Arc;
use transport::{SmallMsgTransport, TcpDevTransport};

#[derive(Parser, Debug)]
#[command(name="ronode", version, about="RustyOnions node")]
struct Args {
    /// Path to config (JSON)
    #[arg(long, default_value = "config.json")]
    config: String,

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Put a file into the overlay store (over the network)
    #[command(name = "overlay-put", alias = "overlayput")]
    OverlayPut {
        /// Path to the local file to upload
        file: String,
    },

    /// Get a chunk by hash and write to file (over the network)
    #[command(name = "overlay-get", alias = "overlayget")]
    OverlayGet {
        /// Hash of the chunk to fetch
        hash: String,
        /// Output file path
        out: String,
    },

    /// Start services (overlay + inbox)
    #[command(name = "run")]
    Run,

    /// Send a tiny message over the small-msg transport (dev TCP for now)
    #[command(name = "msg-send", alias = "msgsend")]
    MsgSend {
        /// Address of the recipient (ip:port)
        to: SocketAddr,
        /// Text message to send
        text: String,
    },

    /// Show metered totals (what will become Tor usage)
    #[command(name = "stats")]
    Stats,

    /// Relay helper (stub)
    #[command(name = "relay")]
    Relay {
        /// Action: start | stop | status
        action: String,
    },

    /// Print onion address (placeholder until Arti is wired)
    #[command(name = "onion")]
    Onion,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse();
    let cfg: Config = load_or_create(&args.config)?;

    match args.cmd.unwrap_or(Cmd::Run) {
        Cmd::Run => run(cfg),
        Cmd::OverlayPut { file } => overlay_put(&cfg, &file),
        Cmd::OverlayGet { hash, out } => overlay_get(&cfg, &hash, &out),
        Cmd::MsgSend { to, text } => msg_send(&cfg, to, text),
        Cmd::Stats => stats(&cfg),
        Cmd::Relay { action } => relay_action(&cfg, &action),
        Cmd::Onion => {
            if cfg.tor_enabled {
                println!("Tor is enabled in config, but Arti is not wired yet. Next milestone: print your .onion here.");
            } else {
                println!("Tor is disabled. Enable it by setting \"tor_enabled\": true in config.json.");
            }
            Ok(())
        }
    }
}

fn load_or_create(path: &str) -> Result<Config> {
    if Path::new(path).exists() {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    } else {
        let cfg = Config::default();
        if let Some(p) = Path::new(path).parent() {
            fs::create_dir_all(p).ok();
        }
        fs::write(path, serde_json::to_string_pretty(&cfg)?)?;
        Ok(cfg)
    }
}

fn run(cfg: Config) -> Result<()> {
    // Overlay store + listener (holds the sled lock)
    let store = Store::open(&cfg.db_path, cfg.chunk_size)?;
    let overlay_addr = cfg.overlay_listen;
    std::thread::spawn(move || {
        let _ = run_overlay_listener(overlay_addr, store);
    });

    // Small-message transport (dev TCP now, Arti later)
    let t = TcpDevTransport::new(Duration::from_secs(cfg.accounting_window_secs));
    let inbox_addr = cfg.inbox_listen.to_string();

    // Inbox handler: echo back "OK:<text_len>"
    t.listen(&inbox_addr, Arc::new(move |mut stream| {
        let mut buf_len = [0u8; 2];
        if stream.read(&mut buf_len).ok() == Some(2) {
            let len = u16::from_be_bytes(buf_len) as usize;
            let mut buf = vec![0u8; len];
            if stream.read(&mut buf).ok() == Some(len) {
                let _ = stream.write_all(b"OK:");
                let _ = stream.write_all(&(len as u16).to_be_bytes());
            }
        }
        let _ = stream.flush();
    }))?;

    info!("Node running. Overlay: {overlay_addr}, Inbox(dev): {inbox_addr}");
    loop {
        std::thread::park();
    }
}

fn overlay_put(cfg: &Config, file: &str) -> Result<()> {
    let bytes = fs::read(file)?;
    let hash = client_put(cfg.overlay_listen, &bytes)?;
    println!("{hash}");
    Ok(())
}

fn overlay_get(cfg: &Config, hash: &str, out: &str) -> Result<()> {
    match client_get(cfg.overlay_listen, hash)? {
        Some(bytes) => {
            fs::write(out, bytes)?;
            Ok(())
        }
        None => bail!("not found"),
    }
}

fn msg_send(cfg: &Config, to: SocketAddr, text: String) -> Result<()> {
    let t = TcpDevTransport::new(Duration::from_secs(cfg.accounting_window_secs));
    let mut s = t.dial(&to.to_string())?;
    let bytes = text.into_bytes();
    let len = (bytes.len() as u16).to_be_bytes();
    s.write_all(&len)?;
    s.write_all(&bytes)?;
    s.flush()?;

    // Read tiny reply
    let mut hdr = [0u8; 3];
    s.read_exact(&mut hdr)?;
    if &hdr[0..3] == b"OK:" {
        let mut sz = [0u8; 2];
        s.read_exact(&mut sz)?;
        let n = u16::from_be_bytes(sz);
        println!("Remote acknowledged {} bytes.", n);
    }
    Ok(())
}

fn stats(cfg: &Config) -> Result<()> {
    let t = TcpDevTransport::new(Duration::from_secs(cfg.accounting_window_secs));
    let (tx_total, rx_total) = t.counters().totals();
    let (tx_win, rx_win) = t.counters().window_totals();
    println!(
        "Small-msg transport usage (window={}s):",
        cfg.accounting_window_secs
    );
    println!(" totals: tx={}B rx={}B", tx_total, rx_total);
    println!(" window: tx={}B rx={}B", tx_win, rx_win);

    let target_bytes = ((tx_win + rx_win) as f32 * cfg.contribution_ratio) as u64;
    println!(
        " contribution target ({}×): {}B over window",
        cfg.contribution_ratio, target_bytes
    );
    Ok(())
}

fn relay_action(_cfg: &Config, action: &str) -> Result<()> {
    match action {
        "start" => println!("(stub) Starting Tor relay with rate caps based on accounting..."),
        "stop" => println!("(stub) Stopping Tor relay..."),
        "status" => println!("(stub) Relay is stopped. (Arti/Tor wiring TODO)"),
        other => println!("unknown action: {other} (use start|stop|status)"),
    }
    Ok(())
}
