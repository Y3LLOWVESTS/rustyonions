#![forbid(unsafe_code)]

pub mod types;
pub mod validate;
pub mod watch;

// Re-exports to preserve the old API surface:
pub use types::{load_from_file, Config, TransportConfig};
pub use validate::validate;
pub use watch::spawn_config_watcher;
