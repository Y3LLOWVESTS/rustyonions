use std::io;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rand::Rng;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";

#[derive(Parser)]
#[command(name = "ronctl", author, version, about = "RustyOnions control tool")]
struct Cli {
    /// Path to the index service socket (defaults to RON_INDEX_SOCK or /tmp/ron/svc-index.sock)
    #[arg(global = true, long)]
    index_sock: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Health check the index service
    Ping,
    /// Resolve an address to its bundle directory
    Resolve { addr: String },
    /// Insert/update an address mapping
    Put { addr: String, dir: String },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let sock = cli
        .index_sock
        .map(|p| p.to_string_lossy().into_owned())
        .or_else(|| std::env::var("RON_INDEX_SOCK").ok())
        .unwrap_or_else(|| DEFAULT_SOCK.into());

    let mut stream = UnixStream::connect(&sock)?;

    let (req, method) = match cli.cmd {
        Cmd::Ping => (IndexReq::Health, "v1.health"),
        Cmd::Resolve { addr } => (IndexReq::Resolve { addr }, "v1.resolve"),
        Cmd::Put { addr, dir } => (IndexReq::PutAddress { addr, dir }, "v1.put"),
    };

    let corr_id: u64 = rand::thread_rng().gen();
    let env = Envelope {
        service: "svc.index".into(),
        method: method.into(),
        corr_id,
        token: vec![],
        payload: rmp_serde::to_vec(&req).expect("encode req"),
    };

    send(&mut stream, &env).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let reply = recv(&mut stream).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if reply.corr_id != corr_id {
        eprintln!("Correlation ID mismatch");
        return Ok(());
    }

    match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
        Ok(IndexResp::HealthOk) => println!("svc-index: OK"),
        Ok(IndexResp::Resolved { dir }) => {
            if dir.is_empty() { println!("NOT FOUND"); } else { println!("{dir}"); }
        }
        Ok(IndexResp::PutOk) => println!("PUT OK"),
        Err(e) => eprintln!("decode error: {e}"),
    }
    Ok(())
}
