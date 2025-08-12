use accounting::{Counters, CountingStream};
use anyhow::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

/// Callback signature for inbound connections.
pub type Handler = Arc<dyn Fn(Box<dyn ReadWrite + Send>) + Send + Sync>;

/// Dyn-erasable trait for a bidirectional byte stream.
pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// Messages are tiny and already-encrypted at higher layers.
/// The transport just delivers bytes stream-wise.
pub trait SmallMsgTransport: Send + Sync + 'static {
    type Stream: Read + Write + Send + 'static;
    fn dial(&self, to_addr: &str) -> Result<Self::Stream>;
    fn listen(&self, bind: &str, handler: Handler) -> Result<()>;
    fn counters(&self) -> Counters;
}

/// A simple TCP development transport, instrumented with accounting counters.
pub struct TcpDevTransport {
    ctrs: Counters,
}

impl TcpDevTransport {
    pub fn new(window: std::time::Duration) -> Self {
        Self {
            ctrs: Counters::new(window),
        }
    }
}

impl SmallMsgTransport for TcpDevTransport {
    type Stream = CountingStream<TcpStream>;

    fn dial(&self, to_addr: &str) -> Result<Self::Stream> {
        let s = TcpStream::connect(to_addr)?;
        Ok(CountingStream::new(s, self.ctrs.clone()))
    }

    fn listen(&self, bind: &str, handler: Handler) -> Result<()> {
        let listener = TcpListener::bind(bind)?;
        let ctrs = self.ctrs.clone();
        info!("Inbox listening on {bind}");
        thread::spawn(move || {
            for c in listener.incoming() {
                match c {
                    Ok(s) => {
                        let stream = CountingStream::new(s, ctrs.clone());
                        let boxed: Box<dyn ReadWrite + Send> = Box::new(stream);
                        let h = handler.clone();
                        thread::spawn(move || h(boxed));
                    }
                    Err(e) => error!("inbox accept: {e:?}"),
                }
            }
        });
        Ok(())
    }

    fn counters(&self) -> Counters {
        self.ctrs.clone()
    }
}
