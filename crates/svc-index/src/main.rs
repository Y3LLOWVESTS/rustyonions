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

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).json().try_init().ok();

    let sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    let db_path = env::var("RON_INDEX_DB").unwrap_or_else(|_| DEFAULT_DB.into());

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

        IndexReq::PutAddress { addr, dir } => {
            match addr.parse::<Address>() {
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
