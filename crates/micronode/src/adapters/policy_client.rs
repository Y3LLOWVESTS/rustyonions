//! RO:WHAT — Handle for the policy service (ron-policy / svc-policy)
//!           as seen from Micronode.
//!
//! RO:WHY  — Leave policy evaluation and rich authorization to a
//!           dedicated service. Micronode can forward the minimal
//!           context it knows about a request and let policy decide.
//!
//! RO:SECURITY — Capability tokens or macaroons should be treated as
//!               opaque values and passed along as such. Parsing or
//!               verifying them belongs in dedicated security code,
//!               not in this adapter.
//!
//! RO:INVARIANTS —
//!   * No networking in this module.
//!   * The adapter may be absent in some deployments; Micronode
//!     should continue to function with local-only policy.

#[derive(Clone, Debug)]
pub struct PolicyClient {
    base_url: String,
}

impl PolicyClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Short, stable tag suitable for metrics or logging labels.
    pub fn tag(&self) -> &'static str {
        "svc-policy"
    }
}
