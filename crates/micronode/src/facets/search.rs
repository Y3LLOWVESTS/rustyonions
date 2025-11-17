// crates/micronode/src/facets/search.rs
//! RO:WHAT — Placeholder for search facet wiring.
//! RO:WHY  — Search endpoints (full-text, filters) will surface svc-index
//!           capabilities via Micronode facets.
//! RO:STATUS — Stub only; no runtime behavior yet.

#[derive(Debug, Clone)]
pub struct SearchFacetConfig {
    // TODO: index names, query limits, ranking configs.
    _placeholder: (),
}
