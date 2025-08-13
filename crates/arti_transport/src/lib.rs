#![forbid(unsafe_code)]
//! arti_transport: outbound via SOCKS5 (Tor/Arti compatible) and a minimal
//! control-port helper to publish a v3 hidden service (ephemeral by default,
//! or persistent if RO_HS_KEY_FILE is set).

use accounting::{Counters, CountingStream};
use anyhow::{anyhow, bail, Context, Result};
use socks::Socks5Stream;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use tracing::info;
use transport::{Handler, ReadWrite, Transport};

/// Transport over Tor/Arti (SOCKS5 + Tor control-port).
pub struct ArtiTransport {
    counters: Counters,
    socks_addr: String,
    tor_ctrl_addr: String,
    connect_timeout: Duration,
}

impl ArtiTransport {
    /// Create a new `ArtiTransport`.
    ///
    /// - `socks_addr`: e.g., `"127.0.0.1:9050"`
    /// - `tor_ctrl_addr`: e.g., `"127.0.0.1:9051"`
    /// - `connect_timeout`: per-stream I/O timeout
    pub fn new(socks_addr: String, tor_ctrl_addr: String, connect_timeout: Duration) -> Self {
        Self {
            counters: Counters::new(),
            socks_addr,
            tor_ctrl_addr,
            connect_timeout,
        }
    }

    /// Optional: expose counters for periodic logging by the caller.
    pub fn counters(&self) -> Counters {
        self.counters.clone()
    }

    // ===== Tor control-port helpers =====

    fn ctrl_authenticate(&self, cookie_override: Option<&str>) -> Result<TcpStream> {
        let s = TcpStream::connect(&self.tor_ctrl_addr)?;
        s.set_nodelay(true).ok();
        let mut r = BufReader::new(s.try_clone()?);

        let mut send_cmd = |cmd: &str| -> Result<String> {
            let mut w = s.try_clone()?;
            w.write_all(cmd.as_bytes())?;
            w.write_all(b"\r\n")?;
            w.flush()?;
            let mut line = String::new();
            r.read_line(&mut line)?;
            Ok(line)
        };

        let is_ok = |line: &str| line.starts_with("250 OK");

        // Try cookie-auth without parameter first (some Tor builds allow it).
        let mut last_resp = send_cmd("AUTHENTICATE")?;
        let mut authed = is_ok(&last_resp);

        if !authed {
            // Query cookie path, then hex-encode cookie and retry.
            last_resp = send_cmd("PROTOCOLINFO 1")?;
            let cookie_path = if let Some(p) = cookie_override {
                p.to_string()
            } else {
                match last_resp.split_whitespace().find(|w| w.starts_with("COOKIEFILE=")) {
                    Some(kv) => kv.trim_start_matches("COOKIEFILE=").trim_matches('"').to_string(),
                    None => return Err(anyhow!("Tor PROTOCOLINFO missing COOKIEFILE")),
                }
            };
            let cookie = std::fs::read(&cookie_path)
                .with_context(|| format!("reading cookie {}", cookie_path))?;
            let cookie_hex = cookie.iter().map(|b| format!("{:02X}", b)).collect::<String>();
            last_resp = send_cmd(&format!("AUTHENTICATE {}", cookie_hex))?;
            authed = is_ok(&last_resp);
        }

        if !authed {
            bail!("AUTHENTICATE failed: {last_resp:?}");
        }
        Ok(s)
    }

    fn ctrl_cmd_multi(&self, ctrl: &mut TcpStream, cmd: &str) -> Result<Vec<String>> {
        let mut r = BufReader::new(ctrl.try_clone()?);
        {
            let mut w = ctrl.try_clone()?;
            w.write_all(cmd.as_bytes())?;
            w.write_all(b"\r\n")?;
            w.flush()?;
        }
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            r.read_line(&mut line)?;
            if line.is_empty() {
                break;
            }
            let trimmed = line.trim_end().to_string();
            let done = trimmed.starts_with("250 OK")
                || trimmed.starts_with("250 ")
                || trimmed.starts_with("550");
            lines.push(trimmed);
            if done {
                break;
            }
        }
        Ok(lines)
    }
}

/// RAII guard to cleanly remove the HS on drop.
struct HsGuard {
    tor_ctrl_addr: String,
    service_id: String,
}
impl Drop for HsGuard {
    fn drop(&mut self) {
        // Build a tiny ArtiTransport just to reuse its ctrl helpers.
        let helper = ArtiTransport::new(
            String::new(),                 // socks not used here
            self.tor_ctrl_addr.clone(),
            Duration::from_secs(5),
        );
        if let Ok(mut ctrl) = helper.ctrl_authenticate(None) {
            let _ = helper.ctrl_cmd_multi(&mut ctrl, &format!("DEL_ONION {}", self.service_id));
        }
    }
}

impl Transport for ArtiTransport {
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>> {
        // addr: "host:port" — host may be FQDN or .onion
        let (host, port) = {
            let mut parts = addr.rsplitn(2, ':');
            let p = parts.next().ok_or_else(|| anyhow!("missing :port in addr"))?;
            let h = parts.next().ok_or_else(|| anyhow!("missing host in addr"))?;
            (h.to_string(), p.parse::<u16>().context("parsing port")?)
        };
        // socks expects (&str, u16), not (String, u16)
        let stream =
            Socks5Stream::connect(self.socks5_addr().as_str(), (host.as_str(), port))?.into_inner();
        stream.set_read_timeout(Some(self.connect_timeout)).ok();
        stream.set_write_timeout(Some(self.connect_timeout)).ok();
        Ok(Box::new(CountingStream::new(stream, self.counters.clone())))
    }

    /// Listen by publishing a v3 hidden service.
    ///
    /// - **Ephemeral (default):** if `RO_HS_KEY_FILE` env var is **unset**.
    /// - **Persistent:** if `RO_HS_KEY_FILE` points to a file; we reuse it if present,
    ///   otherwise we request a new key from Tor and write it to that path.
    ///
    /// A clean `DEL_ONION` is sent on drop (best effort).
    fn listen(&self, handler: Handler) -> Result<()> {
        // 1) Bind a local listener on 127.0.0.1:0 where Tor will forward to.
        let ln = TcpListener::bind("127.0.0.1:0")?;
        let local_port = ln.local_addr()?.port();

        // 2) Build ADD_ONION command depending on persistence.
        let key_file = std::env::var("RO_HS_KEY_FILE").ok();
        let mut ctrl = self.ctrl_authenticate(None)?;

        let service_id = if let Some(ref path) = key_file {
            // Persistent mode
            if std::path::Path::new(path).exists() {
                // Reuse existing key (exact string Tor expects, e.g., "ED25519-V3:AAAA...")
                let key = std::fs::read_to_string(path)
                    .with_context(|| format!("reading HS key from {}", path))?
                    .trim()
                    .to_string();
                let cmd = format!(
                    "ADD_ONION ED25519-V3:{} Port=1777,127.0.0.1:{}",
                    key, local_port
                );
                let lines = self.ctrl_cmd_multi(&mut ctrl, &cmd)?;
                let mut sid: Option<String> = None;
                for l in &lines {
                    if let Some(rest) = l.strip_prefix("250-ServiceID=") {
                        sid = Some(rest.to_string());
                        break;
                    }
                }
                sid.ok_or_else(|| anyhow!("ADD_ONION reuse missing ServiceID"))?
            } else {
                // Ask Tor to generate a new key; persist it.
                let cmd = format!(
                    "ADD_ONION NEW:ED25519-V3 Port=1777,127.0.0.1:{}",
                    local_port
                );
                let lines = self.ctrl_cmd_multi(&mut ctrl, &cmd)?;
                let mut sid: Option<String> = None;
                let mut pk: Option<String> = None;
                for l in &lines {
                    if let Some(rest) = l.strip_prefix("250-ServiceID=") {
                        sid = Some(rest.to_string());
                    }
                    if let Some(rest) = l.strip_prefix("250-PrivateKey=") {
                        pk = Some(rest.to_string()); // e.g., "ED25519-V3:AAAA..."
                    }
                }
                let pk = pk.ok_or_else(|| anyhow!("ADD_ONION new missing PrivateKey"))?;
                // Persist the key exactly as Tor returns it.
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(path, &pk)
                    .with_context(|| format!("writing HS key to {}", path))?;
                sid.ok_or_else(|| anyhow!("ADD_ONION new missing ServiceID"))?
            }
        } else {
            // Ephemeral mode: discard PK so Tor doesn't send it.
            let cmd = format!(
                "ADD_ONION NEW:ED25519-V3 Port=1777,127.0.0.1:{} Flags=DiscardPK",
                local_port
            );
            let lines = self.ctrl_cmd_multi(&mut ctrl, &cmd)?;
            let mut sid: Option<String> = None;
            for l in &lines {
                if let Some(rest) = l.strip_prefix("250-ServiceID=") {
                    sid = Some(rest.to_string());
                    break;
                }
            }
            sid.ok_or_else(|| anyhow!("ADD_ONION ephemeral missing ServiceID"))?
        };

        info!("hidden service available at {}.onion:1777", service_id);

        // Keep a guard so we DEL_ONION on drop.
        let _guard = HsGuard {
            tor_ctrl_addr: self.tor_ctrl_addr.clone(),
            service_id: service_id.clone(),
        };

        // 3) Accept in a background thread and drive the handler.
        let ctrs = self.counters.clone();
        thread::spawn(move || {
            for conn in ln.incoming() {
                match conn {
                    Ok(s) => {
                        s.set_read_timeout(Some(Duration::from_secs(30))).ok();
                        s.set_write_timeout(Some(Duration::from_secs(30))).ok();
                        let boxed: Box<dyn ReadWrite + Send> =
                            Box::new(CountingStream::new(s, ctrs.clone()));
                        (handler)(boxed);
                    }
                    Err(e) => eprintln!("arti_transport accept error: {e:?}"),
                }
            }
        });

        Ok(())
    }
}

// tiny helper so we can borrow a &str for socks
impl ArtiTransport {
    fn socks5_addr(&self) -> &String {
        &self.socks_addr
    }
}
