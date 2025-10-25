//! RO:WHAT — Runtime implementation and mailbox factory methods.
//! RO:WHY  — Host-owned container to spawn mailboxes with per-actor overrides.
//! RO:INTERACTS — config::RykerConfig, mailbox::{Mailbox, MailboxBuilder}.
//! RO:INVARIANTS — never allocates unbounded queues; respects defaults & overrides.

use crate::config::RykerConfig;
use crate::mailbox::{Mailbox, MailboxBuilder};
use std::sync::Arc;

#[derive(Clone)]
pub struct Runtime {
    cfg: Arc<RykerConfig>,
}

impl Runtime {
    pub fn new(cfg: RykerConfig) -> Self {
        Self { cfg: Arc::new(cfg) }
    }

    pub fn mailbox<T>(&self, actor_name: impl Into<String>) -> MailboxBuilder<T> {
        MailboxBuilder::new(actor_name.into(), self.cfg.clone())
    }

    /// Build a mailbox immediately with defaults (no overrides).
    pub fn mailbox_default<T>(&self, actor_name: impl Into<String>) -> Mailbox<T> {
        self.mailbox(actor_name).build()
    }

    /// Access the effective config snapshot.
    pub fn config(&self) -> Arc<RykerConfig> {
        self.cfg.clone()
    }
}
