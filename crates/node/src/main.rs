use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use common::Config;
use overlay::{client_get, client_put, run_overlay_listener, Store};
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;
use transport::{TcpDevTransport};
use arti_transport::ArtiTransport;

#[derive(Parser, Debug)]
#[command(name = "ronode", version, about = "RustyOnions node")]
struct Args {
    /// Path to config (JSON)
    #[arg(long, default_value = "config.json")]
    config: String,

    #[arg(long)]
    transport: Option<String>,

    #[arg(long)]
    control: Option<String>,

    #[arg(long, default_value = "false")]
    verbose: bool,

    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve {
        #[arg(long)]
        bind: String,
        #[arg(long)]
        store: String,
        #[arg(long, default_value = "1024")]
        chunk: usize,
    },
    Put {
        #[arg(long)]
        to: String,
        #[arg(long)]
        file: String,
    },
    Get {
        #[arg(long)]
        from: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        out: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut builder = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(if args.verbose { Level::DEBUG.into() } else { Level::INFO.into() }),
        )
        .with_target(false);

    builder.init();

    let cfg = Config::load_from_path(&args.config).context("loading config")?;

    match args.cmd {
        Some(Command::Serve { bind, store, chunk }) => {
            info!("Effective transport: {:?}", args.transport);

            if let Some(t) = args.transport.as_deref() {
                if t.eq_ignore_ascii_case("tor") {
                    let control_addr = args.control.as_deref().unwrap_or("127.0.0.1:9051");
                    let socks_addr = "127.0.0.1:9050";

                    info!("Tor control: {}, Tor socks: {}", control_addr, socks_addr);

                    // Create ArtiTransport
                    let arti = ArtiTransport::new(Duration::from_secs(10), socks_addr.to_string());

                    // Extract port from bind
                    let bind_addr: SocketAddr = bind.parse()?;
                    let port = bind_addr.port();

                    match ArtiTransport::create_hidden_service(control_addr, None, port, bind_addr) {
                        Ok(onion_addr) => {
                            info!("Hidden service created at {}", onion_addr);
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Failed to create hidden service: {}", e));
                        }
                    }
                }
            }

            // Open store
            let store = Store::open(&store, chunk)?;
            run_overlay_listener(bind.parse()?, store)?;
        }

        Some(Command::Put { to, file }) => {
            let data = fs::read(&file)?;
            let addr: SocketAddr = to.parse()?;

            client_put(addr, &data)?;
            info!("File {} sent to {}", file, to);
        }

        Some(Command::Get { from, key, out }) => {
            let addr: SocketAddr = from.parse()?;
            if let Some(bytes) = client_get(addr, &key)? {
                fs::write(&out, &bytes)?;
                info!("OK wrote {} bytes to {}", bytes.len(), out);
            } else {
                return Err(anyhow::anyhow!("Key {} not found at {}", key, from));
            }
        }

        None => {
            eprintln!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}
