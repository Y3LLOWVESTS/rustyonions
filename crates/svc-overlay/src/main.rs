// crates/svc-overlay/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;
use std::path::Path;

use ron_bus::api::{
    Envelope, IndexReq, IndexResp, OverlayReq, OverlayResp, Status, StorageReq, StorageResp,
};
use ron_bus::uds::{listen, recv, send};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_SOCK: &str = "/tmp/ron/svc-overlay.sock";
const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_STORAGE_SOCK: &str = "/tmp/ron/svc-storage.sock";

/// Encode a value to MessagePack. On failure, log and return an empty Vec instead of panicking.
/// This keeps runtime code free of `expect()` while surfacing the error.
fn to_vec_or_log<T: serde::Serialize>(value: &T) -> Vec<u8> {
    match rmp_serde::to_vec(value) {
        Ok(v) => v,
        Err(e) => {
            error!(error=?e, "svc-overlay: msgpack encode failed");
            Vec::new()
        }
    }
}

fn main() -> std::io::Result<()> {
    // Logging: honor RUST_LOG (fallback to info)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Socket path (ensure parent exists for macOS tmp dirs)
    let sock = env::var("RON_OVERLAY_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    if let Some(parent) = Path::new(&sock).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Listen + accept
    let listener = listen(&sock)?;
    info!("svc-overlay listening on {}", sock);

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                // Spawn a thread per connection (same as your original), but move the stream in.
                std::thread::spawn(move || {
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
            info!("overlay health probe");
            let _st = Status {
                ok: true,
                message: "ok".into(),
            };
            let payload = to_vec_or_log(&OverlayResp::HealthOk);
            Envelope {
                service: "svc.overlay".into(),
                method: "v1.ok".into(),
                corr_id: env.corr_id,
                token: vec![],
                payload,
            }
        }

        Ok(OverlayReq::Get { addr, rel }) => {
            // High-signal log: show incoming request fields
            info!(%addr, %rel, "overlay get");
            match overlay_get(&addr, &rel) {
                Ok(Some(bytes)) => {
                    info!(%addr, %rel, bytes = bytes.len(), "overlay get OK");
                    let payload = to_vec_or_log(&OverlayResp::Bytes { data: bytes });
                    Envelope {
                        service: "svc.overlay".into(),
                        method: "v1.ok".into(),
                        corr_id: env.corr_id,
                        token: vec![],
                        payload,
                    }
                }
                Ok(None) => {
                    info!(%addr, %rel, "overlay get NOT FOUND");
                    let payload = to_vec_or_log(&OverlayResp::NotFound);
                    Envelope {
                        service: "svc.overlay".into(),
                        method: "v1.not_found".into(),
                        corr_id: env.corr_id,
                        token: vec![],
                        payload,
                    }
                }
                Err(e) => {
                    error!(%addr, %rel, error=?e, "overlay get error");
                    let payload = to_vec_or_log(&OverlayResp::Err { err: e.to_string() });
                    Envelope {
                        service: "svc.overlay".into(),
                        method: "v1.err".into(),
                        corr_id: env.corr_id,
                        token: vec![],
                        payload,
                    }
                }
            }
        }

        Err(e) => {
            error!(error=?e, "bad overlay req");
            let payload = to_vec_or_log(&OverlayResp::Err {
                err: format!("bad req: {e}"),
            });
            Envelope {
                service: "svc.overlay".into(),
                method: "v1.err".into(),
                corr_id: env.corr_id,
                token: vec![],
                payload,
            }
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

    // ---- Resolve via index
    let dir = {
        let mut s = UnixStream::connect(&index_sock)?;
        let req = Envelope {
            service: "svc.index".into(),
            method: "v1.resolve".into(),
            corr_id: 1,
            token: vec![],
            payload: rmp_serde::to_vec(&IndexReq::Resolve {
                addr: addr.to_string(),
            })?,
        };
        send(&mut s, &req)?;
        let env = recv(&mut s)?;
        match rmp_serde::from_slice::<IndexResp>(&env.payload)? {
            IndexResp::Resolved { dir } => {
                info!(%addr, %dir, "index resolve FOUND");
                dir
            }
            IndexResp::NotFound => {
                info!(%addr, "index resolve NOT FOUND");
                return Ok(None);
            }
            IndexResp::Err { err } => {
                error!(%addr, error=%err, "index resolve ERR");
                return Err(anyhow::anyhow!(err));
            }
            IndexResp::HealthOk | IndexResp::PutOk => {
                let msg = "unexpected index resp";
                error!(%addr, msg);
                return Err(anyhow::anyhow!(msg));
            }
        }
    };

    // ---- Read file via storage
    let rel = if rel.is_empty() { "payload.bin" } else { rel };
    let mut s = UnixStream::connect(&storage_sock)?;
    let req = Envelope {
        service: "svc.storage".into(),
        method: "v1.read_file".into(),
        corr_id: 2,
        token: vec![],
        payload: rmp_serde::to_vec(&StorageReq::ReadFile {
            dir: dir.clone(),
            rel: rel.to_string(),
        })?,
    };
    send(&mut s, &req)?;
    let env = recv(&mut s)?;
    match rmp_serde::from_slice::<StorageResp>(&env.payload)? {
        StorageResp::File { bytes } => {
            info!(%addr, %dir, %rel, bytes = bytes.len(), "storage read OK");
            Ok(Some(bytes))
        }
        StorageResp::NotFound => {
            info!(%addr, %dir, %rel, "storage read NOT FOUND");
            Ok(None)
        }
        StorageResp::Err { err } => {
            error!(%addr, %dir, %rel, error=%err, "storage read ERR");
            Err(anyhow::anyhow!(err))
        }
        StorageResp::HealthOk | StorageResp::Written => {
            let msg = "unexpected storage resp";
            error!(%addr, %dir, %rel, msg);
            Err(anyhow::anyhow!(msg))
        }
    }
}
