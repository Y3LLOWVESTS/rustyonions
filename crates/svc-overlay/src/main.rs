// crates/svc-overlay/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;

use ron_bus::api::{
    Envelope, IndexReq, IndexResp, OverlayReq, OverlayResp, StorageReq, StorageResp, Status,
};
use ron_bus::uds::{listen, recv, send};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_SOCK: &str = "/tmp/ron/svc-overlay.sock";
const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_STORAGE_SOCK: &str = "/tmp/ron/svc-storage.sock";

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let sock = env::var("RON_OVERLAY_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());

    if let Some(parent) = std::path::Path::new(&sock).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = listen(&sock)?;
    info!("svc-overlay listening on {}", sock);

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

    let reply_env = match rmp_serde::from_slice::<OverlayReq>(&env.payload) {
        Ok(OverlayReq::Health) => {
            let _st = Status { ok: true, message: "ok".into() };
            let payload = rmp_serde::to_vec(&OverlayResp::HealthOk).expect("encode");
            Envelope { service: "svc.overlay".into(), method: "v1.ok".into(), corr_id: env.corr_id, token: vec![], payload }
        }
        Ok(OverlayReq::Get { addr, rel }) => {
            match overlay_get(&addr, &rel) {
                Ok(Some(bytes)) => {
                    let payload = rmp_serde::to_vec(&OverlayResp::Bytes { data: bytes }).expect("encode");
                    Envelope { service: "svc.overlay".into(), method: "v1.ok".into(), corr_id: env.corr_id, token: vec![], payload }
                }
                Ok(None) => {
                    let payload = rmp_serde::to_vec(&OverlayResp::NotFound).expect("encode");
                    Envelope { service: "svc.overlay".into(), method: "v1.not_found".into(), corr_id: env.corr_id, token: vec![], payload }
                }
                Err(e) => {
                    let payload = rmp_serde::to_vec(&OverlayResp::Err { err: e.to_string() }).expect("encode");
                    Envelope { service: "svc.overlay".into(), method: "v1.err".into(), corr_id: env.corr_id, token: vec![], payload }
                }
            }
        }
        Err(e) => {
            let payload = rmp_serde::to_vec(&OverlayResp::Err { err: format!("bad req: {e}") }).expect("encode");
            Envelope { service: "svc.overlay".into(), method: "v1.err".into(), corr_id: env.corr_id, token: vec![], payload }
        }
    };

    let _ = send(&mut stream, &reply_env);
    Ok(())
}

/// Resolve addr via svc-index, then read file via svc-storage.
/// rel="" defaults to "payload.bin".
fn overlay_get(addr: &str, rel: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let index_sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_INDEX_SOCK.into());
    let storage_sock = env::var("RON_STORAGE_SOCK").unwrap_or_else(|_| DEFAULT_STORAGE_SOCK.into());

    // Resolve
    let dir = {
        let mut s = UnixStream::connect(index_sock)?;
        let req = Envelope {
            service: "svc.index".into(),
            method: "v1.resolve".into(),
            corr_id: 1,
            token: vec![],
            payload: rmp_serde::to_vec(&IndexReq::Resolve { addr: addr.to_string() })?,
        };
        send(&mut s, &req)?;
        let env = recv(&mut s)?;
        match rmp_serde::from_slice::<IndexResp>(&env.payload)? {
            IndexResp::Resolved { dir } => dir,
            IndexResp::NotFound => return Ok(None),
            IndexResp::Err { err } => return Err(anyhow::anyhow!(err)),
            IndexResp::HealthOk | IndexResp::PutOk => return Err(anyhow::anyhow!("unexpected index resp")),
        }
    };

    // Read file via storage
    let rel = if rel.is_empty() { "payload.bin" } else { rel };
    let mut s = UnixStream::connect(storage_sock)?;
    let req = Envelope {
        service: "svc.storage".into(),
        method: "v1.read_file".into(),
        corr_id: 2,
        token: vec![],
        payload: rmp_serde::to_vec(&StorageReq::ReadFile { dir, rel: rel.to_string() })?,
    };
    send(&mut s, &req)?;
    let env = recv(&mut s)?;
    match rmp_serde::from_slice::<StorageResp>(&env.payload)? {
        StorageResp::File { bytes } => Ok(Some(bytes)),
        StorageResp::NotFound => Ok(None),
        StorageResp::Err { err } => Err(anyhow::anyhow!(err)),
        StorageResp::HealthOk | StorageResp::Written => Err(anyhow::anyhow!("unexpected storage resp")),
    }
}
