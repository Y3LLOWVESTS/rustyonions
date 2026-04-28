//! RO:WHAT — Integration-test module root for ron-accounting property tests.
//! RO:WHY — Pillar 12; Concerns: DX/ECON. Ensures tests/prop/*.rs are executed by Cargo.
//! RO:INTERACTS — proptest modules for encoding and labels.
//! RO:INVARIANTS — property tests stay deterministic enough for CI.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — label properties check basic privacy/cardinality behavior.
//! RO:TEST — cargo test -p ron-accounting --test prop.

#[path = "prop/encoding_prop.rs"]
mod encoding_prop;
#[path = "prop/labels_prop.rs"]
mod labels_prop;
