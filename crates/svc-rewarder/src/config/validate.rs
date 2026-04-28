//! RO:WHAT — Fail-closed configuration validation for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/SEC/GOV. Prevents unsafe service startup and drift from docs.
//! RO:INTERACTS — config::types, util::bytes, util::timeouts.
//! RO:INVARIANTS — request cap bounded; workers/queues non-zero; TLS paths complete if enabled.
//! RO:METRICS — startup failure is logged by main.
//! RO:CONFIG — validates service knobs including wallet egress seam.
//! RO:SECURITY — TLS and PQ modes fail closed on inconsistent settings.
//! RO:TEST — tests/unit/config.rs.

use crate::config::types::Config;
use crate::util::{bytes::parse_size_bytes, timeouts::parse_duration};
use crate::{Result, RewarderError};

/// Validate config after all overlays have been applied.
pub fn validate_config(cfg: &Config) -> Result<()> {
    if cfg.max_conns == 0 {
        return Err(RewarderError::Config("max_conns must be > 0".into()));
    }
    for (name, value) in [
        ("read_timeout", &cfg.read_timeout),
        ("write_timeout", &cfg.write_timeout),
        ("idle_timeout", &cfg.idle_timeout),
        ("epoch_duration", &cfg.rewarder.epoch_duration),
        ("inputs_cache_ttl", &cfg.rewarder.inputs_cache_ttl),
        ("max_epoch_skew", &cfg.rewarder.max_epoch_skew),
        ("retain_runs", &cfg.rewarder.retain_runs),
    ] {
        if parse_duration(value)?.is_zero() {
            return Err(RewarderError::Config(format!("{name} must be > 0")));
        }
    }

    let max_body = parse_size_bytes(&cfg.limits.max_body_bytes)?;
    if !(1024..=1_048_576).contains(&max_body) {
        return Err(RewarderError::Config(
            "limits.max_body_bytes must be between 1KiB and 1MiB".into(),
        ));
    }
    if cfg.limits.decompress_ratio_cap == 0 || cfg.limits.decompress_ratio_cap > 10 {
        return Err(RewarderError::Config(
            "limits.decompress_ratio_cap must be in 1..=10".into(),
        ));
    }
    if cfg.concurrency.compute_workers == 0 {
        return Err(RewarderError::Config(
            "concurrency.compute_workers must be > 0".into(),
        ));
    }
    if cfg.concurrency.io_inflight == 0 || cfg.concurrency.work_queue_max == 0 {
        return Err(RewarderError::Config(
            "concurrency.io_inflight and work_queue_max must be > 0".into(),
        ));
    }
    if cfg.tls.enabled && (cfg.tls.cert_path.is_none() || cfg.tls.key_path.is_none()) {
        return Err(RewarderError::Config(
            "tls.enabled requires cert_path and key_path".into(),
        ));
    }

    validate_http_base_url(
        "ingress.accounting_base_url",
        &cfg.ingress.accounting_base_url,
    )?;
    validate_http_base_url("ingress.wallet_base_url", &cfg.ingress.wallet_base_url)?;
    validate_http_base_url("ingress.ledger_base_url", &cfg.ingress.ledger_base_url)?;
    validate_http_base_url("ingress.policy_base_url", &cfg.ingress.policy_base_url)?;

    if !cfg.ingress.wallet_issue_path.starts_with('/') {
        return Err(RewarderError::Config(
            "ingress.wallet_issue_path must start with /".into(),
        ));
    }
    if cfg
        .ingress
        .wallet_issue_path
        .chars()
        .any(char::is_whitespace)
    {
        return Err(RewarderError::Config(
            "ingress.wallet_issue_path must not contain whitespace".into(),
        ));
    }
    if cfg.ingress.wallet_cap_scope.trim().is_empty() {
        return Err(RewarderError::Config(
            "ingress.wallet_cap_scope must not be empty".into(),
        ));
    }

    if !matches!(cfg.pq.mode.as_str(), "off" | "hybrid") {
        return Err(RewarderError::Config("pq.mode must be off|hybrid".into()));
    }
    if !matches!(
        cfg.shard.strategy.as_str(),
        "single" | "by_actor" | "by_content"
    ) {
        return Err(RewarderError::Config(
            "shard.strategy must be single|by_actor|by_content".into(),
        ));
    }
    if cfg.shard.shards == 0 {
        return Err(RewarderError::Config("shard.shards must be > 0".into()));
    }
    Ok(())
}

fn validate_http_base_url(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(RewarderError::Config(format!("{name} must not be empty")));
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(RewarderError::Config(format!(
            "{name} must start with http:// or https://"
        )));
    }
    if value.chars().any(char::is_whitespace) {
        return Err(RewarderError::Config(format!(
            "{name} must not contain whitespace"
        )));
    }
    Ok(())
}
