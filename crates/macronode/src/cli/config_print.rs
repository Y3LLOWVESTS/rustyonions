//! RO:WHAT — Implementation of `config print`.
//! RO:WHY  — Give operators a way to see the **effective** config after
//!           defaults, optional file (RON_CONFIG/MACRO_CONFIG), and env
//!           overlays have been applied.

use crate::{config::load_config, errors::Result};

pub fn run() -> Result<()> {
    let cfg = load_config()?;
    let json = serde_json::to_string_pretty(&cfg)?;
    println!("{json}");
    Ok(())
}
