//! RO:WHAT — Implementation of `config validate`.
//! RO:WHY  — Off-line validation of config without starting listeners.
//! RO:NOTE — For now this is equivalent to `config print`/`check` using
//!           env-based config; file-based validation will land later.

use crate::{config::load_config, errors::Result};

pub fn run() -> Result<()> {
    let _cfg = load_config()?;
    println!("macronode config validate: OK (env-based config)");
    Ok(())
}
