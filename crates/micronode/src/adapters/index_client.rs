//! RO:WHAT — Lightweight handle for talking to the index service
//!           (svc-index) from Micronode.
//!
//! RO:WHY  — Keep any future index-related flows behind a tiny,
//!           testable wrapper so Micronode core is not sprinkled with
//!           raw URLs.
//!
//! RO:INVARIANTS —
//!   * No network I/O here; this is just a configuration container.
//!   * `base_url` is assumed to point at the svc-index HTTP or OAP
//!     ingress surface depending on how the node is wired.
//!
//! RO:TEST — Trivial type; basic behavior is indirectly tested via
//!           the `adapters` module tests.

#[derive(Clone, Debug)]
pub struct IndexClient {
    base_url: String,
}

impl IndexClient {
    /// Construct a new index client from a base URL.
    ///
    /// The `base_url` should be something like
    /// `http://127.0.0.1:9913` or an internal overlay address once
    /// svc-index is running on the same node.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    /// Return the base URL this client is configured for.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Short, stable tag suitable for metrics or logging labels.
    pub fn tag(&self) -> &'static str {
        "svc-index"
    }
}
