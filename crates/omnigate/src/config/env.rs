//! RO:WHAT — Apply environment variable overrides to Omnigate config.
//! RO:WHY — 12 Pillars hardening: explicit/typed config; Concerns: GOV/SEC.
//! RO:INTERACTS — `config::Config`, HTTP admission body/decompression guards, server bind config.
//! RO:INVARIANTS — OAP frame caps remain separate; HTTP body caps are bounded; malformed env fails closed.
//! RO:CONFIG — OMNIGATE_BIND, OMNIGATE_METRICS_ADDR, OMNIGATE_AMNESIA, OMNIGATE_MAX_BODY_BYTES.
//! RO:SECURITY — no secret material is read here; env only tunes local operator/runtime limits.

use super::Config;
use anyhow::{anyhow, bail, Context, Result};
use std::env;

const MIB: u64 = 1024 * 1024;
const MAX_CONFIGURABLE_HTTP_BODY_BYTES: u64 = 64 * MIB;

pub fn apply_env_overrides(cfg: &mut Config) -> Result<()> {
    if let Ok(v) = env::var("OMNIGATE_BIND") {
        cfg.server.bind = v
            .parse()
            .with_context(|| format!("invalid OMNIGATE_BIND `{v}`"))?;
    }

    if let Ok(v) = env::var("OMNIGATE_METRICS_ADDR") {
        cfg.server.metrics_addr = v
            .parse()
            .with_context(|| format!("invalid OMNIGATE_METRICS_ADDR `{v}`"))?;
    }

    if let Ok(v) = env::var("OMNIGATE_AMNESIA") {
        cfg.server.amnesia = parse_boolish(&v);
    }

    if let Some((name, value)) = first_u64_env(&[
        "OMNIGATE_MAX_BODY_BYTES",
        "OMNIGATE_MAX_CONTENT_LENGTH",
        "CRABLINK_DEV_IMAGE_BODY_BYTES",
    ])? {
        if value == 0 {
            bail!("{name} must be greater than zero");
        }

        if value > MAX_CONFIGURABLE_HTTP_BODY_BYTES {
            bail!(
                "{name}={} exceeds max HTTP body cap {}",
                value,
                MAX_CONFIGURABLE_HTTP_BODY_BYTES
            );
        }

        cfg.admission.body.max_content_length = value;
    }

    Ok(())
}

fn parse_boolish(value: &str) -> bool {
    matches!(
        value.trim(),
        "1" | "true" | "TRUE" | "True" | "on" | "ON" | "On" | "yes" | "YES" | "Yes"
    )
}

fn first_u64_env(names: &[&str]) -> Result<Option<(String, u64)>> {
    for &name in names {
        match env::var(name) {
            Ok(raw) => {
                let value = raw
                    .trim()
                    .parse::<u64>()
                    .map_err(|error| anyhow!("{name} must be a u64, got `{raw}`: {error}"))?;

                return Ok(Some((name.to_string(), value)));
            }
            Err(env::VarError::NotPresent) => {}
            Err(error) => {
                return Err(anyhow!("failed to read {name}: {error}"));
            }
        }
    }

    Ok(None)
}
