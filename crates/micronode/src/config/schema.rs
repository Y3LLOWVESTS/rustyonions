//! RO:WHAT — Config schema for Micronode.
//! RO:WHY  — Define a typed configuration model (TOML + env overlays)
//!           including server bind options and storage posture.
//! RO:INTERACTS — Parsed from TOML in `config::load`, validated in
//!                `config::validate`, stored in `AppState`.
//! RO:INVARIANTS —
//!   - Defaults are safe and amnesia-first (in-memory storage).
//!   - `StorageEngine::Sled` requires a non-empty `storage.path`
//!     (enforced in `validate`).
//!   - Config is cloneable and sendable across tasks.

use serde::Deserialize;
use std::net::SocketAddr;

/// Top-level Micronode configuration.
///
/// Maps 1:1 to the `micronode.toml` structure:
///
/// ```toml
/// [server]
/// bind = "127.0.0.1:5310"
/// dev_routes = true
///
/// [storage]
/// engine = "mem"
/// # path = "micronode-data"
/// ```
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub server: Server,
    #[serde(default)]
    pub storage: StorageCfg,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    /// Bind address for the Micronode HTTP listener.
    pub bind: SocketAddr,
    /// Whether to expose `/dev/*` routes (echo, etc.).
    #[serde(default)]
    pub dev_routes: bool,
}

/// Storage configuration.
///
/// Beta scope:
/// - `engine = "mem"` — in-memory KV (amnesia-first, no persistence).
/// - `engine = "sled"` — persistent sled-backed KV (requires `path`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StorageCfg {
    /// Storage engine selection, defaults to `"mem"`.
    ///
    /// Serialized as lowercase strings: `"mem"`, `"sled"`.
    #[serde(default)]
    pub engine: StorageEngine,
    /// Optional on-disk path for sled.
    ///
    /// Required (non-empty) when `engine = "sled"`.
    #[serde(default)]
    pub path: Option<String>,
}

/// Storage engine kind.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageEngine {
    /// In-memory store (amnesia-first profile).
    #[default]
    Mem,
    /// Sled-backed KV store (persistent profile).
    Sled,
}

impl Default for Server {
    fn default() -> Self {
        // Same default bind the crate already uses in configs.
        let bind =
            "127.0.0.1:5310".parse().expect("hard-coded default bind must be valid SocketAddr");

        Server { bind, dev_routes: false }
    }
}
