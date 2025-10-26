//! RO:WHAT — Feature-safe TLS ServerConfig alias.
//! This lets the rest of the crate reference `TlsServerConfig` regardless of
//! whether `feature = "tls"` is enabled.

#[cfg(feature = "tls")]
pub type TlsServerConfig = tokio_rustls::rustls::ServerConfig;

#[cfg(not(feature = "tls"))]
pub struct TlsServerConfig;
