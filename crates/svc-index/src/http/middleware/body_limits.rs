//! RO:WHAT — Body size cap middleware placeholder (MVP).
//! RO:WHY  — Hardening v2.0 will wire strict limits per extractor/route.
//! NOTE: Using identity layer for now to avoid unused imports/warnings.

pub fn layer(_max: usize) -> tower::layer::util::Identity {
    tower::layer::util::Identity::new()
}
