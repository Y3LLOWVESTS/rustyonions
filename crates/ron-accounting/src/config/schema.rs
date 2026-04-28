//! RO:WHAT — Serde config schema for ron-accounting core/export/WAL/HTTP knobs.
//! RO:WHY — Pillar 12; Concerns: RES/GOV/DX. Typed config prevents runtime drift.
//! RO:INTERACTS — config::validate, recorder, exporter lanes, WAL, future HTTP adapter.
//! RO:INVARIANTS — defaults are bounded; unknown fields rejected by host parser discipline.
//! RO:METRICS — metrics sampling config protects label cardinality.
//! RO:CONFIG — mirrors docs/CONFIG.MD and RON_ACC_* variables.
//! RO:SECURITY — amnesia and TLS/WAL fields live here for validation.
//! RO:TEST — config tests in later batch.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level ron-accounting configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Core accounting recorder/window settings.
    pub accounting: AccountingConfig,
    /// Ordered exporter settings.
    pub exporter: ExporterConfig,
    /// WAL settings; disabled automatically when accounting.amnesia=true.
    pub wal: WalConfig,
    /// Optional HTTP/OAP exporter adapter settings.
    pub export_http: ExportHttpConfig,
}

/// Core accounting settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AccountingConfig {
    /// Fixed window length in seconds. Restart required to change.
    pub window_len_s: u32,
    /// Hot-path recorder shard count. Must be power-of-two.
    pub shards: u32,
    /// Maximum distinct rows held in memory.
    pub capacity_rows: u64,
    /// Maximum pending sealed slices awaiting export.
    pub pending_slices_cap: u32,
    /// RAM-only posture; disables WAL persistence.
    pub amnesia: bool,
    /// Cross-tenant fairness strategy.
    pub fairness: Fairness,
    /// Metrics sampling/cardinality settings.
    pub metrics: MetricsSamplingConfig,
}

impl Default for AccountingConfig {
    fn default() -> Self {
        Self {
            window_len_s: 300,
            shards: 64,
            capacity_rows: 200_000,
            pending_slices_cap: 8_192,
            amnesia: false,
            fairness: Fairness::RoundRobin,
            metrics: MetricsSamplingConfig::default(),
        }
    }
}

/// Metrics cardinality protection settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsSamplingConfig {
    /// Whether backlog gauges may sample `(tenant,dimension)` labels.
    pub sample_backlog_labels: bool,
}

impl Default for MetricsSamplingConfig {
    fn default() -> Self {
        Self {
            sample_backlog_labels: true,
        }
    }
}

/// Export fairness policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fairness {
    /// Round-robin streams.
    RoundRobin,
    /// Weighted fair queueing placeholder for future batch.
    Wfq,
}

/// Ordered exporter settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExporterConfig {
    /// Per-stream ordered queue cap.
    pub ordered_buffer_cap: u32,
    /// Retry base backoff in milliseconds.
    pub backoff_base_ms: u32,
    /// Retry maximum backoff cap in milliseconds.
    pub backoff_cap_ms: u32,
    /// Enable full jitter in worker retry loops.
    pub jitter: bool,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            ordered_buffer_cap: 1_024,
            backoff_base_ms: 50,
            backoff_cap_ms: 5_000,
            jitter: true,
        }
    }
}

/// Bounded WAL settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WalConfig {
    /// Enable persistence for sealed-but-unacked slices.
    pub enabled: bool,
    /// WAL directory.
    pub dir: PathBuf,
    /// Maximum WAL bytes.
    pub max_bytes: u64,
    /// Maximum WAL entries.
    pub max_entries: u64,
    /// Maximum staged age in seconds.
    pub max_age_s: u32,
    /// fsync file when segment closes.
    pub fsync_on_close: bool,
    /// fsync directory when creating/renaming segments.
    pub fsync_dir_on_create: bool,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dir: PathBuf::new(),
            max_bytes: 512 * 1_024 * 1_024,
            max_entries: 200_000,
            max_age_s: 86_400,
            fsync_on_close: true,
            fsync_dir_on_create: true,
        }
    }
}

/// Optional HTTP/OAP exporter adapter settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExportHttpConfig {
    /// Enable adapter.
    pub enabled: bool,
    /// Bind address string.
    pub bind_addr: String,
    /// Metrics bind address string.
    pub metrics_addr: String,
    /// Read timeout in milliseconds.
    pub read_timeout_ms: u64,
    /// Write timeout in milliseconds.
    pub write_timeout_ms: u64,
    /// Idle timeout in milliseconds.
    pub idle_timeout_ms: u64,
    /// Request/body hard limits.
    pub limits: HttpLimitsConfig,
    /// TLS settings.
    pub tls: TlsConfig,
}

impl Default for ExportHttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_addr: "127.0.0.1:0".to_string(),
            metrics_addr: "127.0.0.1:0".to_string(),
            read_timeout_ms: 5_000,
            write_timeout_ms: 5_000,
            idle_timeout_ms: 60_000,
            limits: HttpLimitsConfig::default(),
            tls: TlsConfig::default(),
        }
    }
}

/// HTTP/OAP hard limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpLimitsConfig {
    /// Maximum body size; must be <= OAP/1 max_frame=1MiB.
    pub max_body_bytes: u64,
    /// Maximum decompression ratio.
    pub decompress_ratio_cap: u32,
}

impl Default for HttpLimitsConfig {
    fn default() -> Self {
        Self {
            max_body_bytes: 1_048_576,
            decompress_ratio_cap: 10,
        }
    }
}

/// TLS adapter settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct TlsConfig {
    /// Enable TLS.
    pub enabled: bool,
    /// Certificate path.
    pub cert_path: PathBuf,
    /// Private key path.
    pub key_path: PathBuf,
}
