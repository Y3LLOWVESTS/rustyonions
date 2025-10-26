//! RO:WHAT — TLS accept wrapper (placeholder).
#![cfg(feature = "tls")]
use tokio_rustls::rustls::ServerConfig;

pub type TlsServerConfig = ServerConfig;
// Integration will wrap TcpStream with TlsAcceptor::from(Arc<ServerConfig>) later.
