//! RO:WHAT — Policy module fanout for svc-storage.
//! RO:WHY — Concerns: ECON/SEC/GOV; keeps admission, quota, residency, and economics seams separate.
//! RO:INTERACTS — http::routes::paid_object, policy::{paid_write,economics,quotas,residency}.
//! RO:INVARIANTS — paid writes fail closed; no name resolution; no ledger mutation inside storage.
//! RO:METRICS — policy decisions are observed by route handlers.
//! RO:CONFIG — future quota/residency knobs live under storage config.
//! RO:SECURITY — verifier seams isolate dev headers from future wallet-backed proof checks.
//! RO:TEST — tests/paid_write_policy.rs, tests/paid_write_verifier.rs, tests/web3_paid_storage_loop.rs.

pub mod economics;
pub mod paid_write;
pub mod quotas;
pub mod residency;
