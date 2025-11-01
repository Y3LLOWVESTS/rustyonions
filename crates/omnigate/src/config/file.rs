//! RO:WHAT — Load Config from TOML file if present (simple search).

use super::Config;
use std::{fs, path::PathBuf};

const DEFAULT_PATHS: &[&str] = &[
    "crates/omnigate/configs/omnigate.toml", // repo-relative (dev)
    "configs/omnigate.toml",                 // crate-relative (installed)
    "/etc/ron/omnigate.toml",                // system
];

pub fn load_from_default_path() -> anyhow::Result<Option<Config>> {
    for p in DEFAULT_PATHS {
        if let Some(cfg) = try_load(PathBuf::from(p))? {
            return Ok(Some(cfg));
        }
    }
    Ok(None)
}

fn try_load(path: PathBuf) -> anyhow::Result<Option<Config>> {
    if !path.exists() {
        return Ok(None);
    }
    let s = fs::read_to_string(&path)?;
    let cfg: Config = toml::from_str::<Config>(&s)?;
    Ok(Some(cfg))
}
