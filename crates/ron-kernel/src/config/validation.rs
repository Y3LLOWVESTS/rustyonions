//! RO:WHAT — Pure validation/sanitization for `Config`.
//! RO:WHY  — Keeps I/O out of validation; enables property/fuzz tests; SEC/RES concern.
//! RO:INTERACTS — config::watcher (apply), metrics/readiness (config_loaded), events.
//! RO:INVARIANTS — Deterministic; no side effects; clamps to safe ranges; deny unknown fields on struct.
//! RO:METRICS/LOGS — N/A.
//! RO:CONFIG — Validates {version, amnesia}; extend as struct grows.
//! RO:SECURITY — Reject malformed values early.
//! RO:TEST HOOKS — table tests; property: idempotent sanitize.

use crate::Config; // expected re-export at crate root per project notes

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("version must be >= {0}")]
    InvalidVersion(u64),
}

const VERSION_MIN: u64 = 1;

/// Validate a config snapshot against canonical guardrails.
///
/// Current canon:
/// - `version` must be >= 1 (monotonic sequence used for ordering reloads)
/// - `amnesia` is a boolean (always valid)
pub fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.version < VERSION_MIN {
        return Err(ConfigError::InvalidVersion(VERSION_MIN));
    }
    Ok(())
}

/// Sanitize a config by clamping to safe ranges, preserving semantics.
/// Idempotent: calling twice yields the same result.
///
/// Current canon:
/// - Clamp `version` to at least 1
/// - `amnesia` unchanged (boolean)
pub fn sanitize(mut cfg: Config) -> Result<Config, ConfigError> {
    if cfg.version < VERSION_MIN {
        cfg.version = VERSION_MIN;
    }
    // `amnesia` requires no changes.
    validate(&cfg)?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ok() {
        let cfg = Config { version: 1, amnesia: false };
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn validate_rejects_version_zero() {
        let cfg = Config { version: 0, amnesia: true };
        assert!(validate(&cfg).is_err());
    }

    #[test]
    fn sanitize_clamps_version_and_is_idempotent() {
        let cfg = Config { version: 0, amnesia: true };
        let a = sanitize(cfg).unwrap();
        assert_eq!(a.version, 1);
        assert!(a.amnesia);

        let b = sanitize(a.clone()).unwrap();
        assert_eq!(a, b); // idempotent
    }
}
