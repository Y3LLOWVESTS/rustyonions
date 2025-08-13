#![forbid(unsafe_code)]
//! arti_transport: outbound via SOCKS5 (Tor/Arti compatible) with optional
//! control-port helpers for authentication and commands.

use accounting::{Counters, CountingStream};
use anyhow::{anyhow, bail, Context, Result};
use socks::Socks5Stream;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;
use transport::{Handler, ReadWrite, Transport};

/// Outbound transport over a Tor/Arti SOCKS5 proxy.
pub struct ArtiTransport {
    counters: Counters,
    socks_addr: String,
    connect_timeout: Duration,
}

impl ArtiTransport {
    /// Create a new `ArtiTransport`.
    ///
    /// - `socks_addr`: e.g., `"127.0.0.1:9150"`
    /// - `connect_timeout`: used for read/write timeouts on the proxied stream
    pub fn new(socks_addr: String, connect_timeout: Duration) -> Self {
        Self {
            counters: Counters::new(),
            socks_addr,
            connect_timeout,
        }
    }

    // --- Tor control helpers (AUTHENTICATE, arbitrary command) ---

    /// Authenticate to Tor's control port.
    ///
    /// - If `cookie_override` is provided, that path is used for the cookie file.
    /// - Otherwise we run `PROTOCOLINFO 1` to discover `COOKIEFILE` and use it.
    #[allow(dead_code)]
    pub fn ctrl_authenticate(
        &self,
        ctrl_addr: &str,
        cookie_override: Option<&str>,
    ) -> Result<TcpStream> {
        let s = TcpStream::connect(ctrl_addr)?;
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

        // Try a no-arg AUTHENTICATE first (cookie mode may already be enabled).
        let mut last_resp = send_cmd("AUTHENTICATE")?;
        let mut authed = is_ok(&last_resp);

        if !authed {
            // Fallback to cookie authentication:
            last_resp = send_cmd("PROTOCOLINFO 1")?;
            let cookie_path = if let Some(p) = cookie_override {
                p.to_string()
            } else {
                match last_resp
                    .split_whitespace()
                    .find(|w| w.starts_with("COOKIEFILE="))
                {
                    Some(kv) => kv
                        .trim_start_matches("COOKIEFILE=")
                        .trim_matches('"')
                        .to_string(),
                    None => return Err(anyhow!("Tor PROTOCOLINFO missing COOKIEFILE")),
                }
            };

            let cookie = std::fs::read(cookie_path).context("reading tor auth cookie")?;
            let hex = hex::encode(cookie);
            last_resp = send_cmd(&format!("AUTHENTICATE {}", hex))?;
            authed = is_ok(&last_resp);
        }

        if !authed {
            return Err(anyhow!("Tor control authentication failed: {last_resp:?}"));
        }

        Ok(s)
    }

    /// Send a command over an authenticated control connection; return response lines.
    #[allow(dead_code)]
    pub fn ctrl_command(&self, stream: &TcpStream, cmd: &str) -> Result<Vec<String>> {
        let mut w = stream.try_clone()?;
        w.write_all(cmd.as_bytes())?;
        w.write_all(b"\r\n")?;
        w.flush()?;

        let mut r = BufReader::new(stream.try_clone()?);
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            let n = r.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end().to_string();
            let done = trimmed.starts_with("250 OK") || trimmed.starts_with("550");
            lines.push(trimmed);
            if done {
                break;
            }
        }
        Ok(lines)
    }
}

impl Transport for ArtiTransport {
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>> {
        // Use SOCKS5 proxy to reach `addr` (host:port).
        let s = Socks5Stream::connect(self.socks_addr.as_str(), addr)?;
        let stream = s.into_inner();
        stream.set_nodelay(true).ok();
        stream.set_read_timeout(Some(self.connect_timeout)).ok();
        stream.set_write_timeout(Some(self.connect_timeout)).ok();
        Ok(Box::new(CountingStream::new(stream, self.counters.clone())))
    }

    fn listen(&self, _handler: Handler) -> Result<()> {
        bail!("ArtiTransport inbound listener not implemented (hidden service WIP)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_construct() {
        let t = ArtiTransport::new("127.0.0.1:9150".into(), Duration::from_secs(10));
        let _ = t;
    }
}
