//! Async transport abstraction with TCP and Tor backends.
//!
//! This module also provides *compatibility shims* for older code:
//! - `transport::ReadWrite` (alias trait for an async stream)
//! - `transport::Handler`   (generic connection handler trait)

use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

// Public submodules
pub mod tcp;
pub mod tor;

/// Convenient alias for any async stream we can read/write.
pub trait IoStream: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> IoStream for T {}

/// Back-compat: expose `ReadWrite` as an alias trait for async IO streams.
/// Old code imported `transport::ReadWrite`; keep that working.
pub trait ReadWrite: IoStream {}
impl<T: IoStream> ReadWrite for T {}

/// Back-compat: a generic connection handler interface.
/// Old code imported `transport::Handler`.
#[async_trait]
pub trait Handler: Send + Sync {
    /// Stream type this handler works with.
    type Stream: IoStream;

    /// Handle an accepted/connected stream. `peer` may be a socket address (if known).
    async fn handle(&self, stream: Self::Stream, peer: SocketAddr) -> Result<()>;
}

/// A listener that can accept inbound connections for a given transport.
#[async_trait]
pub trait TransportListener: Send {
    type Stream: IoStream;

    /// Accept the next inbound connection.
    async fn accept(&mut self) -> Result<(Self::Stream, SocketAddr)>;
}

/// A simple async transport abstraction. Implemented by TCP and Tor.
#[async_trait]
pub trait Transport: Send + Sync {
    type Stream: IoStream;
    type Listener: TransportListener<Stream = Self::Stream>;

    /// Connect to a peer address. For Tor, this may be a `.onion:port`.
    async fn connect(&self, peer_addr: &str) -> Result<Self::Stream>;

    /// Bind a local listener (e.g., `127.0.0.1:1777`).
    async fn listen(&self, bind: SocketAddr) -> Result<Self::Listener>;
}
