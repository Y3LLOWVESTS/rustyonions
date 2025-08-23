#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

use overlay::{client_get, client_put, run_overlay_listener};

/// RustyOnions node CLI (TCP overlay + optional Tor flags for compatibility)
#[derive(Parser, Debug)]
#[command(name = "ronode", version, about = "RustyOnions node")]
struct Cli {
    /// RUST_LOG-like filter, e.g. "info,overlay=debug"
    #[arg(long, default_value = "info")]
    log: String,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start an overlay listener
    Serve {
        /// Bind address (ip:port), e.g. 127.0.0.1:1777
        #[arg(long, default_value = "127.0.0.1:1777")]
        bind: SocketAddr,

        /// Transport to use (accepted for compatibility). Only `tcp` is supported here.
        #[arg(long, default_value = "tcp")]
        transport: String,

        /// Path to the sled DB for the overlay store
        #[arg(long, default_value = ".data/sled")]
        store_db: PathBuf,
    },

    /// PUT a file to a remote overlay listener, prints the content hash.
    Put {
        /// Target address (ip:port)
        #[arg(long, default_value = "127.0.0.1:1777")]
        to: String,
        /// File to upload
        #[arg(long)]
        path: PathBuf,
    },

    /// GET a blob by hash from a remote overlay listener, writes to a file.
    Get {
        /// Target address (ip:port)
        #[arg(long, default_value = "127.0.0.1:1777")]
        from: String,
        /// Hex hash
        #[arg(long)]
        hash: String,
        /// Output file
        #[arg(long)]
        out: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(cli.log));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.cmd {
        Commands::Serve { bind, transport, store_db } => {
            if transport.to_lowercase() != "tcp" {
                bail!("only tcp transport is supported by this binary at the moment (got `{transport}`)");
            }
            info!(%bind, store=?store_db, "starting overlay TCP listener");
            run_overlay_listener(bind, &store_db).context("start overlay listener")?;
            info!("press Ctrl-C to stop…");
            signal::ctrl_c().await?;
            Ok(())
        }
        Commands::Put { to, path } => {
            let hash = client_put(&to, &path).await.context("client put")?;
            println!("{hash}");
            Ok(())
        }
        Commands::Get { from, hash, out } => {
            client_get(&from, &hash, &out).await.context("client get")?;
            println!("wrote {}", out.display());
            Ok(())
        }
    }
}
