// crates/svc-storage/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ron_bus::api::{Envelope, StorageReq, StorageResp};
use ron_bus::uds::{listen, recv, send};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_SOCK: &str = "/tmp/ron/svc-storage.sock";

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let sock = env::var("RON_STORAGE_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());

    if let Some(parent) = std::path::Path::new(&sock).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = listen(&sock)?;
    info!("svc-storage listening on {}", sock);

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                std::thread::spawn(|| {
                    if let Err(e) = handle_client(stream) {
                        error!(error=?e, "client handler error");
                    }
                });
            }
            Err(e) => error!(error=?e, "accept error"),
        }
    }
    Ok(())
}

fn handle_client(mut stream: UnixStream) -> std::io::Result<()> {
    let env = match recv(&mut stream) {
        Ok(e) => e,
        Err(e) => {
            error!(error=?e, "recv error");
            return Ok(());
        }
    };

    let resp = match rmp_serde::from_slice::<StorageReq>(&env.payload) {
        Ok(StorageReq::Health) => StorageResp::HealthOk,
        Ok(StorageReq::ReadFile { dir, rel }) => match read_file(&dir, &rel) {
            Ok(bytes) => StorageResp::File { bytes },
            Err(e) if e.downcast_ref::<std::io::Error>().is_some() => StorageResp::NotFound,
            Err(e) => StorageResp::Err { err: e.to_string() },
        },
        Ok(StorageReq::WriteFile { dir, rel, bytes }) => match write_file(&dir, &rel, &bytes) {
            Ok(()) => StorageResp::Written,
            Err(e) => StorageResp::Err { err: e.to_string() },
        },
        Err(e) => StorageResp::Err { err: format!("bad req: {e}") },
    };

    let payload = rmp_serde::to_vec(&resp).expect("encode resp");
    let reply = Envelope {
        service: "svc.storage".into(),
        method: "v1.ok".into(),
        corr_id: env.corr_id,
        token: vec![],
        payload,
    };
    let _ = send(&mut stream, &reply);
    Ok(())
}

fn read_file(dir: &str, rel: &str) -> Result<Vec<u8>> {
    let mut path = PathBuf::from(dir);
    let relp = Path::new(rel);
    path.push(relp);
    Ok(fs::read(&path).with_context(|| format!("read {}", path.display()))?)
}

fn write_file(dir: &str, rel: &str, bytes: &[u8]) -> Result<()> {
    let mut path = PathBuf::from(dir);
    path.push(Path::new(rel));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes)?;
    Ok(())
}
