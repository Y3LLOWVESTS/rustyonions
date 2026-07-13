//! RO:WHAT — Validation for Micronode configuration.
//! RO:WHY  — Catch invalid configs early (on startup) with clear
//!           error messages instead of failing deep in runtime code.
//! RO:INTERACTS — Called from `config::load::load_config` once TOML
//!                and env overlays have been applied.
//! RO:INVARIANTS —
//!   - Bind address must be usable (non-zero port).
//!   - `StorageEngine::Sled` requires a non-empty path.
//!   - Validation never mutates the config.

use crate::errors::{Error, Result};

use super::schema::{Config, StorageEngine};

/// Validate a fully assembled configuration.
///
/// Returns `Ok(())` if the config is usable; otherwise returns
/// `Error::Config` with a human-readable description.
pub fn validate(cfg: &Config) -> Result<()> {
    // Basic sanity on server.bind.
    if cfg.server.bind.port() == 0 {
        return Err(Error::Config("server.bind must not use port 0 (ephemeral)".to_string()));
    }

    // User-node privacy posture: Phase 6 user nodes are private/outbound by default.
    // Do not allow accidental public listeners while passive_runtime_enabled is true.
    if cfg.user_node.passive_runtime_enabled && !cfg.server.bind.ip().is_loopback() {
        return Err(Error::Config(
            "user_node passive runtime requires server.bind to be loopback-only".to_string(),
        ));
    }

    if cfg.user_node.max_cpu_percent == 0 || cfg.user_node.max_cpu_percent > 100 {
        return Err(Error::Config("user_node.max_cpu_percent must be in 1..=100".to_string()));
    }

    if cfg.user_node.max_background_kbps == 0 {
        return Err(Error::Config("user_node.max_background_kbps must be non-zero".to_string()));
    }

    if cfg.user_node.pending_evidence_limit == 0 {
        return Err(Error::Config("user_node.pending_evidence_limit must be non-zero".to_string()));
    }

    // Storage posture checks.
    match cfg.storage.engine {
        StorageEngine::Mem => {
            // In-memory is always valid; path is ignored.
        }
        StorageEngine::Sled => {
            // Sled requires a non-empty path so we don't silently spray
            // data into the working directory.
            let path_ok =
                cfg.storage.path.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);

            if !path_ok {
                return Err(Error::Config(
                    "storage.engine=\"sled\" requires storage.path to be set and non-empty"
                        .to_string(),
                ));
            }
        }
    }

    Ok(())
}
