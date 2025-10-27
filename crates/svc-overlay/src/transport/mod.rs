//! RO:WHAT — Transport facade for svc-overlay.
//! RO:WHY  — Allow swapping plain TCP for `ron-transport` without touching call-sites.
//! RO:NOTE — For now, BOTH feature paths use Tokio TCP. When we align with the real
//!           `ron-transport` API, only this file needs changes.

use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub struct Listener {
    inner: TcpListener,
}

pub struct TransportStream {
    pub inner: TcpStream,
}

impl TransportStream {
    pub fn into_split(
        self,
    ) -> (
        tokio::net::tcp::OwnedReadHalf,
        tokio::net::tcp::OwnedWriteHalf,
    ) {
        self.inner.into_split()
    }
}

pub async fn bind_listener(addr: SocketAddr) -> std::io::Result<(Listener, SocketAddr)> {
    let inner = TcpListener::bind(addr).await?;
    let local = inner.local_addr()?;
    Ok((Listener { inner }, local))
}

impl Listener {
    pub async fn accept(&self) -> std::io::Result<(TransportStream, SocketAddr)> {
        let (sock, peer) = self.inner.accept().await?;
        Ok((TransportStream { inner: sock }, peer))
    }
}
