// crates/ron-kernel/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use ron_bus::api::{
    Envelope, IndexReq, IndexResp, OverlayReq, OverlayResp, StorageReq, StorageResp,
};
use ron_bus::uds::{recv, send};

use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const DEFAULT_INDEX_BIN: &str = "svc-index";
const DEFAULT_OVERLAY_BIN: &str = "svc-overlay";
const DEFAULT_STORAGE_BIN: &str = "svc-storage";

const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_OVERLAY_SOCK: &str = "/tmp/ron/svc-overlay.sock";
const DEFAULT_STORAGE_SOCK: &str = "/tmp/ron/svc-storage.sock";

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let index_bin = env::var("RON_SVC_INDEX_BIN").unwrap_or_else(|_| DEFAULT_INDEX_BIN.into());
    let overlay_bin = env::var("RON_SVC_OVERLAY_BIN").unwrap_or_else(|_| DEFAULT_OVERLAY_BIN.into());
    let storage_bin = env::var("RON_SVC_STORAGE_BIN").unwrap_or_else(|_| DEFAULT_STORAGE_BIN.into());

    let index_sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_INDEX_SOCK.into());
    let overlay_sock = env::var("RON_OVERLAY_SOCK").unwrap_or_else(|_| DEFAULT_OVERLAY_SOCK.into());
    let storage_sock = env::var("RON_STORAGE_SOCK").unwrap_or_else(|_| DEFAULT_STORAGE_SOCK.into());

    info!("ron-kernel starting…");
    info!(%index_bin, %index_sock, "svc-index");
    info!(%overlay_bin, %overlay_sock, "svc-overlay");
    info!(%storage_bin, %storage_sock, "svc-storage");

    let mut idx = spawn(&index_bin, &[])?;
    let mut ovl = spawn(&overlay_bin, &[])?;
    let mut sto = spawn(&storage_bin, &[])?;

    loop {
        let iok = check_index_health(&index_sock);
        let ook = check_overlay_health(&overlay_sock);
        let sok = check_storage_health(&storage_sock);
        info!(index = iok, overlay = ook, storage = sok, "health");

        if !iok {
            warn!("restarting svc-index…");
            let _ = idx.kill();
            let _ = idx.wait();
            std::thread::sleep(Duration::from_secs(1));
            idx = spawn(&index_bin, &[])?;
        }
        if !ook {
            warn!("restarting svc-overlay…");
            let _ = ovl.kill();
            let _ = ovl.wait();
            std::thread::sleep(Duration::from_secs(1));
            ovl = spawn(&overlay_bin, &[])?;
        }
        if !sok {
            warn!("restarting svc-storage…");
            let _ = sto.kill();
            let _ = sto.wait();
            std::thread::sleep(Duration::from_secs(1));
            sto = spawn(&storage_bin, &[])?;
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

fn spawn(bin: &str, args: &[&str]) -> std::io::Result<Child> {
    Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}

fn check_index_health(sock: &str) -> bool {
    if let Ok(mut s) = UnixStream::connect(sock) {
        let payload = rmp_serde::to_vec(&IndexReq::Health).unwrap_or_default();
        let req = Envelope {
            service: "svc.index".into(),
            method: "v1.health".into(),
            corr_id: 0,
            token: vec![],
            payload,
        };
        if send(&mut s, &req).is_ok() {
            if let Ok(env) = recv(&mut s) {
                if let Ok(resp) = rmp_serde::from_slice::<IndexResp>(&env.payload) {
                    return matches!(resp, IndexResp::HealthOk);
                }
            }
        }
    }
    false
}

fn check_overlay_health(sock: &str) -> bool {
    if let Ok(mut s) = UnixStream::connect(sock) {
        let req = Envelope {
            service: "svc.overlay".into(),
            method: "v1.health".into(),
            corr_id: 0,
            token: vec![],
            payload: rmp_serde::to_vec(&OverlayReq::Health).unwrap_or_default(),
        };
        if send(&mut s, &req).is_ok() {
            if let Ok(env) = recv(&mut s) {
                if let Ok(resp) = rmp_serde::from_slice::<OverlayResp>(&env.payload) {
                    return matches!(resp, OverlayResp::HealthOk);
                }
            }
        }
    }
    false
}

fn check_storage_health(sock: &str) -> bool {
    if let Ok(mut s) = UnixStream::connect(sock) {
        let req = Envelope {
            service: "svc.storage".into(),
            method: "v1.health".into(),
            corr_id: 0,
            token: vec![],
            payload: rmp_serde::to_vec(&StorageReq::Health).unwrap_or_default(),
        };
        if send(&mut s, &req).is_ok() {
            if let Ok(env) = recv(&mut s) {
                if let Ok(resp) = rmp_serde::from_slice::<StorageResp>(&env.payload) {
                    return matches!(resp, StorageResp::HealthOk);
                }
            }
        }
    }
    false
}
