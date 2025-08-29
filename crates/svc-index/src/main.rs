use std::env;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use naming::Address;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{listen, recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_DB: &str = ".data/index";

fn main() -> std::io::Result<()> {
    // logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .json()
        .with_current_span(false)
        .with_span_list(false)
        .init();

    let sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    let db_path = env::var("RON_INDEX_DB").unwrap_or_else(|_| DEFAULT_DB.into());

    // Ensure parent dirs exist
    if let Some(parent) = PathBuf::from(&sock).parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open DB (wrap in Arc for per-conn threads)
    let idx = Arc::new(index::Index::open(&db_path).expect("failed to open index database"));

    info!(socket = sock.as_str(), db = db_path.as_str(), "svc-index listening");
    let listener: UnixListener = listen(&sock)?;

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                let idx = idx.clone();
                std::thread::spawn(move || {
                    if let Err(e) = serve_client(stream, idx) {
                        error!(error=?e, "client handler error");
                    }
                });
            }
            Err(e) => error!(error=?e, "accept error"),
        }
    }
    Ok(())
}

fn serve_client(mut stream: UnixStream, idx: Arc<index::Index>) -> std::io::Result<()> {
    let env = match recv(&mut stream) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("recv error: {e:?}");
            return Ok(());
        }
    };

    let req: IndexReq = match rmp_serde::from_slice(&env.payload) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("decode req error: {e:?}");
            return Ok(());
        }
    };

    let resp = match req {
        IndexReq::Health => IndexResp::HealthOk,

        IndexReq::Resolve { addr } => {
            match addr.parse::<Address>() {
                Ok(a) => match idx.get_bundle_dir(&a) {
                    Ok(Some(p)) => IndexResp::Resolved { dir: p.to_string_lossy().into_owned() },
                    _ => IndexResp::Resolved { dir: String::new() }, // NOT FOUND
                },
                Err(_) => IndexResp::Resolved { dir: String::new() }, // invalid address
            }
        }

        IndexReq::PutAddress { addr, dir } => {
            match addr.parse::<Address>() {
                Ok(a) => match idx.put_address(&a, &dir) {
                    Ok(()) => IndexResp::PutOk,
                    Err(_) => IndexResp::PutOk, // TODO: consider an error variant later
                },
                Err(_) => IndexResp::PutOk,
            }
        }
    };

    let payload = rmp_serde::to_vec(&resp).expect("encode resp");
    let reply = Envelope {
        service: "svc.index".into(),
        method: "v1.ok".into(),
        corr_id: env.corr_id,
        token: vec![],
        payload,
    };
    let _ = send(&mut stream, &reply);
    Ok(())
}
