//! RO:WHAT — Public mailbox types: builder and queue facade.
//! RO:WHY  — Bounded single-consumer mailbox; Busy on overflow.
//! RO:INTERACTS — queue (tokio mpsc), observer hooks, errors.
//! RO:INVARIANTS — FIFO per-mailbox; reject-new; deadlines enforced via timeout.

mod builder;
mod error;
pub mod observer;
mod queue;

pub use builder::MailboxBuilder;
pub use error::{MailboxError, MailboxResult};
pub use queue::Mailbox;

// Convenience re-exports so users can `use ryker::mailbox::*;`
pub use observer::{DropReason, MailboxObserver, NoopObserver, Observer};
