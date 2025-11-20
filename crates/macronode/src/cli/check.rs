//! RO:WHAT — Implementation of the `check` subcommand.
//! RO:WHY  — Fast validation of config/env without starting listeners.
//! RO:INVARIANTS —
//!   - Returns non-error only if config loads successfully.

use crate::{config::load_config, errors::Result};

pub fn run() -> Result<()> {
    let cfg = load_config()?;
    println!(
        "macronode check: OK (http_addr={}, metrics_addr={}, log_level={})",
        cfg.http_addr, cfg.metrics_addr, cfg.log_level
    );
    Ok(())
}
