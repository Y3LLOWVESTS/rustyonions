#![forbid(unsafe_code)]
//! transport: developer transports and a tiny `Transport` trait.
//!
//! - `Transport` trait abstracts how we connect/listen at the byte-stream level.
//! - `SmallMsgTransport` is a dev-only TCP transport suitable for LAN/local tests.
//!
//! Higher layers (overlay) handle encryption and framing.

use accounting::{Counters, CountingStream};
use anyhow::{bail, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{error, info};

/// A dyn-erasable byte stream used by transports.
pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// Callback invoked for each accepted inbound connection.
/// The handler receives ownership of a boxed bidirectional stream.
pub type Handler = Arc<dyn Fn(Box<dyn ReadWrite + Send>) + Send + Sync>;

/// Minimal transport interface used by higher layers.
pub trait Transport {
    /// Connect to a remote address and yield a byte stream.
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>>;

    /// Begin accepting inbound connections and invoke `handler` for each.
    ///
    /// Implementations may return an error if inbound is not supported.
    fn listen(&self, _handler: Handler) -> Result<()> {
        bail!("inbound listening not implemented for this transport")
    }
}

/** A tiny developer transport over TCP.

This is **development-only** plumbing used to glue nodes together on a
single host or LAN while overlay protocols mature.

- `listen` spawns a thread accepting inbound TCP and invokes your `Handler`.
- `connect` dials a remote and yields a boxed `ReadWrite` stream.
- `send_small` writes a single small message and returns immediately. */
pub struct SmallMsgTransport {
    inbox: String,
    ctrs: Counters,
}

const IO_TIMEOUT: Duration = Duration::from_secs(30);

impl SmallMsgTransport {
    pub fn new(listen_addr: String) -> Self {
        Self {
            inbox: listen_addr,
            ctrs: Counters::new(),
        }
    }

    /// Send a one-shot small message (dev helper).
    pub fn send_small(&self, addr: &str, bytes: &[u8]) -> Result<()> {
        let mut s = TcpStream::connect(addr)?;
        s.set_nodelay(true).ok();
        s.set_read_timeout(Some(IO_TIMEOUT)).ok();
        s.set_write_timeout(Some(IO_TIMEOUT)).ok();
        s.write_all(bytes)?;
        s.flush()?;
        Ok(())
    }

    /// Start a background thread accepting inbound connections and invoking `handler`.
    pub fn listen(&self, handler: Handler) -> Result<()> {
        let addr = self.inbox.clone();
        let listener = TcpListener::bind(&addr)?;
        info!("transport inbox listening on {}", addr);

        thread::spawn(move || loop {
            match listener.accept() {
                Ok((stream, peer)) => {
                    info!("inbox connection from {}", peer);
                    let boxed: Box<dyn ReadWrite + Send> = Box::new(stream);
                    let h = handler.clone();
                    thread::spawn(move || h(boxed));
                }
                Err(e) => error!("inbox accept: {e:?}"),
            }
        });
        Ok(())
    }
}

impl Transport for SmallMsgTransport {
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>> {
        let s = TcpStream::connect(addr)?;
        s.set_nodelay(true).ok();
        s.set_read_timeout(Some(IO_TIMEOUT)).ok();
        s.set_write_timeout(Some(IO_TIMEOUT)).ok();
        Ok(Box::new(CountingStream::new(s, self.ctrs.clone())))
    }

    fn listen(&self, handler: Handler) -> Result<()> {
        SmallMsgTransport::listen(self, handler)
    }
}
