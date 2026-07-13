//! RO:WHAT — Implementation of the `check` subcommand.
//! RO:WHY  — Fast validation of config/env without starting listeners.
//! RO:INVARIANTS —
//!   - Returns non-error only if config loads successfully.

use crate::{config::load_config, errors::Result};

pub fn run() -> Result<()> {
    let cfg = load_config()?;
    println!(
        "macronode check: OK (http_addr={}, metrics_addr={}, log_level={}, headless_mode={}, admin_ui_enabled={}, admin_ui_bind={}, admin_ui_runtime_required={}, operator_ui_profile={})",
        cfg.http_addr,
        cfg.metrics_addr,
        cfg.log_level,
        cfg.headless_mode,
        cfg.admin_ui_enabled,
        cfg.admin_ui_bind,
        cfg.admin_ui_runtime_required,
        cfg.operator_ui_profile
    );
    Ok(())
}
