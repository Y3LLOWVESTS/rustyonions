//! RO:WHAT — Config loading helpers for TOML files and RON_ACC_* environment overrides.
//! RO:WHY — Pillar 12; Concerns: RES/DX/GOV. Hosts need deterministic config precedence.
//! RO:INTERACTS — config::schema, config::validate, future adapter startup.
//! RO:INVARIANTS — file → env overrides → normalize → validate; failures are typed.
//! RO:METRICS — future adapters count reload failures around this module.
//! RO:CONFIG — RON_ACC_WINDOW_LEN_S, RON_ACC_SHARDS, RON_ACC_AMNESIA, RON_ACC_WAL_ENABLED.
//! RO:SECURITY — amnesia normalization disables persistence before validation.
//! RO:TEST — config unit tests in later batch.

use std::{env, fs, path::Path};

use crate::{
    config::{normalize_config, schema::Config, validate},
    errors::{Error, Result},
};

/// Parse, normalize, and validate a TOML config string.
pub fn from_toml_str(input: &str) -> Result<Config> {
    let cfg: Config = toml::from_str(input).map_err(|err| Error::schema(err.to_string()))?;
    finish(cfg)
}

/// Load, normalize, and validate a TOML config file.
pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Config> {
    let input = fs::read_to_string(path)?;
    from_toml_str(&input)
}

/// Load a config from an optional file and environment overrides.
pub fn from_env_and_file(path: Option<&Path>) -> Result<Config> {
    let cfg = if let Some(path) = path {
        let input = fs::read_to_string(path)?;
        toml::from_str(&input).map_err(|err| Error::schema(err.to_string()))?
    } else {
        Config::default()
    };
    finish(apply_env(cfg)?)
}

fn finish(cfg: Config) -> Result<Config> {
    let cfg = normalize_config(cfg);
    validate(&cfg)?;
    Ok(cfg)
}

fn apply_env(mut cfg: Config) -> Result<Config> {
    if let Some(value) = env_u32("RON_ACC_WINDOW_LEN_S")? {
        cfg.accounting.window_len_s = value;
    }
    if let Some(value) = env_u32("RON_ACC_SHARDS")? {
        cfg.accounting.shards = value;
    }
    if let Some(value) = env_u64("RON_ACC_CAP_ROWS")? {
        cfg.accounting.capacity_rows = value;
    }
    if let Some(value) = env_u32("RON_ACC_PEND")? {
        cfg.accounting.pending_slices_cap = value;
    }
    if let Some(value) = env_bool("RON_ACC_AMNESIA")? {
        cfg.accounting.amnesia = value;
    }
    if let Some(value) = env_bool("RON_ACC_WAL_ENABLED")? {
        cfg.wal.enabled = value;
    }
    if let Ok(value) = env::var("RON_ACC_WAL_DIR") {
        cfg.wal.dir = value.into();
    }
    Ok(cfg)
}

fn env_u32(name: &str) -> Result<Option<u32>> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u32>()
            .map(Some)
            .map_err(|err| Error::schema(format!("{name}: {err}"))),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(Error::schema(format!("{name}: {err}"))),
    }
}

fn env_u64(name: &str) -> Result<Option<u64>> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map(Some)
            .map_err(|err| Error::schema(format!("{name}: {err}"))),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(Error::schema(format!("{name}: {err}"))),
    }
}

fn env_bool(name: &str) -> Result<Option<bool>> {
    match env::var(name) {
        Ok(value) => parse_bool(name, &value).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(Error::schema(format!("{name}: {err}"))),
    }
}

fn parse_bool(name: &str, value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(Error::schema(format!("{name}: expected boolean"))),
    }
}
