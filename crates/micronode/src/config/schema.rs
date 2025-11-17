//! RO:WHAT — Config schema for Micronode.
//! RO:WHY  — Define a typed configuration model (TOML + env overlays)
//!           including server bind options, storage posture, security mode, and facets.
//! RO:INTERACTS — Parsed from TOML in `config::load`, validated in
//!                `config::validate`, stored in `AppState`.
//! RO:INVARIANTS —
//!   - Defaults are safe and amnesia-first (in-memory storage).
//!   - `StorageEngine::Sled` requires a non-empty `storage.path` (enforced in `validate`).
//!   - Config is cloneable and sendable across tasks.

use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    /// Server settings; defaults to 127.0.0.1:5310 with dev_routes=false
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub storage: StorageCfg,
    /// Security posture (deny-by-default unless explicitly relaxed).
    #[serde(default)]
    pub security: SecurityCfg,
    /// Facet loading configuration.
    #[serde(default)]
    pub facets: FacetsCfg,
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

impl Default for Server {
    fn default() -> Self {
        let bind: SocketAddr =
            "127.0.0.1:5310".parse().expect("hard-coded default bind must be valid SocketAddr");
        Server { bind, dev_routes: false }
    }
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

/// Security configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityCfg {
    /// Security policy for Micronode HTTP surfaces.
    #[serde(default)]
    pub mode: SecurityMode,
}

impl Default for SecurityCfg {
    fn default() -> Self {
        Self { mode: SecurityMode::DenyAll }
    }
}

/// Security enforcement modes.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecurityMode {
    /// Deny all non-admin surfaces unless explicitly allowed.
    #[default]
    DenyAll,
    /// Developer convenience: allow KV/facets without a macaroon.
    DevAllow,
    /// Delegate verification to external auth/policy service (future).
    External,
}

/// Facet loader configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FacetsCfg {
    /// Enable manifest-driven facets.
    #[serde(default)]
    pub enabled: bool,
    /// Directory containing `*.toml` facet manifests.
    #[serde(default)]
    pub dir: Option<String>,
}
