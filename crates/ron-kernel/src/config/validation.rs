//! RO:WHAT — Pure validation/sanitization for `Config`.
//! RO:WHY  — Keeps I/O out of validation; enables property/fuzz tests; SEC/RES concern.
//! RO:INTERACTS — config::watcher (apply), metrics/readiness (config_loaded), events.
//! RO:INVARIANTS — Deterministic; no side effects; clamps to safe ranges; deny unknown fields is enforced on the struct.
//! RO:METRICS/LOGS — N/A.
//! RO:CONFIG — Validates ports/timeouts/amnesia; extend as struct grows.
//! RO:SECURITY — Rejects malformed values early.
//! RO:TEST HOOKS — table tests; property: idempotent sanitize.

use std::time::Duration;

use crate::internal::types::BoxError;
use crate::Config; // expected in crate root per project notes

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("invalid port: {0}")]
    InvalidPort(u16),
    #[error("timeout out of range: {0:?}")]
    InvalidTimeout(Duration),
}

const PORT_MIN: u16 = 1;
const PORT_MAX: u16 = 65535;
const TIMEOUT_MIN: Duration = Duration::from_millis(10);
const TIMEOUT_MAX: Duration = Duration::from_secs(300);

pub fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if !(PORT_MIN..=PORT_MAX).contains(&cfg.http_port) {
        return Err(ConfigError::InvalidPort(cfg.http_port));
    }
    if cfg.request_timeout < TIMEOUT_MIN || cfg.request_timeout > TIMEOUT_MAX {
        return Err(ConfigError::InvalidTimeout(cfg.request_timeout));
    }
    // amnesia is boolean; always valid
    Ok(())
}

pub fn sanitize(mut cfg: Config) -> Result<Config, BoxError> {
    if cfg.http_port == 0 {
        cfg.http_port = 9600;
    }
    if cfg.request_timeout < TIMEOUT_MIN {
        cfg.request_timeout = TIMEOUT_MIN;
    } else if cfg.request_timeout > TIMEOUT_MAX {
        cfg.request_timeout = TIMEOUT_MAX;
    }
    Ok(cfg)
}
