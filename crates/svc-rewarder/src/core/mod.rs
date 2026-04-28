//! RO:WHAT — Pure reward computation module facade.
//! RO:WHY — Pillar 12; Concerns: ECON/PERF/GOV. Keeps deterministic math isolated from HTTP and IO.
//! RO:INTERACTS — inputs, outputs, http handlers.
//! RO:INVARIANTS — no floats; checked arithmetic; no ledger mutation.
//! RO:METRICS — callers time compute and classify errors.
//! RO:CONFIG — idempotency salt and policy knobs are passed in.
//! RO:SECURITY — malformed inputs fail before egress.
//! RO:TEST — unit/integration tests.

pub mod algebra;
pub mod compute;
pub mod invariants;

pub use algebra::{checked_mul_div_floor, AmountMinor};
pub use compute::{compute_manifest, run_key, ComputeInput};
pub use invariants::{validate_payouts, InvariantReport};
