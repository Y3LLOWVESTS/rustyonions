use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;

use crate::commands;

#[derive(Parser, Debug)]
#[command(name="ronode", version, about="RustyOnions node")]
pub struct Args {
    /// Path to config (JSON or TOML)
    #[arg(long, default_value = "config.toml")]
    pub config: String,

    /// Override the data directory from config (e.g., ".data-tcp")
    #[arg(long)]
    pub data_dir: Option<String>,

    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Serve the overlay store (TCP or Tor HS).
    Serve {
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
        /// Optional HS key file for persistent onion (Tor only).
        #[arg(long)]
        hs_key_file: Option<std::path::PathBuf>,
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

    /// Quick Tor test: open a raw connection to <host:port> via Tor/Arti.
    TorDial {
        /// Target like "example.com:80" or "somerandomv3.onion:1777"
        to: String,
    },

    /// Initialize a default config file (TOML). Default path: ./config.toml
    Init {
        /// Output path (file). If omitted, writes ./config.toml
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },

    /// Print local store stats as JSON using the configured (or overridden) data_dir.
    StatsJson,
}

pub fn run() -> Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_max_level(Level::INFO).init();

    let args = Args::parse();

    match args.cmd {
        Some(Cmd::Serve { transport, hs_key_file }) => {
            commands::serve::serve(&args.config, args.data_dir.as_deref(), &transport, hs_key_file.as_deref())
        }
        Some(Cmd::Put { path, to, transport }) => {
            commands::put::put(&args.config, args.data_dir.as_deref(), &path, to.as_deref(), &transport)
        }
        Some(Cmd::Get { key, out, to, transport }) => {
            commands::get::get(&args.config, args.data_dir.as_deref(), &key, &out, to.as_deref(), &transport)
        }
        Some(Cmd::TorDial { to }) => commands::tor_dial::tor_dial(&args.config, &to),
        Some(Cmd::Init { path }) => commands::init::init(path.as_deref()),
        Some(Cmd::StatsJson) => commands::stats::stats_json(&args.config, args.data_dir.as_deref()),
        None => {
            eprintln!("No command provided. Use --help for usage.");
            Ok(())
        }
    }
}
