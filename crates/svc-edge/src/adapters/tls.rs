//! TLS adapter placeholders (server config wiring) — stub.

/// TLS mode (placeholder).
#[derive(Debug, Clone, Copy)]
pub enum TlsMode {
    /// Plain TCP (no TLS).
    Plain,
    /// TLS via rustls (details TBD).
    Rustls,
}

impl Default for TlsMode {
    fn default() -> Self {
        TlsMode::Plain
    }
}
