//! RO:WHAT — Mailbox DTO entrypoint (Send/Recv/Ack).
//! RO:WHY  — Cross-service message shapes; at-least-once semantics live elsewhere.

pub mod ack;
pub mod recv;
pub mod send;

pub use ack::Ack;
pub use recv::Recv;
pub use send::Send;
