use crate::ctrl::CtrlClient;
use accounting::{Counters, CountingStream};
use anyhow::{anyhow, Context, Result};
use std::net::{TcpListener};
use std::path::Path;
use std::thread;
use std::time::Duration;
use transport::{Handler, ReadWrite};

const OVERLAY_PORT: u16 = 1777;

/// Publish a v3 onion service and accept incoming connections, dispatching to `handler`.
pub(crate) fn publish_and_serve(
    tor_ctrl_addr: &str,
    counters: Counters,
    _io_timeout: std::time::Duration,
    handler: Handler,
) -> Result<()> {
    // 1) Bind a local listener on 127.0.0.1:0 where Tor will forward to.
    let ln = TcpListener::bind("127.0.0.1:0")?;
    let local_port = ln.local_addr()?.port();

    // 2) Build ADD_ONION command depending on persistence.
    let key_file = std::env::var("RO_HS_KEY_FILE").ok();
    let mut ctrl = CtrlClient::authenticate(tor_ctrl_addr, None)?;
    let service_id = if let Some(ref path) = key_file {
        // Persistent mode
        if Path::new(path).exists() {
            // Reuse existing key (exact string Tor expects, e.g., "ED25519-V3:AAAA...")
            let key = std::fs::read_to_string(path)
                .with_context(|| format!("reading HS key from {}", path))?
                .trim()
                .to_string();
            let cmd = format!(
                "ADD_ONION ED25519-V3:{} Port={},127.0.0.1:{}",
                key, OVERLAY_PORT, local_port
            );
            parse_service_id(ctrl.cmd_multi(&cmd)?)?
        } else {
            // Ask Tor to generate a new key; persist it.
            let cmd = format!(
                "ADD_ONION NEW:ED25519-V3 Port={},127.0.0.1:{}",
                OVERLAY_PORT, local_port
            );
            let lines = ctrl.cmd_multi(&cmd)?;
            let (sid, pk) = parse_sid_and_pk(lines)?;
            // Persist the key exactly as Tor returns it.
            if let Some(parent) = Path::new(path).parent() { std::fs::create_dir_all(parent).ok(); }
            std::fs::write(path, &pk).with_context(|| format!("writing HS key to {}", path))?;
            sid
        }
    } else {
        // Ephemeral mode: discard PK so Tor doesn't send it.
        let cmd = format!(
            "ADD_ONION NEW:ED25519-V3 Port={},127.0.0.1:{} Flags=DiscardPK",
            OVERLAY_PORT, local_port
        );
        parse_service_id(ctrl.cmd_multi(&cmd)?)?
    };

    // Keep a guard so we DEL_ONION on drop.
    let _guard = HsGuard {
        tor_ctrl_addr: tor_ctrl_addr.to_string(),
        service_id: service_id.clone(),
    };

    eprintln!("hidden service available at {}.onion:{}", service_id, OVERLAY_PORT);

    // 3) Accept in a background thread and drive the handler.
    thread::spawn(move || {
        for conn in ln.incoming() {
            match conn {
                Ok(s) => {
                    s.set_read_timeout(Some(Duration::from_secs(30))).ok();
                    s.set_write_timeout(Some(Duration::from_secs(30))).ok();
                    let boxed: Box<dyn ReadWrite + Send> =
                        Box::new(CountingStream::new(s, counters.clone()));
                    (handler)(boxed);
                }
                Err(e) => eprintln!("arti_transport accept error: {e:?}"),
            }
        }
    });

    Ok(())
}

/// RAII guard to cleanly remove the HS on drop.
struct HsGuard {
    tor_ctrl_addr: String,
    service_id: String,
}
impl Drop for HsGuard {
    fn drop(&mut self) {
        if let Ok(mut ctrl) = CtrlClient::authenticate(&self.tor_ctrl_addr, None) {
            let _ = ctrl.cmd_multi(&format!("DEL_ONION {}", self.service_id));
        }
    }
}

fn parse_service_id(lines: Vec<String>) -> Result<String> {
    for l in lines {
        if let Some(rest) = l.strip_prefix("250-ServiceID=") {
            return Ok(rest.to_string());
        }
        if l.starts_with("550") {
            return Err(anyhow!("Tor error: {l}"));
        }
    }
    Err(anyhow!("ADD_ONION missing ServiceID"))
}

fn parse_sid_and_pk(lines: Vec<String>) -> Result<(String, String)> {
    let mut sid: Option<String> = None;
    let mut pk:  Option<String> = None;
    for l in lines {
        if let Some(rest) = l.strip_prefix("250-ServiceID=") {
            sid = Some(rest.to_string());
        }
        if let Some(rest) = l.strip_prefix("250-PrivateKey=") {
            pk = Some(rest.to_string()); // e.g., "ED25519-V3:AAAA..."
        }
        if l.starts_with("550") {
            return Err(anyhow!("Tor error: {l}"));
        }
    }
    Ok((
        sid.ok_or_else(|| anyhow!("ADD_ONION new missing ServiceID"))?,
        pk.ok_or_else(|| anyhow!("ADD_ONION new missing PrivateKey"))?,
    ))
}
