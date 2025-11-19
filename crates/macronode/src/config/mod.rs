//! RO:WHAT — Config module root for Macronode.
//! RO:WHY  — Centralize schema + loaders (env/file/CLI overlays).
//! RO:INVARIANTS —
//!   - `Config` is the single source of truth for runtime settings.
//!   - Callers use `load_config()`; no ad-hoc env access sprinkled around.

pub mod cli_overlay;
pub mod env_overlay;
pub mod load;
pub mod schema;
pub mod validate;

pub use load::load_config;
pub use schema::Config;
