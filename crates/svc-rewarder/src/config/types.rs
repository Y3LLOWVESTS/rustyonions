//! RO:WHAT — Strongly typed configuration model for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF/GOV. Defaults encode safe local service behavior.
//! RO:INTERACTS — config::load, config::validate, main bootstrap, readiness.
//! RO:INVARIANTS — serde deny_unknown_fields; amnesia explicit; no external-chain settings.
//! RO:METRICS — config affects metrics bind and readiness behavior.
//! RO:CONFIG — owns all documented knobs for the service.
//! RO:SECURITY — macaroon_path is a path only; never log secret contents.
//! RO:TEST — tests/unit/config.rs.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

/// Top-level service config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Public HTTP bind address.
    pub bind_addr: SocketAddr,
    /// Optional separate metrics bind address reserved for future split-plane serving.
    pub metrics_addr: SocketAddr,
    /// Max accepted TCP connections once listener-level admission is wired.
    pub max_conns: usize,
    /// Read timeout string.
    pub read_timeout: String,
    /// Write timeout string.
    pub write_timeout: String,
    /// Idle timeout string.
    pub idle_timeout: String,
    /// TLS config.
    pub tls: TlsConfig,
    /// Size/ratio caps.
    pub limits: LimitsConfig,
    /// Rewarder domain config.
    pub rewarder: RewarderConfig,
    /// Downstream endpoints and cap paths.
    pub ingress: IngressConfig,
    /// Worker/concurrency bounds.
    pub concurrency: ConcurrencyConfig,
    /// Shard config for future horizontal split.
    pub shard: ShardConfig,
    /// Amnesia mode.
    pub amnesia: AmnesiaConfig,
    /// PQ posture.
    pub pq: PqConfig,
    /// Logging config.
    pub log: LogConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 8090)),
            metrics_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            max_conns: 1024,
            read_timeout: "5s".into(),
            write_timeout: "5s".into(),
            idle_timeout: "60s".into(),
            tls: TlsConfig::default(),
            limits: LimitsConfig::default(),
            rewarder: RewarderConfig::default(),
            ingress: IngressConfig::default(),
            concurrency: ConcurrencyConfig::default(),
            shard: ShardConfig::default(),
            amnesia: AmnesiaConfig::default(),
            pq: PqConfig::default(),
            log: LogConfig::default(),
        }
    }
}

/// TLS toggle and path refs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct TlsConfig {
    /// Enables direct TLS serving once the TLS feature is wired.
    pub enabled: bool,
    /// PEM certificate path.
    pub cert_path: Option<String>,
    /// PEM private key path.
    pub key_path: Option<String>,
}

/// Request hardening caps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct LimitsConfig {
    /// Post-inflate request body cap.
    pub max_body_bytes: String,
    /// Decompression expansion ratio cap.
    pub decompress_ratio_cap: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_body_bytes: "1MiB".into(),
            decompress_ratio_cap: 10,
        }
    }
}

/// Rewarder computation and artifact config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RewarderConfig {
    /// Epoch window length.
    pub epoch_duration: String,
    /// Default policy id.
    pub policy_id: String,
    /// Snapshot/policy cache TTL.
    pub inputs_cache_ttl: String,
    /// Allowed clock skew for epochs.
    pub max_epoch_skew: String,
    /// Domain separation salt for run_key generation.
    pub idempotency_salt: String,
    /// Artifact output directory when amnesia is off.
    pub artifact_dir: String,
    /// Artifact retention horizon.
    pub retain_runs: String,
    /// Future zk proof toggle.
    pub enable_zk_proofs: bool,
}

impl Default for RewarderConfig {
    fn default() -> Self {
        Self {
            epoch_duration: "1h".into(),
            policy_id: "policy:v1".into(),
            inputs_cache_ttl: "5m".into(),
            max_epoch_skew: "2m".into(),
            idempotency_salt: "svc-rewarder|v1".into(),
            artifact_dir: "/var/run/svc-rewarder/artifacts".into(),
            retain_runs: "24h".into(),
            enable_zk_proofs: false,
        }
    }
}

/// Downstream value-plane endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct IngressConfig {
    /// Accounting snapshot service base URL.
    pub accounting_base_url: String,
    /// Wallet service base URL. Rewarder targets wallet as the mutation boundary.
    pub wallet_base_url: String,
    /// Wallet issue route path.
    pub wallet_issue_path: String,
    /// Capability scope needed for future wallet issue egress.
    pub wallet_cap_scope: String,
    /// Legacy ledger or wallet service base URL. Kept as a compatibility seam during transition.
    pub ledger_base_url: String,
    /// Policy service base URL.
    pub policy_base_url: String,
    /// Macaroon path for downstream intent emission.
    pub macaroon_path: String,
}

impl Default for IngressConfig {
    fn default() -> Self {
        Self {
            accounting_base_url: "http://127.0.0.1:7101".into(),
            wallet_base_url: "http://127.0.0.1:8088".into(),
            wallet_issue_path: "/v1/issue".into(),
            wallet_cap_scope: "wallet.issue".into(),
            ledger_base_url: "http://127.0.0.1:7201".into(),
            policy_base_url: "http://127.0.0.1:7301".into(),
            macaroon_path: String::new(),
        }
    }
}

/// Bounded worker and queue config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ConcurrencyConfig {
    /// CPU compute workers.
    pub compute_workers: usize,
    /// IO inflight permits.
    pub io_inflight: usize,
    /// Work queue bound.
    pub work_queue_max: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            compute_workers: 4,
            io_inflight: 64,
            work_queue_max: 512,
        }
    }
}

/// Future sharding strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ShardConfig {
    /// `single`, `by_actor`, or `by_content`.
    pub strategy: String,
    /// Number of shards.
    pub shards: usize,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            strategy: "single".into(),
            shards: 1,
        }
    }
}

/// Amnesia mode config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct AmnesiaConfig {
    /// RAM-first/no-disk posture.
    pub enabled: bool,
}

impl Default for AmnesiaConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// PQ posture config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PqConfig {
    /// `off` or `hybrid` for current service posture.
    pub mode: String,
}

impl Default for PqConfig {
    fn default() -> Self {
        Self { mode: "off".into() }
    }
}

/// Logging config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    /// `json` or `text`.
    pub format: String,
    /// tracing level.
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: "text".into(),
            level: "info".into(),
        }
    }
}
