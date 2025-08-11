use accounting::{Counters, CountingStream};
use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::net::TcpStream; // placeholder; will become an Arti stream
use transport::{Handler, SmallMsgTransport};

/// Placeholder Arti transport that satisfies the trait.
/// We'll replace TcpStream with Arti streams and real logic next.
pub struct ArtiTransport {
    counters: Counters,
}

impl ArtiTransport {
    pub fn new(window: std::time::Duration) -> Self {
        Self { counters: Counters::new(window) }
    }

    pub fn onion_address(&self) -> Result<String> {
        Err(anyhow!("Arti not wired yet (enable feature 'tor' in next step)"))
    }
}

impl SmallMsgTransport for ArtiTransport {
    type Stream = CountingStream<TcpStream>;

    fn dial(&self, _to_addr: &str) -> Result<Self::Stream> {
        Err(anyhow!("Arti dial not implemented yet"))
    }

    fn listen(&self, _bind: &str, _handler: Handler) -> Result<()> {
        Err(anyhow!("Arti hidden-service inbox not implemented yet"))
    }

    fn counters(&self) -> Counters {
        self.counters.clone()
    }
}
