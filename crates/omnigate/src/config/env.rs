//! RO:WHAT — Apply env var overrides to Config.
//! RO:WHY  — 12 Pillars hardening: explicit/typed config; Concerns: GOV.
//! RO:INVARIANTS — Only documented keys; parse-safe; no panics.

use super::Config;
use std::env;

pub fn apply_env_overrides(cfg: &mut Config) -> anyhow::Result<()> {
    if let Ok(v) = env::var("OMNIGATE_BIND") {
        cfg.server.bind = v.parse()?;
    }
    if let Ok(v) = env::var("OMNIGATE_METRICS_ADDR") {
        cfg.server.metrics_addr = v.parse()?;
    }
    if let Ok(v) = env::var("OMNIGATE_AMNESIA") {
        cfg.server.amnesia = matches!(v.as_str(), "1" | "true" | "on" | "yes" | "TRUE");
    }
    Ok(())
}
