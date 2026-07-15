//! RO:WHAT — Environment overlays for Macronode config.
//! RO:WHY  — Separate side-effectful env reading from pure config logic.
//! RO:INVARIANTS —
//!   - Never panics on bad env; all issues bubble as `Error::Config`.
//!   - Aliases `MACRO_*` are supported for one minor with a warning.
//!   - `metrics_addr` inherits `http_addr` when no explicit metrics env is set.

use std::{env, net::SocketAddr};

use humantime::parse_duration;

use crate::errors::{Error, Result};

use super::schema::Config;

/// Apply environment-based overlays to a `Config` value.
///
/// Supported env vars:
///   - `RON_HTTP_ADDR` / `MACRO_HTTP_ADDR`
///   - `RON_METRICS_ADDR` / `MACRO_METRICS_ADDR`
///   - `RON_LOG`
///   - `RON_ADMIN_UI_ENABLED` / `MACRO_ADMIN_UI_ENABLED`
///   - `RON_ADMIN_UI_BIND` / `MACRO_ADMIN_UI_BIND`
///   - `RON_HEADLESS_MODE` / `MACRO_HEADLESS_MODE`
///   - `RON_OPERATOR_UI_PROFILE` / `MACRO_OPERATOR_UI_PROFILE`
///   - `RON_ADMIN_UI_RUNTIME_REQUIRED` / `MACRO_ADMIN_UI_RUNTIME_REQUIRED`
///   - `RON_ADMIN_SETUP_TOKEN_TTL` / `MACRO_ADMIN_SETUP_TOKEN_TTL`
///   - `RON_READ_TIMEOUT` / `MACRO_READ_TIMEOUT`
///   - `RON_WRITE_TIMEOUT` / `MACRO_WRITE_TIMEOUT`
///   - `RON_IDLE_TIMEOUT` / `MACRO_IDLE_TIMEOUT`
pub fn apply_env_overlays(mut cfg: Config) -> Result<Config> {
    let mut metrics_overridden = false;

    // Metrics addr — may override HTTP if explicitly set.
    if let Some(val) = first_of(&["RON_METRICS_ADDR", "MACRO_METRICS_ADDR"]) {
        let addr: SocketAddr = val
            .parse()
            .map_err(|e| Error::config(format!("invalid metrics addr {val:?}: {e}")))?;
        cfg.metrics_addr = addr;
        metrics_overridden = true;
    }

    // HTTP addr — if set and metrics were not explicitly overridden, we keep
    // the invariant that metrics inherits HTTP by default.
    if let Some(val) = first_of(&["RON_HTTP_ADDR", "MACRO_HTTP_ADDR"]) {
        let addr: SocketAddr = val
            .parse()
            .map_err(|e| Error::config(format!("invalid HTTP addr {val:?}: {e}")))?;
        cfg.http_addr = addr;
        if !metrics_overridden {
            cfg.metrics_addr = addr;
        }
    }

    // Optional service-node admin UI posture.
    if let Some(val) = first_of(&["RON_ADMIN_UI_ENABLED", "MACRO_ADMIN_UI_ENABLED"]) {
        cfg.admin_ui_enabled = parse_bool_checked("admin_ui_enabled", &val)?;
    }

    if let Some(val) = first_of(&["RON_ADMIN_UI_BIND", "MACRO_ADMIN_UI_BIND"]) {
        let addr: SocketAddr = val
            .parse()
            .map_err(|e| Error::config(format!("invalid admin UI bind addr {val:?}: {e}")))?;
        cfg.admin_ui_bind = addr;
    }

    if let Some(val) = first_of(&["RON_HEADLESS_MODE", "MACRO_HEADLESS_MODE"]) {
        cfg.headless_mode = parse_bool_checked("headless_mode", &val)?;
    }

    if let Some(val) = first_of(&["RON_OPERATOR_UI_PROFILE", "MACRO_OPERATOR_UI_PROFILE"]) {
        if !val.trim().is_empty() {
            cfg.operator_ui_profile = val;
        }
    }

    if let Some(val) = first_of(&[
        "RON_ADMIN_UI_RUNTIME_REQUIRED",
        "MACRO_ADMIN_UI_RUNTIME_REQUIRED",
    ]) {
        cfg.admin_ui_runtime_required = parse_bool_checked("admin_ui_runtime_required", &val)?;
    }

    if let Some(val) = first_of(&["RON_ADMIN_SETUP_TOKEN_TTL", "MACRO_ADMIN_SETUP_TOKEN_TTL"]) {
        cfg.admin_setup_token_ttl = parse_duration_checked("admin_setup_token_ttl", &val)?;
    }

    // Log level
    if let Ok(val) = env::var("RON_LOG") {
        if !val.trim().is_empty() {
            cfg.log_level = val;
        }
    }

    // Timeouts
    if let Some(val) = first_of(&["RON_READ_TIMEOUT", "MACRO_READ_TIMEOUT"]) {
        cfg.read_timeout = parse_duration_checked("read_timeout", &val)?;
    }

    if let Some(val) = first_of(&["RON_WRITE_TIMEOUT", "MACRO_WRITE_TIMEOUT"]) {
        cfg.write_timeout = parse_duration_checked("write_timeout", &val)?;
    }

    if let Some(val) = first_of(&["RON_IDLE_TIMEOUT", "MACRO_IDLE_TIMEOUT"]) {
        cfg.idle_timeout = parse_duration_checked("idle_timeout", &val)?;
    }

    Ok(cfg)
}

fn first_of(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(v) = env::var(key) {
            if !v.trim().is_empty() {
                if key.starts_with("MACRO_") {
                    eprintln!(
                        "[macronode-config] WARNING: {key} is deprecated; \
                         prefer the RON_* variant instead."
                    );
                }
                return Some(v);
            }
        }
    }
    None
}

fn parse_bool_checked(field: &str, input: &str) -> Result<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(Error::config(format!(
            "invalid boolean for {field}: {input:?} — expected true/false, 1/0, yes/no, or on/off"
        ))),
    }
}

fn parse_duration_checked(field: &str, input: &str) -> Result<std::time::Duration> {
    parse_duration(input).map_err(|e| {
        Error::config(format!(
            "invalid duration for {field}: {input:?} ({e}) \
             — expected forms like \"10s\", \"500ms\", \"1m\""
        ))
    })
}
