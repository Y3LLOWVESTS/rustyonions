/*! MailboxObserver hooks (stub). Forward to metrics facade without exporter lock-in. */

//! RO:WHAT — Mailbox-local observer trait re-exports and helpers.
//! RO:WHY  — Give callers a single `mailbox::observer` import surface.
//! RO:INTERACTS — Wraps `observe::metrics` to avoid leaking crate internals.
//! RO:INVARIANTS — Non-blocking hooks only; never hold locks across `.await`.

#![forbid(unsafe_code)]

pub use crate::observe::metrics::{DropReason, MailboxObserver, NoopObserver, Observer};
