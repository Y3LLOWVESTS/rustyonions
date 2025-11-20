//! RO:WHAT — Config hot-reload stub.
//! RO:WHY  — `/api/v1/reload` calls this; later it will re-read config file/env.
//!
//! RO:INVARIANTS —
//!   - Non-blocking.
//!   - Does *not* mutate live config yet (will be replaced when we wire reload).

use crate::config::schema::Config;
use tracing::info;

pub fn hot_reload(_cfg: &Config) -> Result<(), String> {
    info!("macronode config hot_reload(): stub (no-op)");
    Ok(())
}
