// crates/macronode/src/services/ports.rs

//! RO:WHAT — Canonical CrabNode default bind addresses and collision-free service topology.
//! RO:WHY — CN-2 requires one authoritative port map before full gateway/Omnigate composition.
//! RO:INTERACTS — macronode admin, svc-gateway, Omnigate, svc-passport, overlay, DHT, storage, index, mailbox.
//! RO:INVARIANTS — canonical ports are unique; admin is loopback; client ingress is 8090.
//! RO:CONFIG — historical env overrides remain supported by individual service owners.
//! RO:SECURITY — admin and internal defaults remain loopback; public exposure is explicit later.
//! RO:TEST — services::ports::tests proves the canonical map is collision-free.

#![forbid(unsafe_code)]

use std::net::SocketAddr;

/// Default bind for the private CrabNode operator/admin plane.
pub const DEFAULT_ADMIN_ADDR_STR: &str = "127.0.0.1:8080";

/// Optional local svc-admin UI bind. The UI is never required for CrabNode runtime.
pub const DEFAULT_SVC_ADMIN_ADDR_STR: &str = "127.0.0.1:5300";

/// Canonical internal Omnigate product/BFF listener.
pub const DEFAULT_OMNIGATE_ADDR_STR: &str = "127.0.0.1:9090";

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

/// Canonical internal svc-passport public-profile listener.
pub const DEFAULT_PASSPORT_ADDR_STR: &str = "127.0.0.1:5307";

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
    use std::collections::BTreeSet;

    #[test]
    fn defaults_do_not_collide_index_vs_mailbox() {
        assert_ne!(
            default_index_addr(),
            default_mailbox_addr(),
            "index and mailbox default bind addresses must not collide"
        );
    }

    #[test]
    fn cn2_canonical_crabnode_port_map_is_collision_free() {
        let entries = [
            ("admin", DEFAULT_ADMIN_ADDR_STR),
            ("svc-admin", DEFAULT_SVC_ADMIN_ADDR_STR),
            ("overlay", DEFAULT_OVERLAY_ADDR_STR),
            ("dht", DEFAULT_DHT_ADDR_STR),
            ("storage", DEFAULT_STORAGE_ADDR_STR),
            ("index", DEFAULT_INDEX_BIND_STR),
            ("mailbox", DEFAULT_MAILBOX_ADDR_STR),
            ("gateway", DEFAULT_GATEWAY_ADDR_STR),
            ("omnigate", DEFAULT_OMNIGATE_ADDR_STR),
            ("passport", DEFAULT_PASSPORT_ADDR_STR),
        ];

        let mut addresses = BTreeSet::new();

        for (service, raw) in entries {
            let addr = parse_addr(raw);

            assert!(
                addr.ip().is_loopback(),
                "{service} canonical CN-2 default must remain loopback before explicit public exposure"
            );

            assert!(
                addresses.insert(addr),
                "canonical CN-2 port collision at {service}={addr}"
            );
        }

        assert_eq!(DEFAULT_ADMIN_ADDR_STR, "127.0.0.1:8080");
        assert_eq!(DEFAULT_GATEWAY_ADDR_STR, "127.0.0.1:8090");
        assert_eq!(DEFAULT_OMNIGATE_ADDR_STR, "127.0.0.1:9090");
        assert_eq!(DEFAULT_OVERLAY_ADDR_STR, "127.0.0.1:5301");
        assert_eq!(DEFAULT_DHT_ADDR_STR, "127.0.0.1:5302");
        assert_eq!(DEFAULT_STORAGE_ADDR_STR, "127.0.0.1:5303");
        assert_eq!(DEFAULT_INDEX_BIND_STR, "127.0.0.1:5304");
        assert_eq!(DEFAULT_MAILBOX_ADDR_STR, "127.0.0.1:5305");
        assert_eq!(DEFAULT_PASSPORT_ADDR_STR, "127.0.0.1:5307");
    }
}
