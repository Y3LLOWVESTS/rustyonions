//! RO:WHAT — Provider records with typed CID, node identity, expiry, and local status hints.
//! RO:WHY — Phase 12 needs privacy-safe candidate ordering without trusting self-declared health.
//! RO:INTERACTS — provider::Store, types::{B3Cid, CrabNodeId}, pipeline::lookup.
//! RO:INVARIANTS — status is locally observed; advertisement refresh cannot overwrite it.
//! RO:SECURITY — no raw IP/socket/transport URI or user identity is stored.
//! RO:TEST — tests/provider_selection.rs, tests/provider_roundtrip.rs.

use crate::types::{B3Cid, CrabNodeId};
use std::time::{Duration, Instant};

/// Local, non-authoritative observation about a provider.
///
/// This is not accepted from provider advertisements. It is intended to be
/// updated only from local probe, fetch, timeout, and integrity-review results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProviderStatusHint {
    /// A recent local operation observed a responsive provider.
    Responsive,

    /// No current local observation is available.
    #[default]
    Unknown,

    /// The provider remains usable but recent local behavior was impaired.
    Degraded,

    /// The provider must not currently be selected for a fetch attempt.
    Unavailable,
}

impl ProviderStatusHint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Responsive => "responsive",
            Self::Unknown => "unknown",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }

    pub(crate) fn selection_rank(self) -> Option<u8> {
        match self {
            Self::Responsive => Some(0),
            Self::Unknown => Some(1),
            Self::Degraded => Some(2),
            Self::Unavailable => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProviderRecord {
    pub cid: B3Cid,
    pub node: CrabNodeId,
    pub expires_at: Instant,
    status_hint: ProviderStatusHint,
}

impl ProviderRecord {
    pub fn new(cid: B3Cid, node: CrabNodeId, ttl: Duration) -> Self {
        Self {
            cid,
            node,
            expires_at: Instant::now() + ttl,
            status_hint: ProviderStatusHint::Unknown,
        }
    }

    pub fn expired(&self, now: Instant) -> bool {
        now >= self.expires_at
    }

    pub fn status_hint(&self) -> ProviderStatusHint {
        self.status_hint
    }

    pub(crate) fn refresh_expiry(&mut self, ttl: Duration) {
        self.expires_at = Instant::now() + ttl;
    }

    pub(crate) fn set_status_hint(&mut self, status_hint: ProviderStatusHint) {
        self.status_hint = status_hint;
    }
}
