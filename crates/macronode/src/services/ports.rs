// crates/macronode/src/services/ports.rs

//! RO:WHAT — Canonical default bind addresses for macronode-managed planes.
//! RO:WHY  — Prevent port collisions and keep defaults consistent across services.
//!
//! RO:INVARIANTS —
//!   - Defaults are local-only (127.0.0.1) in this slice.
//!   - Index + Mailbox must never share a default port.
//!   - Constants are strings to avoid const-eval limitations across toolchains.

#![forbid(unsafe_code)]

use std::net::SocketAddr;

/// Default bind for overlay plane (stub worker in this slice).
pub const DEFAULT_OVERLAY_ADDR_STR: &str = "127.0.0.1:5301";

/// Default bind for DHT plane (stub worker in this slice).
pub const DEFAULT_DHT_ADDR_STR: &str = "127.0.0.1:5302";

/// Default bind for storage plane (embedded svc-storage HTTP server).
pub const DEFAULT_STORAGE_ADDR_STR: &str = "127.0.0.1:5303";

/// Default bind for index plane (embedded svc-index HTTP server).
pub const DEFAULT_INDEX_BIND_STR: &str = "127.0.0.1:5304";

/// Default bind for mailbox plane (stub worker in this slice).
///
/// IMPORTANT: must not collide with DEFAULT_INDEX_BIND_STR.
pub const DEFAULT_MAILBOX_ADDR_STR: &str = "127.0.0.1:5305";

/// Default bind for gateway plane (svc-gateway ingress).
pub const DEFAULT_GATEWAY_ADDR_STR: &str = "127.0.0.1:8090";

#[inline]
pub fn parse_addr(s: &str) -> SocketAddr {
    s.parse()
        .unwrap_or_else(|_| panic!("invalid SocketAddr literal in ports.rs: {s}"))
}

#[inline]
pub fn default_overlay_addr() -> SocketAddr {
    parse_addr(DEFAULT_OVERLAY_ADDR_STR)
}

#[inline]
pub fn default_dht_addr() -> SocketAddr {
    parse_addr(DEFAULT_DHT_ADDR_STR)
}

#[inline]
pub fn default_storage_addr() -> SocketAddr {
    parse_addr(DEFAULT_STORAGE_ADDR_STR)
}

#[inline]
pub fn default_index_addr() -> SocketAddr {
    parse_addr(DEFAULT_INDEX_BIND_STR)
}

#[inline]
pub fn default_mailbox_addr() -> SocketAddr {
    parse_addr(DEFAULT_MAILBOX_ADDR_STR)
}

#[inline]
pub fn default_gateway_addr() -> SocketAddr {
    parse_addr(DEFAULT_GATEWAY_ADDR_STR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_do_not_collide_index_vs_mailbox() {
        assert_ne!(
            default_index_addr(),
            default_mailbox_addr(),
            "index and mailbox default bind addresses must not collide"
        );
    }
}
