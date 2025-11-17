//! RO:WHAT — Handle for the overlay or discovery service
//!           (svc-overlay / svc-dht).
//!
//! RO:WHY  — Micronode may need to ask the overlay about other nodes,
//!           capabilities, or routes. Keeping that behind an adapter
//!           lets us mock or swap it out cleanly.
//!
//! RO:INVARIANTS —
//!   * Pure configuration container for now.
//!   * Does not assume a particular protocol; `base_url` is just a
//!     locator string that higher layers agree on.

#[derive(Clone, Debug)]
pub struct OverlayClient {
    base_url: String,
}

impl OverlayClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Short, stable tag suitable for metrics or logging labels.
    pub fn tag(&self) -> &'static str {
        "svc-overlay"
    }
}
