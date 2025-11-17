//! RO:WHAT — Adapter layer for Micronode.
//! RO:WHY  — Provide typed, low-friction handles for talking to other
//!           RON-CORE services (index, mailbox, storage, overlay,
//!           policy) without coupling Micronode core to any specific
//!           HTTP or RPC client.
//!
//! RO:INTERACTS — In future cuts, these clients will wrap a shared
//!                HTTP or OAP/1 client and offer small, well-typed
//!                methods for a few high-value flows. For the
//!                foundation cut they are simple data holders.
//!
//! RO:INVARIANTS —
//!   * No network I/O in this module for now.
//!   * Handles are cheap to clone and can be stored in `AppState`.
//!   * Adapters are optional: Micronode can run without any of them.
//!
//! RO:CONFIG — Construction of each client is driven by higher-layer
//!             config modules or CLI overlays, not from here.
//!
//! RO:SECURITY — Capability handling and macaroon verification are
//!               handled by dedicated security services; these
//!               adapters should carry any required capability tokens
//!               as opaque strings, not parse them.
//!
//! RO:TEST — Light unit tests here ensure the basic ergonomics work;
//!           future integration tests can live in `tests/adapters_*.rs`
//!           once we wire Micronode to real remote services.

mod index_client;
mod mailbox_client;
mod overlay_client;
mod policy_client;
mod storage_client;

pub use index_client::IndexClient;
pub use mailbox_client::MailboxClient;
pub use overlay_client::OverlayClient;
pub use policy_client::PolicyClient;
pub use storage_client::StorageClient;

/// Bag of optional adapters that Micronode may use.
///
/// This is a convenience struct for higher layers that want to pass
/// around a single handle rather than five independent options.
/// Nothing in the core currently depends on this type; it is provided
/// as ready-made glue for future integration with `AppState`.
#[derive(Clone, Debug, Default)]
pub struct Adapters {
    pub index: Option<IndexClient>,
    pub mailbox: Option<MailboxClient>,
    pub storage: Option<StorageClient>,
    pub overlay: Option<OverlayClient>,
    pub policy: Option<PolicyClient>,
}

impl Adapters {
    /// Construct a new, empty adapter bag.
    ///
    /// This is equivalent to `Adapters::default()` but reads more
    /// clearly at call sites.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return `true` if no adapters are configured.
    ///
    /// This is useful for callers that want to short-circuit any
    /// remote flows if Micronode is running in a completely isolated
    /// profile.
    pub fn is_empty(&self) -> bool {
        self.index.is_none()
            && self.mailbox.is_none()
            && self.storage.is_none()
            && self.overlay.is_none()
            && self.policy.is_none()
    }

    /// Convenience builder for setting the index client.
    pub fn with_index(mut self, client: IndexClient) -> Self {
        self.index = Some(client);
        self
    }

    /// Convenience builder for setting the mailbox client.
    pub fn with_mailbox(mut self, client: MailboxClient) -> Self {
        self.mailbox = Some(client);
        self
    }

    /// Convenience builder for setting the storage client.
    pub fn with_storage(mut self, client: StorageClient) -> Self {
        self.storage = Some(client);
        self
    }

    /// Convenience builder for setting the overlay client.
    pub fn with_overlay(mut self, client: OverlayClient) -> Self {
        self.overlay = Some(client);
        self
    }

    /// Convenience builder for setting the policy client.
    pub fn with_policy(mut self, client: PolicyClient) -> Self {
        self.policy = Some(client);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_adapters_reports_empty() {
        let adapters = Adapters::new();
        assert!(adapters.is_empty());
        assert!(adapters.index.is_none());
        assert!(adapters.mailbox.is_none());
        assert!(adapters.storage.is_none());
        assert!(adapters.overlay.is_none());
        assert!(adapters.policy.is_none());
    }

    #[test]
    fn builder_methods_mark_adapters_present() {
        let idx = IndexClient::new("http://idx");
        let mbx = MailboxClient::new("http://mbx");
        let st = StorageClient::new("http://st");
        let ov = OverlayClient::new("http://ov");
        let pol = PolicyClient::new("http://pol");

        let adapters = Adapters::new()
            .with_index(idx.clone())
            .with_mailbox(mbx.clone())
            .with_storage(st.clone())
            .with_overlay(ov.clone())
            .with_policy(pol.clone());

        assert!(!adapters.is_empty());
        assert_eq!(adapters.index.as_ref().unwrap().base_url(), "http://idx");
        assert_eq!(adapters.mailbox.as_ref().unwrap().base_url(), "http://mbx");
        assert_eq!(adapters.storage.as_ref().unwrap().base_url(), "http://st");
        assert_eq!(adapters.overlay.as_ref().unwrap().base_url(), "http://ov");
        assert_eq!(adapters.policy.as_ref().unwrap().base_url(), "http://pol");
    }
}
