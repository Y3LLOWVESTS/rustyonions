//! RO:WHAT — Common types/aliases for transport.
//! RO:WHY  — Keep the public surface small & stable; define bus events.

use bytes::Bytes;
use std::net::SocketAddr;

/// Owned frame bytes on hot paths (upper layers decode OAP/1).
pub type FrameBytes = Bytes;

/// Event type emitted on the kernel bus for observability/supervision.
#[derive(Debug, Clone)]
pub enum TransportEvent {
    Connected {
        peer: SocketAddr,
        name: &'static str,
    },
    Disconnected {
        peer: SocketAddr,
        name: &'static str,
        reason: Option<String>,
    },
}
