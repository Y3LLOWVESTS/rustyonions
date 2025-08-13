#![forbid(unsafe_code)]
//! transport: minimal `Transport` trait + counted TCP transports.
//!
//! - `Transport` abstracts connect/listen at the byte-stream level.
//! - `TcpTransport` is the default counted TCP transport for overlay.
//! - `SmallMsgTransport` remains as a dev-only demo transport.

use accounting::{Counters, CountingStream};
use anyhow::{bail, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{error, info};

/// Combined I/O trait for our transports.
pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// Handler invoked for every accepted inbound connection (Arc so we can clone).
pub type Handler = Arc<dyn Fn(Box<dyn ReadWrite + Send>) + Send + Sync + 'static>;

/// Transport abstraction.
pub trait Transport: Send + Sync {
    /// Dial `addr` and return a counted stream.
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>>;

    /// Accept inbound connections and invoke `handler` for each.
    fn listen(&self, _handler: Handler) -> Result<()> {
        bail!("inbound listening not implemented for this transport")
    }
}

/* ------------------------- TcpTransport (counted) ------------------------- */

const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Counted TCP transport used for overlay TCP mode.
pub struct TcpTransport {
    ctrs: Counters,
    /// Optional explicit bind address for listen(); default "0.0.0.0:0".
    bind_addr: Option<String>,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            ctrs: Counters::new(),
            bind_addr: None,
        }
    }
    pub fn with_bind_addr(addr: String) -> Self {
        Self {
            ctrs: Counters::new(),
            bind_addr: Some(addr),
        }
    }
    pub fn counters(&self) -> Counters {
        self.ctrs.clone()
    }

    fn do_listen(&self, addr: &str, handler: Handler) -> Result<()> {
        let ln = TcpListener::bind(addr)?;
        let local = ln.local_addr()?;
        info!("tcp listening on {}", local);
        let ctrs = self.ctrs.clone();

        // Accept loop on a dedicated thread
        let handler_main = handler.clone();
        thread::spawn(move || loop {
            match ln.accept() {
                Ok((s, peer)) => {
                    s.set_nodelay(true).ok();
                    s.set_read_timeout(Some(IO_TIMEOUT)).ok();
                    s.set_write_timeout(Some(IO_TIMEOUT)).ok();
                    let ctrs2 = ctrs.clone();
                    let h = handler_main.clone();
                    thread::spawn(move || {
                        let boxed = Box::new(CountingStream::new(s, ctrs2));
                        // Wrap the call so it satisfies UnwindSafe requirements.
                        let call = || (h)(boxed);
                        if let Err(e) = std::panic::catch_unwind(AssertUnwindSafe(call)) {
                            error!("handler panic from {peer}: {:?}", e);
                        }
                    });
                }
                Err(e) => error!("accept error: {e:?}"),
            }
        });
        Ok(())
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for TcpTransport {
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>> {
        let s = TcpStream::connect(addr)?;
        s.set_nodelay(true).ok();
        s.set_read_timeout(Some(IO_TIMEOUT)).ok();
        s.set_write_timeout(Some(IO_TIMEOUT)).ok();
        Ok(Box::new(CountingStream::new(s, self.ctrs.clone())))
    }

    fn listen(&self, handler: Handler) -> Result<()> {
        let addr = self.bind_addr.as_deref().unwrap_or("0.0.0.0:0");
        self.do_listen(addr, handler)
    }
}

/* ---------------------- SmallMsgTransport (dev/demo) ---------------------- */

/// Tiny developer transport used for simple local demos.
pub struct SmallMsgTransport {
    inbox: String,
    ctrs: Counters,
}

impl SmallMsgTransport {
    pub fn new(listen_addr: String) -> Self {
        Self {
            inbox: listen_addr,
            ctrs: Counters::new(),
        }
    }
    pub fn counters(&self) -> Counters {
        self.ctrs.clone()
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
        let ln = TcpListener::bind(&self.inbox)?;
        let local = ln.local_addr()?;
        info!("dev inbox listening on {}", local);

        let ctrs = self.ctrs.clone();
        let handler_main = handler.clone();
        thread::spawn(move || loop {
            match ln.accept() {
                Ok((s, peer)) => {
                    s.set_nodelay(true).ok();
                    s.set_read_timeout(Some(IO_TIMEOUT)).ok();
                    s.set_write_timeout(Some(IO_TIMEOUT)).ok();
                    let ctrs2 = ctrs.clone();
                    let h = handler_main.clone();
                    thread::spawn(move || {
                        let boxed = Box::new(CountingStream::new(s, ctrs2));
                        let call = || (h)(boxed);
                        if let Err(e) = std::panic::catch_unwind(AssertUnwindSafe(call)) {
                            error!("handler panic from {peer}: {:?}", e);
                        }
                    });
                }
                Err(e) => error!("accept error: {e:?}"),
            }
        });
        Ok(())
    }
}
