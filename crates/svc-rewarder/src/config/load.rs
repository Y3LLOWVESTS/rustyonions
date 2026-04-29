//! RO:WHAT — Config loading and overlay logic for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: DX/RES/GOV. Deterministic precedence keeps ops predictable.
//! RO:INTERACTS — config::types, config::validate, main bootstrap.
//! RO:INVARIANTS — defaults < file < env < CLI; invalid effective config fails startup.
//! RO:METRICS — none directly.
//! RO:CONFIG — reads --config and SVC_REWARDER_* env vars.
//! RO:SECURITY — secret file contents are never logged or parsed here.
//! RO:TEST — tests/unit/config.rs and live WEB3 rewarder→wallet smoke.

use std::env;
use std::path::Path;

use crate::config::types::Config;
use crate::config::validate::validate_config;
use crate::Result;

/// Load config from defaults, optional `--config`, and environment overlays.
pub fn load_config_from_env() -> Result<Config> {
    let args = env::args().collect::<Vec<_>>();
    let mut cfg = if let Some(path) = config_path_from_args(&args) {
        load_config_file(path)?
    } else if let Ok(path) = env::var("SVC_REWARDER_CONFIG") {
        load_config_file(path)?
    } else {
        Config::default()
    };

    apply_env_overlay(&mut cfg)?;
    validate_config(&cfg)?;
    Ok(cfg)
}

/// Load a TOML config file over defaults.
pub fn load_config_file(path: impl AsRef<Path>) -> Result<Config> {
    let raw = std::fs::read_to_string(path)?;
    let cfg = toml::from_str::<Config>(&raw)?;
    validate_config(&cfg)?;
    Ok(cfg)
}

fn config_path_from_args(args: &[String]) -> Option<String> {
    args.windows(2)
        .find_map(|pair| (pair[0] == "--config").then(|| pair[1].clone()))
}

fn apply_env_overlay(cfg: &mut Config) -> Result<()> {
    if let Ok(v) = env::var("SVC_REWARDER_BIND_ADDR") {
        cfg.bind_addr = v
            .parse()
            .map_err(|_| crate::RewarderError::Config("invalid SVC_REWARDER_BIND_ADDR".into()))?;
    }

    if let Ok(v) = env::var("SVC_REWARDER_METRICS_ADDR") {
        cfg.metrics_addr = v.parse().map_err(|_| {
            crate::RewarderError::Config("invalid SVC_REWARDER_METRICS_ADDR".into())
        })?;
    }

    if let Ok(v) = env::var("SVC_REWARDER_POLICY_ID") {
        cfg.rewarder.policy_id = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_WALLET_BASE_URL") {
        cfg.ingress.wallet_base_url = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_WALLET_ISSUE_PATH") {
        cfg.ingress.wallet_issue_path = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_WALLET_CAP_SCOPE") {
        cfg.ingress.wallet_cap_scope = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_ACCOUNTING_BASE_URL") {
        cfg.ingress.accounting_base_url = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_LEDGER_BASE_URL") {
        cfg.ingress.ledger_base_url = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_POLICY_BASE_URL") {
        cfg.ingress.policy_base_url = v;
    }

    if let Ok(v) = env::var("SVC_REWARDER_AMNESIA") {
        cfg.amnesia.enabled = matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on");
    }

    Ok(())
}
