//! RO:WHAT — Config module root.
//! RO:WHY  — Schema + loaders + validation + overlays (env/CLI).
pub mod cli_overlay;
pub mod env_overlay;
pub mod hot_reload;
pub mod load;
pub mod schema;
pub mod validate;
