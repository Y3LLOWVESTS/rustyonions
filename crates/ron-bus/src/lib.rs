// crates/ron-bus/src/lib.rs
#![forbid(unsafe_code)]

pub mod api;
pub mod uds;

/// Crate version of the bus protocol (bump if wire format changes).
pub const RON_BUS_PROTO_VERSION: u32 = 1;
