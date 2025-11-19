//! RO:WHAT — CLI overlays for Macronode config.
//! RO:WHY  — Let `macronode run` flags override defaults/env in a single place.
//! RO:INVARIANTS —
//!   - Only overrides fields that are explicitly set on `CliOverlay`.
//!   - Never panics on bad input; errors bubble as `Error::Config`.

use std::net::SocketAddr;

use crate::errors::{Error, Result};

use super::schema::Config;

/// Minimal set of config fields that can be overridden via CLI.
///
/// This deliberately mirrors the subset of `RunOpts` we support today.
/// We keep it here (in the config module) to avoid a circular dependency
/// on `crate::cli`.
#[derive(Debug, Default, Clone)]
pub struct CliOverlay {
    pub http_addr: Option<String>,
    pub log_level: Option<String>,
}

pub fn apply_cli_overlays(mut cfg: Config, overlay: &CliOverlay) -> Result<Config> {
    // HTTP addr override
    if let Some(addr_str) = overlay.http_addr.as_deref() {
        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| Error::config(format!("invalid --http-addr {addr_str:?}: {e}")))?;
        cfg.http_addr = addr;
    }

    // Log level override
    if let Some(level) = overlay.log_level.as_ref() {
        if !level.trim().is_empty() {
            cfg.log_level = level.clone();
        }
    }

    Ok(cfg)
}
