//! RO:WHAT — Config module root for Macronode.
//! RO:WHY  — Centralize schema + loaders (env/file/CLI overlays).
//! RO:INVARIANTS —
//!   - `Config` is the single source of truth for runtime settings.
//!   - Callers use `load_config()` or `load_effective_config()`; no ad-hoc
//!     env/file access sprinkled around the crate.

pub mod cli_overlay;
pub mod env_overlay;
pub mod hot_reload;
pub mod load;
pub mod schema;
pub mod validate;

// Public facade:
// - `Config` type
// - `load_config()` for non-run CLI commands (env + optional file)
// - `load_effective_config()` for `run` (CLI --config + env)
// - `hot_reload()` used by `/api/v1/reload` handler.
pub use hot_reload::hot_reload;
pub use load::{load_config, load_effective_config};
pub use schema::Config;
