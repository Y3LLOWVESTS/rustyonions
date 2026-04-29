//! RO:WHAT — Policy module fanout for svc-storage.
//! RO:WHY — Concerns: ECON/SEC/GOV; keeps admission, settlement, quota, residency, and economics seams separate.
//! RO:INTERACTS — http::routes::paid_object, policy::{paid_write,settlement,economics,quotas,residency}.
//! RO:INVARIANTS — paid writes fail closed; no name resolution; no direct ledger mutation inside storage.
//! RO:METRICS — policy decisions are observed by route handlers.
//! RO:CONFIG — quota/residency/economics/settlement knobs live under storage config/env.
//! RO:SECURITY — verifier and settlement seams isolate dev headers from wallet-backed production checks.
//! RO:TEST — tests/paid_write_policy.rs, tests/paid_write_verifier.rs, tests/paid_write_settlement.rs.

pub mod economics;
pub mod paid_write;
pub mod quotas;
pub mod residency;
pub mod settlement;
