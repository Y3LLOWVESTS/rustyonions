// tools/ronctl/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{recv, send};

/// Default UDS path for svc-index if RON_INDEX_SOCK is not set.
const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";

#[derive(Parser, Debug)]
#[command(
    name = "ronctl",
    author,
    version,
    about = "RustyOnions control tool for svc-index"
)]
struct Cli {
    /// Override the index socket path (or set RON_INDEX_SOCK env var)
    #[arg(long)]
    sock: Option<String>,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Health check svc-index
    Health,
    /// Resolve an address to a local directory
    Resolve {
        /// Address like b3:<hex>.ext
        addr: String,
    },
    /// Insert/overwrite an address -> directory mapping
    PutAddress {
        /// Address like b3:<hex>.ext
        addr: String,
        /// Filesystem directory path (absolute or working-dir relative)
        dir: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let sock = cli
        .sock
        .or_else(|| env::var("RON_INDEX_SOCK").ok())
        .unwrap_or_else(|| DEFAULT_INDEX_SOCK.to_string());

    match run(&sock, cli.cmd) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(sock: &str, cmd: Command) -> anyhow::Result<ExitCode> {
    match cmd {
        Command::Health => {
            let mut s = UnixStream::connect(sock)?;
            let env = Envelope {
                service: "svc.index".into(),
                method: "v1.health".into(),
                corr_id: 1,
                token: vec![],
                payload: rmp_serde::to_vec(&IndexReq::Health)?,
            };
            send(&mut s, &env)?;
            let reply = recv(&mut s)?;

            match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
                Ok(IndexResp::HealthOk) => {
                    println!("index: OK");
                    Ok(ExitCode::SUCCESS)
                }
                Ok(other) => {
                    eprintln!("unexpected index response: {:?}", other);
                    Ok(ExitCode::from(2))
                }
                Err(e) => {
                    eprintln!("decode error: {e}");
                    Ok(ExitCode::from(3))
                }
            }
        }
        Command::Resolve { addr } => {
            let mut s = UnixStream::connect(sock)?;
            let env = Envelope {
                service: "svc.index".into(),
                method: "v1.resolve".into(),
                corr_id: 2,
                token: vec![],
                payload: rmp_serde::to_vec(&IndexReq::Resolve { addr: addr.clone() })?,
            };
            send(&mut s, &env)?;
            let reply = recv(&mut s)?;

            match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
                Ok(IndexResp::Resolved { dir }) => {
                    println!("{dir}");
                    Ok(ExitCode::SUCCESS)
                }
                Ok(IndexResp::NotFound) => {
                    eprintln!("not found");
                    Ok(ExitCode::from(4))
                }
                Ok(IndexResp::Err { err }) => {
                    eprintln!("svc-index error: {err}");
                    Ok(ExitCode::from(5))
                }
                Ok(other) => {
                    eprintln!("unexpected index response: {:?}", other);
                    Ok(ExitCode::from(6))
                }
                Err(e) => {
                    eprintln!("decode error: {e}");
                    Ok(ExitCode::from(7))
                }
            }
        }
        Command::PutAddress { addr, dir } => {
            let mut s = UnixStream::connect(sock)?;
            let env = Envelope {
                service: "svc.index".into(),
                method: "v1.put_address".into(),
                corr_id: 3,
                token: vec![],
                payload: rmp_serde::to_vec(&IndexReq::PutAddress {
                    addr: addr.clone(),
                    dir: dir.clone(),
                })?,
            };
            send(&mut s, &env)?;
            let reply = recv(&mut s)?;

            match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
                Ok(IndexResp::PutOk) => {
                    println!("ok");
                    Ok(ExitCode::SUCCESS)
                }
                Ok(IndexResp::NotFound) => {
                    // Not typical for PutAddress, but handle exhaustively
                    eprintln!("not found");
                    Ok(ExitCode::from(8))
                }
                Ok(IndexResp::Err { err }) => {
                    eprintln!("svc-index error: {err}");
                    Ok(ExitCode::from(9))
                }
                Ok(other) => {
                    eprintln!("unexpected index response: {:?}", other);
                    Ok(ExitCode::from(10))
                }
                Err(e) => {
                    eprintln!("decode error: {e}");
                    Ok(ExitCode::from(11))
                }
            }
        }
    }
}
