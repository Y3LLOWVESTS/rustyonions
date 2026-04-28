//! RO:WHAT — Public configuration surface for ron-accounting.
//! RO:WHY — Pillar 12; Concerns: RES/GOV/DX. Centralizes fail-closed startup validation.
//! RO:INTERACTS — schema, validate, load, recorder, exporter, WAL, future HTTP adapter.
//! RO:INVARIANTS — env/file/default precedence; amnesia disables WAL; restart-only window changes.
//! RO:METRICS — config reload failures counted by future adapter metrics.
//! RO:CONFIG — RON_ACC_* env vars and ron-accounting TOML files.
//! RO:SECURITY — TLS/WAL paths validated before use in later batches.
//! RO:TEST — unit: config tests in later batch; recording tests use defaults.

pub mod load;
pub mod schema;
pub mod validate;

pub use load::{from_env_and_file, from_toml_file, from_toml_str};
pub use schema::{
    AccountingConfig, Config, ExportHttpConfig, ExporterConfig, Fairness, HttpLimitsConfig,
    MetricsSamplingConfig, TlsConfig, WalConfig,
};
pub use validate::{normalize_config, validate};
