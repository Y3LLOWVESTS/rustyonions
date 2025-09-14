// crates/svc-index/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use naming::Address;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{listen, recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_DB: &str = ".data/index";

/// Encode a value to MessagePack. On failure, log and return an empty Vec instead of panicking.
fn to_vec_or_log<T: serde::Serialize>(value: &T) -> Vec<u8> {
    match rmp_serde::to_vec(value) {
        Ok(v) => v,
        Err(e) => {
            error!(error=?e, "svc-index: msgpack encode failed");
            Vec::new()
        }
    }
}

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .try_init()
        .ok();

    let sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    let db_path = env::var("RON_INDEX_DB").unwrap_or_else(|_| DEFAULT_DB.into());

    // Open DB without panicking; log and exit non-zero if it fails
    let idx = Arc::new(match index::Index::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            error!(db=%db_path, error=?e, "failed to open index database");
            std::process::exit(2);
        }
    });

    info!(
        socket = sock.as_str(),
        db = db_path.as_str(),
        "svc-index listening"
    );
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
            error!(error=?e, "recv error");
            return Ok(());
        }
    };

    let req: IndexReq = match rmp_serde::from_slice(&env.payload) {
        Ok(x) => x,
        Err(e) => {
            error!(error=?e, "decode req error");
            return Ok(());
        }
    };

    let resp = match req {
        IndexReq::Health => IndexResp::HealthOk,

        IndexReq::Resolve { addr } => {
            info!(%addr, "resolve request");
            match addr.parse::<Address>() {
                Ok(a) => match idx.get_bundle_dir(&a) {
                    Ok(Some(p)) => {
                        let dir = p.to_string_lossy().into_owned();
                        info!(%addr, dir=%dir, "resolve FOUND");
                        IndexResp::Resolved { dir }
                    }
                    Ok(None) => {
                        info!(%addr, "resolve NOT FOUND");
                        IndexResp::NotFound
                    }
                    Err(e) => {
                        error!(%addr, error=?e, "resolve error");
                        IndexResp::Err { err: e.to_string() }
                    }
                },
                Err(e) => {
                    error!(%addr, error=?e, "bad address");
                    IndexResp::Err { err: e.to_string() }
                }
            }
        }

        IndexReq::PutAddress { addr, dir } => match addr.parse::<Address>() {
            Ok(a) => match idx.put_address(&a, PathBuf::from(&dir)) {
                Ok(_) => {
                    info!(%addr, %dir, "index PUT ok");
                    IndexResp::PutOk
                }
                Err(e) => {
                    error!(%addr, %dir, error=?e, "index PUT error");
                    IndexResp::Err { err: e.to_string() }
                }
            },
            Err(e) => {
                error!(%addr, %dir, error=?e, "index PUT bad address");
                IndexResp::Err { err: e.to_string() }
            }
        },
    };

    let payload = to_vec_or_log(&resp);
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
