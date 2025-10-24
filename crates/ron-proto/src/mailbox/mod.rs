//! RO:WHAT — Mailbox DTO entrypoint (Send/Recv/Ack).
//! RO:WHY  — Cross-service message shapes; at-least-once semantics live elsewhere.

pub mod send;
pub mod recv;
pub mod ack;

pub use send::Send;
pub use recv::Recv;
pub use ack::Ack;
