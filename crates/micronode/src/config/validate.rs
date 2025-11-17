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
