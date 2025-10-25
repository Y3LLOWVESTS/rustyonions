//! RO:WHAT — Per-actor mailbox builder with overrides.
//! RO:WHY  — Ergonomics; mirrors README examples; preserves snapshot defaults.
//! RO:INTERACTS — queue::Mailbox, observer hooks, config snapshot.
//! RO:INVARIANTS — capacity>0; max_msg_bytes≤1MiB; deadline bounds; reject-new policy.

#![forbid(unsafe_code)]

use super::observer::{NoopObserver, Observer};
use super::queue::Mailbox;
use crate::config::RykerConfig;
use std::sync::Arc;
use std::time::Duration;

pub struct MailboxBuilder<T> {
    actor_name: String,
    cfg: Arc<RykerConfig>,
    capacity: Option<usize>,
    max_msg_bytes: Option<usize>,
    deadline: Option<Duration>,
    observer: Option<Observer>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> MailboxBuilder<T> {
    pub(crate) fn new(actor_name: String, cfg: Arc<RykerConfig>) -> Self {
        Self {
            actor_name,
            cfg,
            capacity: None,
            max_msg_bytes: None,
            deadline: None,
            observer: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn capacity(mut self, cap: usize) -> Self {
        self.capacity = Some(cap);
        self
    }

    pub fn max_msg_bytes(mut self, max: usize) -> Self {
        self.max_msg_bytes = Some(max);
        self
    }

    pub fn deadline(mut self, d: Duration) -> Self {
        self.deadline = Some(d);
        self
    }

    /// Convenience for ms-based examples/doc parity.
    pub fn deadline_ms(mut self, ms: u64) -> Self {
        self.deadline = Some(Duration::from_millis(ms));
        self
    }

    pub fn observer(mut self, obs: Observer) -> Self {
        self.observer = Some(obs);
        self
    }

    pub fn build(self) -> Mailbox<T> {
        let cap = self.capacity.unwrap_or(self.cfg.defaults.mailbox_capacity);
        let max = self
            .max_msg_bytes
            .unwrap_or(self.cfg.defaults.max_msg_bytes);
        let dl = self.deadline.unwrap_or(self.cfg.defaults.deadline);
        let obs = self.observer.unwrap_or_else(|| Arc::new(NoopObserver));

        Mailbox::new(self.actor_name, cap, max, dl, obs)
    }
}
