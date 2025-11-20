//! RO:WHAT — Implementation of `config validate`.
//! RO:WHY  — Off-line validation of config without starting listeners.
//! RO:NOTE — Uses the same loader as `config print`/`check`, which merges
//!           defaults + optional file (via `RON_CONFIG`) + env overlays.

use crate::{config::load_config, errors::Result};

pub fn run() -> Result<()> {
    let _cfg = load_config()?;
    println!("macronode config validate: OK (file/env-based config)");
    Ok(())
}
