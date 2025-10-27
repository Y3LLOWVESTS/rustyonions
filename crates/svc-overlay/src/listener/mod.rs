//! RO:WHAT — Listener module entry.
//! RO:WHY  — Keep a single listener implementation in `plain.rs` that delegates
//!           transport concerns to `crate::transport` (facade).
//! RO:CFG  — No cfgs here. The `use_ron_transport` feature is implemented in
//!           the transport facade, not at the listener boundary.

pub mod plain;

// Re-export the public API expected by bootstrap.
pub use plain::{spawn_listener, ListenerHandle};
