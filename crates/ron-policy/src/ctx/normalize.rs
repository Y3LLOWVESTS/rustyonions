//! RO:WHAT — Builders and normalizers to construct a `Context` safely.
//!
//! RO:WHY  — Avoid ad-hoc normalization in callers; ensure consistent casing and defaults.

use super::clock::Clock;
use super::Context;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct ContextBuilder {
    tenant: Option<String>,
    method: Option<String>,
    region: Option<String>,
    body_bytes: Option<u64>,
    tags: BTreeSet<String>,
}

impl ContextBuilder {
    #[must_use]
    pub fn tenant(mut self, t: impl Into<String>) -> Self {
        self.tenant = Some(t.into());
        self
    }

    #[must_use]
    pub fn method(mut self, m: impl Into<String>) -> Self {
        self.method = Some(m.into());
        self
    }

    #[must_use]
    pub fn region(mut self, r: impl Into<String>) -> Self {
        self.region = Some(r.into());
        self
    }

    #[must_use]
    pub const fn body_bytes(mut self, n: u64) -> Self {
        self.body_bytes = Some(n);
        self
    }

    #[must_use]
    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.tags.insert(t.into().to_ascii_lowercase());
        self
    }

    pub fn build<C: Clock>(self, clock: &C) -> Context {
        Context {
            tenant: self.tenant.unwrap_or_else(|| "*".to_string()),
            method: self
                .method
                .map_or_else(|| "*".to_string(), |s| s.to_ascii_uppercase()),
            region: self.region.unwrap_or_else(|| "*".to_string()),
            body_bytes: self.body_bytes.unwrap_or(0),
            tags: self.tags,
            now_ms: clock.now_ms(),
        }
    }
}
