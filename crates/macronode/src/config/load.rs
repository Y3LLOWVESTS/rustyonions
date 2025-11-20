//! RO:WHAT — Config load pipeline for Macronode.
//! RO:WHY  — Centralize config loading so CLI and runtime share precedence,
//!           env overlays, and validation.
//! RO:INVARIANTS —
//!   - Always start from `Config::default()`.
//!   - Precedence for config sources:
//!       1) Defaults
//!       2) Optional file from CLI `--config` or env (`RON_CONFIG` / `MACRO_CONFIG`)
//!       3) Env overlays (`RON_*` + `MACRO_*` aliases)
//!   - Validation always runs before returning a config to callers.
//!
//! RO:CONFIG SOURCES —
//!   - `--config PATH` (CLI) has highest precedence for the file path.
//!   - If no CLI path is supplied, `RON_CONFIG` is honored.
//!   - `MACRO_CONFIG` is accepted for one minor with a warning.

use std::{env, fs, path::Path};

use crate::errors::{Error, Result};

use super::{env_overlay::apply_env_overlays, schema::Config, validate::validate_config};

/// Load config using defaults + **optional file from env** + env overlays.
///
/// This is what non-`run` CLI commands (`check`, `config print`,
/// `config validate`) use: they do not accept `--config` themselves, but
/// operators can still provide a file via `RON_CONFIG`/`MACRO_CONFIG`.
pub fn load_config() -> Result<Config> {
    load_effective_config(None)
}

/// Load config using an explicit file path (if provided), then env overlays.
///
/// This is the low-level helper used by the higher-level
/// `load_effective_config`. Precedence inside this function is:
///
///   1) `Config::default()`
///   2) Optional file (if `file_path` is `Some(_)`)
///   3) Env overlays
///
/// Validation is always run before the config is returned.
pub fn load_config_with_file(file_path: Option<&str>) -> Result<Config> {
    let mut cfg = Config::default();

    if let Some(path) = file_path {
        let path = Path::new(path);
        cfg = load_from_file(path)?;
    }

    let cfg = apply_env_overlays(cfg)?;
    validate_config(&cfg)?;
    Ok(cfg)
}

/// Load the **effective** config, combining CLI and env-level file paths.
///
/// Precedence for the config *file path* is:
///
///   1) CLI `--config PATH` (if supplied)
///   2) `RON_CONFIG` (if set and non-empty)
///   3) `MACRO_CONFIG` (deprecated alias; emits a warning)
///
/// After the file (if any) is applied, env overlays and validation are run.
pub fn load_effective_config(cli_file_path: Option<&str>) -> Result<Config> {
    let chosen_path = match cli_file_path {
        Some(p) => Some(p.to_string()),
        None => env_config_path(),
    };

    load_config_with_file(chosen_path.as_deref())
}

/// Discover a config file path from env (`RON_CONFIG` / `MACRO_CONFIG`).
///
/// Returns `Some(path)` if a non-empty value is found, otherwise `None`.
fn env_config_path() -> Option<String> {
    if let Ok(val) = env::var("RON_CONFIG") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }

    // Temporary compatibility alias for older docs/scripts.
    if let Ok(val) = env::var("MACRO_CONFIG") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            eprintln!(
                "macronode: MACRO_CONFIG is deprecated; prefer RON_CONFIG for config file path"
            );
            return Some(trimmed.to_owned());
        }
    }

    None
}

/// Load a `Config` from a TOML or JSON file.
///
/// - If the extension is `.toml`, parse as TOML.
/// - If the extension is `.json`, parse as JSON.
/// - Otherwise, try TOML first, then JSON, and include both errors on failure.
fn load_from_file(path: &Path) -> Result<Config> {
    let raw = fs::read_to_string(path).map_err(|e| {
        Error::config(format!(
            "failed to read config file {}: {e}",
            path.display()
        ))
    })?;

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let parsed: Result<Config> = match ext.as_str() {
        "toml" => toml::from_str(&raw).map_err(|e| {
            Error::config(format!(
                "failed to parse TOML config {}: {e}",
                path.display()
            ))
        }),
        "json" => serde_json::from_str(&raw).map_err(|e| {
            Error::config(format!(
                "failed to parse JSON config {}: {e}",
                path.display()
            ))
        }),
        _ => {
            // Unknown extension: try TOML first, then JSON, and include both errors.
            let toml_err = toml::from_str::<Config>(&raw).map_err(|e| e.to_string());
            match toml_err {
                Ok(cfg) => Ok(cfg),
                Err(t_err) => {
                    let json_err = serde_json::from_str::<Config>(&raw).map_err(|e| e.to_string());
                    match json_err {
                        Ok(cfg) => Ok(cfg),
                        Err(j_err) => Err(Error::config(format!(
                            "failed to parse config {} as TOML or JSON:\n  TOML error: {t_err}\n  JSON error: {j_err}",
                            path.display()
                        ))),
                    }
                }
            }
        }
    };

    parsed
}
