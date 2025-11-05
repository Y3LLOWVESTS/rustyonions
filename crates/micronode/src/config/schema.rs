//! RO:WHAT — Micronode config schema and defaults.
//! RO:WHY  — Deterministic config with sane hardening defaults.
//! RO:INVARIANTS — Defaults truthful; amnesia honored at higher layers later.

use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub bind: SocketAddr,
    pub dev_routes: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Server {
                bind: "127.0.0.1:5310".parse().expect("default bind"),
                dev_routes: false,
            },
        }
    }
}
