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
    /// Optional path to config (accepted for compatibility; currently unused)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Log filter (RUST_LOG also supported, e.g. `info,overlay=debug`)
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
    },

    /// PUT a file to a remote overlay node
    Put {
        /// Path to the local file
        path: PathBuf,

        /// Remote address (ip:port), e.g. 127.0.0.1:1777 or <onion>:1777
        #[arg(long, value_name = "ADDR")]
        to: String,

        /// Transport to use (accepted for compatibility). Only `tcp` is supported here.
        #[arg(long, default_value = "tcp")]
        transport: String,
    },

    /// GET a hash from a remote overlay node
    Get {
        /// Hash (hex)
        hash: String,

        /// Output file path
        #[arg(long)]
        out: PathBuf,

        /// Remote address (ip:port), e.g. 127.0.0.1:1777 or <onion>:1777
        #[arg(long, value_name = "ADDR")]
        from: String,

        /// Transport to use (accepted for compatibility). Only `tcp` is supported here.
        #[arg(long, default_value = "tcp")]
        transport: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Nice error reports; ignore failures so we don’t abort.
    let _ = color_eyre::install();

    // Parse CLI
    let cli = Cli::parse();

    // Tracing / log setup (RUST_LOG respected; `--log` overrides when RUST_LOG unset)
    let filter = if std::env::var(EnvFilter::DEFAULT_ENV).is_ok() {
        EnvFilter::from_default_env()
    } else {
        EnvFilter::new(cli.log)
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.cmd {
        Commands::Serve { bind, transport } => {
            if transport.to_lowercase() != "tcp" {
                bail!("only tcp transport is supported by this binary at the moment (got `{transport}`)");
            }
            info!(%bind, "starting overlay TCP listener");
            run_overlay_listener(bind).context("start overlay listener")?;

            info!("press Ctrl-C to stop…");
            signal::ctrl_c().await.context("waiting for Ctrl-C")?;
            info!("shutting down");
        }

        Commands::Put { path, to, transport } => {
            if transport.to_lowercase() != "tcp" {
                bail!("only tcp transport is supported by this binary at the moment (got `{transport}`)");
            }
            let hash = client_put(&to, &path)
                .with_context(|| format!("PUT {} -> {}", path.display(), to))?;
            println!("{hash}");
        }

        Commands::Get { hash, out, from, transport } => {
            if transport.to_lowercase() != "tcp" {
                bail!("only tcp transport is supported by this binary at the moment (got `{transport}`)");
            }
            client_get(&from, &hash, &out)
                .with_context(|| format!("GET {hash} from {from} -> {}", out.display()))?;
            println!("wrote {}", out.display());
        }
    }

    Ok(())
}
