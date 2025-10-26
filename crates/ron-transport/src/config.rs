//! RO:WHAT — Transport configuration (bind, ceilings, timeouts).
//! RO:WHY  — Hard caps & deadlines enforce SEC/RES.
//! RO:INTERACTS — limits, tcp::listener/dialer, tls::{server,client}.
//! RO:INVARIANTS — immutable at runtime; values bounded; amnesia-safe.
//! RO:CONFIG — From env or files (upstream); this struct is runtime snapshot.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Bind address for the listener (e.g., "127.0.0.1:9400").
    pub addr: SocketAddr,
    /// Human-readable transport name (for metrics labels).
    pub name: &'static str,
    /// Maximum concurrent connections allowed.
    pub max_conns: usize,
    /// Read timeout per I/O op.
    pub read_timeout: Duration,
    /// Write timeout per I/O op.
    pub write_timeout: Duration,
    /// Idle timeout (no traffic).
    pub idle_timeout: Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:0".parse().unwrap(),
            name: "tcp",
            max_conns: 1024,
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(15),
        }
    }
}
