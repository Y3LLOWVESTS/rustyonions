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
    /// Passive CrabLink-managed user-node runtime posture.
    #[serde(default)]
    pub user_node: UserNodeCfg,
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

/// Passive user-node runtime configuration.
///
/// This is Phase 6A contract state: it makes Micronode report and validate
/// the posture CrabLink will later manage as a sidecar/background process.
/// The workers are status-visible and stubbed; they do not mutate wallet,
/// ledger, rewards, or confirmed ROC.
#[derive(Debug, Clone, Deserialize)]
pub struct UserNodeCfg {
    /// Enables the passive CrabLink-managed runtime posture.
    #[serde(default = "default_true")]
    pub passive_runtime_enabled: bool,
    /// Enables the local verification queue surface.
    #[serde(default = "default_true")]
    pub verification_queue_enabled: bool,
    /// Enables the read-only economic replay worker surface.
    #[serde(default = "default_true")]
    pub economic_replay_worker_enabled: bool,
    /// Resource profile CrabLink may select later.
    #[serde(default)]
    pub resource_mode: ResourceMode,
    /// Maximum intended background CPU percentage for the passive worker slice.
    #[serde(default = "default_max_cpu_percent")]
    pub max_cpu_percent: u8,
    /// Maximum intended background bandwidth in KiB/s.
    #[serde(default = "default_max_background_kbps")]
    pub max_background_kbps: u32,
    /// Bounded pending evidence queue length.
    #[serde(default = "default_pending_evidence_limit")]
    pub pending_evidence_limit: u32,
}

impl Default for UserNodeCfg {
    fn default() -> Self {
        Self {
            passive_runtime_enabled: true,
            verification_queue_enabled: true,
            economic_replay_worker_enabled: true,
            resource_mode: ResourceMode::Balanced,
            max_cpu_percent: default_max_cpu_percent(),
            max_background_kbps: default_max_background_kbps(),
            pending_evidence_limit: default_pending_evidence_limit(),
        }
    }
}

/// CrabLink user-node resource profile.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMode {
    /// Minimal work; suitable for battery or metered-network mode.
    Low,
    /// Default bounded background mode.
    #[default]
    Balanced,
    /// Larger budgets only when the device is plugged in or explicitly allowed.
    PluggedIn,
}

impl ResourceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ResourceMode::Low => "low",
            ResourceMode::Balanced => "balanced",
            ResourceMode::PluggedIn => "plugged_in",
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_cpu_percent() -> u8 {
    5
}

fn default_max_background_kbps() -> u32 {
    64
}

fn default_pending_evidence_limit() -> u32 {
    1024
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
