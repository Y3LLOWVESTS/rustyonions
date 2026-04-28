//! RO:WHAT — Integration-test module root for future ron-accounting loom models.
//! RO:WHY — Pillar 12; Concerns: RES/PERF. Keeps model test files visible to Cargo.
//! RO:INTERACTS — tests/loom placeholders now; real loom models later.
//! RO:INVARIANTS — test-only module wiring; no production logic.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo test -p ron-accounting --test loom.

#[path = "loom/router_model.rs"]
mod router_model;
#[path = "loom/shutdown_model.rs"]
mod shutdown_model;
