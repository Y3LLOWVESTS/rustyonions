// crates/micronode/src/facets/media.rs
//! RO:WHAT — Placeholder for media facet wiring.
//! RO:WHY  — Media upload/transform pipelines (thumbnails, transcoding) sit
//!           behind Micronode and svc-storage; this file will hold configs.
//! RO:STATUS — Stub only; no runtime behavior yet.

#[derive(Debug, Clone)]
pub struct MediaFacetConfig {
    // TODO: allowed mime types, size caps, transform presets.
    _placeholder: (),
}
