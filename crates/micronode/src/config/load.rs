//! RO:WHAT — Load config from file and env overlays (foundation).
//! RO:WHY  — Keep main.rs tiny; centralized error handling.
//! RO:INVARIANTS — File is optional; env can fully drive defaults.

use super::{schema::Config, validate::validate};
use crate::errors::{Error, Result};
use std::{env, fs, path::PathBuf};

pub fn load_config() -> Result<Config> {
    let mut cfg = Config::default();

    // Optional file: MICRONODE_CONFIG or ./configs/micronode.toml
    if let Some(path) = env::var_os("MICRONODE_CONFIG") {
        overlay_file(&mut cfg, PathBuf::from(path))?;
    } else {
        let p = PathBuf::from("crates/micronode/configs/micronode.toml");
        if p.exists() {
            overlay_file(&mut cfg, p)?;
        }
    }

    // Env overlays
    overlay_env(&mut cfg)?;

    validate(&cfg)?;
    Ok(cfg)
}

fn overlay_file(cfg: &mut Config, path: PathBuf) -> Result<()> {
    let s =
        fs::read_to_string(&path).map_err(|e| Error::Config(format!("read {:?}: {e}", path)))?;
    let from: Config =
        toml::from_str(&s).map_err(|e| Error::Config(format!("parse {:?}: {e}", path)))?;
    *cfg = merge(cfg.clone(), from);
    Ok(())
}

fn overlay_env(cfg: &mut Config) -> Result<()> {
    if let Ok(addr) = std::env::var("BIND_ADDR") {
        cfg.server.bind =
            addr.parse().map_err(|e| Error::Config(format!("BIND_ADDR parse: {e}")))?;
    }
    if let Ok(v) = std::env::var("MICRONODE_DEV_ROUTES") {
        cfg.server.dev_routes = matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON");
    }
    Ok(())
}

fn merge(mut base: Config, from: Config) -> Config {
    base.server.bind = from.server.bind;
    base.server.dev_routes = from.server.dev_routes;
    base
}
