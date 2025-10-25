//! RO:WHAT — Runtime facade holding the effective config snapshot.
//! RO:WHY  — Central factory for MailboxBuilder; no global executors.
//! RO:INTERACTS — mailbox builder/queue, observe hooks, config snapshot.
//! RO:INVARIANTS — snapshot immutable via Arc; hosts own task lifetimes.
#![allow(clippy::module_inception)]

mod runtime;

pub use runtime::Runtime;
