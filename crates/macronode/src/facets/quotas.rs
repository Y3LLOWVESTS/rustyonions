//! RO:WHAT — Quota vocabulary for Macronode admission control.
//! RO:WHY  — Provide a tiny, MACRO-local language for “how much is this
//!           caller allowed to do?” (rate limits, burst windows, etc.).
//!
//! RO:STATUS —
//!   - Foundation slice: pure types only, no counters or storage.
//!   - Evaluation engine will live alongside policy/registry later.
//!
//! RO:INVARIANTS —
//!   - All types are small and clone-friendly.
//!   - No direct dependency on any particular metrics or storage backend.

#![allow(dead_code)]

use std::time::Duration;

/// Logical window over which a quota applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaWindow {
    PerSecond,
    PerMinute,
    PerHour,
    PerDay,
    /// Custom-sized window for future extensions.
    Custom(Duration),
}

/// Stable key identifying a quota bucket.
///
/// Typical examples:
///   - subject = "tenant:abc", category = "gateway-requests"
///   - subject = "ip:203.0.113.1", category = "admin-shutdown"
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct QuotaKey {
    /// Entity being limited (user/tenant/IP/etc.).
    pub subject: String,
    /// Logical category (gateway, admin, storage, etc.).
    pub category: String,
}

impl QuotaKey {
    #[must_use]
    pub fn new<S: Into<String>, C: Into<String>>(subject: S, category: C) -> Self {
        Self {
            subject: subject.into(),
            category: category.into(),
        }
    }
}

/// Request to consume some quota from a bucket.
#[derive(Debug, Clone)]
pub struct QuotaRequest {
    /// Which bucket to charge.
    pub key: QuotaKey,
    /// How large this operation is, in arbitrary units (e.g. “1 request” or
    /// “N bytes”). Interpretation is up to the evaluator.
    pub cost: u64,
    /// Window the quota is evaluated over.
    pub window: QuotaWindow,
}

impl QuotaRequest {
    #[must_use]
    pub fn new(key: QuotaKey, cost: u64, window: QuotaWindow) -> Self {
        Self { key, cost, window }
    }
}

/// Result of a quota evaluation.
#[derive(Debug, Clone)]
pub enum QuotaDecision {
    /// Operation may proceed; remaining is a best-effort hint.
    Allow { remaining: Option<u64> },
    /// Operation is over quota; `retry_after` suggests when to try again.
    Deny { retry_after: Option<Duration> },
}

impl QuotaDecision {
    /// Returns true if this operation is allowed.
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        matches!(self, QuotaDecision::Allow { .. })
    }
}
