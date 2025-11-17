//! RO:WHAT — Load config from file + env overlays into a validated `Config`.
//! RO:WHY  — Keep `main.rs` tiny; centralize config precedence and invariants.
//! RO:ORDER —
//!   1) Start from `Config::default()`.
//!   2) If `MICRONODE_CONFIG` set, load that TOML and merge.
//!   3) Else try `./configs/micronode.toml`, then `./crates/micronode/configs/micronode.toml`.
//!   4) Apply env overrides (bind/dev-routes only; storage/security via TOML).
//!   5) Validate invariants; on error return `Error::Config` (fail-fast).
//!
//! RO:INVARIANTS — Safe defaults (amnesia-first, local bind); deterministic overlay order.

use super::schema::Config;
use super::validate::validate;
use crate::errors::{Error, Result};

use std::{env, fs, net::SocketAddr, path::PathBuf};

/// Entry point used by `main.rs`.
pub fn load_config() -> Result<Config> {
    let mut cfg = Config::default();

    if let Some(path) = discover_config_path() {
        overlay_file(&mut cfg, path)?;
    }

    overlay_env(&mut cfg)?;
    validate(&cfg)?;
    Ok(cfg)
}

/// Resolve a config path by env or well-known files.
fn discover_config_path() -> Option<PathBuf> {
    if let Ok(p) = env::var("MICRONODE_CONFIG") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }

    let workspace_local = PathBuf::from("configs/micronode.toml");
    if workspace_local.exists() {
        return Some(workspace_local);
    }

    let crate_local = PathBuf::from("crates/micronode/configs/micronode.toml");
    if crate_local.exists() {
        return Some(crate_local);
    }

    None
}

/// Overlay TOML at `path` onto `cfg`.
fn overlay_file(cfg: &mut Config, path: PathBuf) -> Result<()> {
    let s =
        fs::read_to_string(&path).map_err(|e| Error::Config(format!("read {:?}: {e}", path)))?;
    let from: Config =
        toml::from_str(&s).map_err(|e| Error::Config(format!("parse {:?}: {e}", path)))?;

    let base = std::mem::take(cfg);
    *cfg = merge(base, from);
    Ok(())
}

/// Apply env-var overrides on top (bind/dev only).
///
/// Supported env:
/// - MICRONODE_BIND = "127.0.0.1:5310"
/// - MICRONODE_DEV_ROUTES = "1" | "true" | "yes"
fn overlay_env(cfg: &mut Config) -> Result<()> {
    if let Ok(bind_s) = env::var("MICRONODE_BIND") {
        let bind: SocketAddr = bind_s.parse().map_err(|e| {
            Error::Config(format!("MICRONODE_BIND parse failed for {:?}: {e}", bind_s))
        })?;
        cfg.server.bind = bind;
    }

    if let Ok(dev_s) = env::var("MICRONODE_DEV_ROUTES") {
        let dev = matches!(dev_s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        cfg.server.dev_routes = dev;
    }

    Ok(())
}

/// Merge `from` onto `base`, field-by-field.
///
/// NOTE: This is where we must carry **security** from TOML, otherwise
/// a file like `[security]\nmode="dev_allow"` won’t take effect.
fn merge(mut base: Config, from: Config) -> Config {
    base.server = from.server;
    base.storage = from.storage;
    base.security = from.security;
    base.facets = from.facets;
    base
}

#[cfg(test)]
mod tests {
    use super::super::schema::{SecurityCfg, SecurityMode};
    use super::*;

    #[test]
    fn merge_applies_security_mode() {
        let base = Config::default(); // DenyAll by default
        let from =
            Config { security: SecurityCfg { mode: SecurityMode::DevAllow }, ..Config::default() };
        let merged = merge(base, from);
        assert_eq!(merged.security.mode, SecurityMode::DevAllow);
    }

    #[test]
    fn overlay_env_parses_bind_and_dev_routes() {
        let _ = env::remove_var("MICRONODE_BIND");
        let _ = env::remove_var("MICRONODE_DEV_ROUTES");
        let mut cfg = Config::default();

        env::set_var("MICRONODE_BIND", "127.0.0.1:5311");
        env::set_var("MICRONODE_DEV_ROUTES", "true");
        overlay_env(&mut cfg).unwrap();

        assert_eq!(cfg.server.bind, "127.0.0.1:5311".parse::<SocketAddr>().unwrap());
        assert!(cfg.server.dev_routes);
    }
}
