#![forbid(unsafe_code)]

// Public modules
pub mod app;
pub mod audit;
pub mod auth;
pub mod bus;
pub mod cache;
pub mod config;
pub mod constants;
pub mod dht;
pub mod error;
pub mod http;
pub mod logging;
pub mod net;
pub mod pipeline;
pub mod router;
pub mod state;
pub mod store;
pub mod telemetry;
pub mod types;
pub mod utils;

// Re-exports
pub use config::Config;
pub use router::build_router;
pub use state::AppState; // <-- from state, not app
