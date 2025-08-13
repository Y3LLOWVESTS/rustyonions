#![forbid(unsafe_code)]

pub mod error;
pub mod protocol;
pub mod store;

pub use store::Store;

// Public API
pub use protocol::{
    client_get, client_put, run_overlay_listener,
    // NEW: generic transport-based server & clients
    run_overlay_listener_with_transport, client_put_via, client_get_via,
};
