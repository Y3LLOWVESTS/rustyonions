#![forbid(unsafe_code)]

// Re-export the OAP server so integration tests and other crates can use it.
// Make sure `crates/gateway/src/oap.rs` exists (we added it earlier).
pub mod oap;

pub use oap::OapServer;
