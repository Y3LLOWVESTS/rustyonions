// crates/micronode/src/facets/feed.rs
//! RO:WHAT — Placeholder for feed/fanout facet wiring.
//! RO:WHY  — Feed-style workloads (timelines, notifications) are a primary
//!           target for Micronode facets; this file will bridge to svc-mailbox
//!           and svc-index in future iterations.
//! RO:INTERACTS — Planned: `svc-index`, `svc-mailbox`, `svc-storage` adapters.
//! RO:STATUS — Stub only; no runtime behavior yet.

#[derive(Debug, Clone)]
pub struct FeedFacetConfig {
    // TODO: topic/bucket config, retention, fanout policies.
    _placeholder: (),
}
