//! RO:WHAT — Public entry for configuration types and loader.
//! RO:WHY  — Keep builder/env/file precedence & validation together.
//! RO:INTERACTS — model (schema), loader (env/file), reload (hooks).
//! RO:INVARIANTS — builder > env > file > defaults; deny invalid combos; amnesia honored.

mod loader;
mod model;
mod reload;

pub use loader::{from_env_validated, RykerConfigBuilder};
pub use model::{FairnessCfg, RykerConfig, SupervisionCfg};
pub use reload::{ReloadCounters, RykerReloadHook};
