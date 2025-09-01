#![forbid(unsafe_code)]

pub mod types;
pub mod validate;
pub mod watch;

// Re-exports to preserve the old API surface:
pub use types::{Config, TransportConfig, load_from_file};
pub use watch::spawn_config_watcher;
pub use validate::validate;
