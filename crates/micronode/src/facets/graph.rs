// crates/micronode/src/facets/graph.rs
//! RO:WHAT — Placeholder for graph/index facet wiring.
//! RO:WHY  — Graph-style queries (follows, relationships, recommendations)
//!           will be hosted as facets backed by `svc-index`.
//! RO:STATUS — Stub only; no runtime behavior yet.

#[derive(Debug, Clone)]
pub struct GraphFacetConfig {
    // TODO: index families, graph projections, query presets.
    _placeholder: (),
}
