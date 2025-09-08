#![forbid(unsafe_code)]

pub mod oap;
pub use oap::OapServer;

// Re-export modules the tests import from the crate root.
pub mod index_client;
pub mod overlay_client;
pub mod pay_enforce;
pub mod routes;
pub mod state;
pub mod utils;
pub mod quotas;
pub mod resolve;
pub mod metrics;

// Convenience re-exports used by tests
pub use routes::router;
pub use state::AppState;
