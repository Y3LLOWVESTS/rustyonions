//! RO:WHAT — Config loader for Macronode.
//! RO:WHY  — Provide a single place to merge defaults + overlays + validation.
//! RO:INVARIANTS —
//!   - No panics; all issues surface as `Error::Config`.
//!   - Order: defaults -> env overlays -> validation.

use crate::errors::Result;

use super::{env_overlay, schema::Config, validate};

/// Load configuration for the current process.
///
/// Pipeline:
///   1. Start from `Config::default()`.
///   2. Apply env overlays (`RON_*` / `MACRO_*` aliases).
///   3. Validate invariants.
pub fn load_config() -> Result<Config> {
    let base = Config::default();
    let cfg = env_overlay::apply_env_overlays(base)?;
    validate::validate_config(&cfg)?;
    Ok(cfg)
}
