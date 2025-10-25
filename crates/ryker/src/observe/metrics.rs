//! RO:WHAT — Observer trait for mailbox lifecycle signals.
//! RO:WHY  — Allow hosts to increment counters/gauges without pulling prometheus here.
//! RO:INTERACTS — mailbox queue calls hooks on enqueue/drop/timeout/drain.
//! RO:INVARIANTS — must be non-blocking; cheap; thread-safe.

use std::sync::Arc;

#[derive(Clone)]
pub struct NoopObserver;

impl MailboxObserver for NoopObserver {
    fn on_enqueue(&self, _actor: &str, _depth: usize) {}
    fn on_drop(&self, _actor: &str, _reason: DropReason) {}
    fn on_timeout(&self, _actor: &str) {}
    fn on_restart(&self, _actor: &str) {}
}

#[derive(Clone, Copy, Debug)]
pub enum DropReason {
    Capacity,
    Closed,
}

pub trait MailboxObserver: Send + Sync + 'static {
    fn on_enqueue(&self, actor: &str, depth: usize);
    fn on_drop(&self, actor: &str, reason: DropReason);
    fn on_timeout(&self, actor: &str);
    fn on_restart(&self, actor: &str);
}

pub type Observer = Arc<dyn MailboxObserver>;
