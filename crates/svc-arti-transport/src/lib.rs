#![forbid(unsafe_code)]
//! arti_transport: outbound via SOCKS5 (Tor/Arti compatible) and a minimal
//! control-port helper to publish a v3 hidden service (ephemeral by default,
//! or persistent if RO_HS_KEY_FILE is set).

mod socks;
mod ctrl;
mod hs;

use accounting::{Counters};
use std::time::Duration;
use transport::{Handler, ReadWrite, Transport};
use anyhow::Result;

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
}

impl Transport for ArtiTransport {
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>> {
        socks::connect_via_socks(
            &self.socks_addr,
            addr,
            self.connect_timeout,
            self.counters.clone(),
        )
    }

    /// Listen by publishing a v3 hidden service.
    ///
    /// - **Ephemeral (default):** if `RO_HS_KEY_FILE` env var is **unset**.
    /// - **Persistent:** if `RO_HS_KEY_FILE` points to a file; we reuse it if present,
    ///   otherwise we request a new key from Tor and write it to that path.
    ///
    /// A clean `DEL_ONION` is sent on drop (best effort).
    fn listen(&self, handler: Handler) -> Result<()> {
        hs::publish_and_serve(
            &self.tor_ctrl_addr,
            self.counters.clone(),
            self.connect_timeout,
            handler,
        )
    }
}
