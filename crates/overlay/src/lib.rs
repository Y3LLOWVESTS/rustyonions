#![forbid(unsafe_code)]

pub mod error;
pub mod protocol;
pub mod store;

// Re-export common entry points for convenience.
pub use protocol::{client_get, client_put, run_overlay_listener};
pub use store::Store;
