//! RO:WHAT — Integration-test module root for ron-accounting unit tests.
//! RO:WHY — Pillar 12; Concerns: DX/RES. Ensures tests/unit/*.rs are executed by Cargo.
//! RO:INTERACTS — tests/unit recording, events, rollover, exporter, reward snapshot, projection, interop, WAL modules.
//! RO:INVARIANTS — test-only module wiring; no production logic.
//! RO:METRICS — none.
//! RO:CONFIG — feature-gated WAL tests.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo test -p ron-accounting --test unit.

#[path = "unit/event_ingest_tests.rs"]
mod event_ingest_tests;
#[path = "unit/exporter_ordering_tests.rs"]
mod exporter_ordering_tests;
#[path = "unit/interop_vector_tests.rs"]
mod interop_vector_tests;
#[path = "unit/recording_tests.rs"]
mod recording_tests;
#[path = "unit/reward_projection_tests.rs"]
mod reward_projection_tests;
#[path = "unit/reward_snapshot_tests.rs"]
mod reward_snapshot_tests;
#[path = "unit/rollover_tests.rs"]
mod rollover_tests;
#[path = "unit/wal_tests.rs"]
mod wal_tests;
