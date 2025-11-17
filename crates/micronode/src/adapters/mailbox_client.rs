//! RO:WHAT — Handle for a future mailbox service used by Micronode.
//!
//! RO:WHY  — Some Micronode deployments may want a durable inbox or
//!           outbox for messages, notifications, or scheduled work.
//!           This client provides the configuration hook without
//!           forcing the mailbox concept into the core.
//!
//! RO:INVARIANTS —
//!   * No networking in this module for now.
//!   * The mailbox abstraction is intentionally vague until the
//!     concrete svc-mailbox design settles.

#[derive(Clone, Debug)]
pub struct MailboxClient {
    base_url: String,
}

impl MailboxClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Short, stable tag suitable for metrics or logging labels.
    pub fn tag(&self) -> &'static str {
        "svc-mailbox"
    }
}
