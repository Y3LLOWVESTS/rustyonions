#![forbid(unsafe_code)]

// Keep modules
pub mod error;
pub mod protocol;
pub mod store;

// Re-export Store for external users
pub use store::Store;

// Public API (async TCP protocol)
pub use protocol::{client_get, client_put, run_overlay_listener};
