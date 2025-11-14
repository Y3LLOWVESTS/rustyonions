/*!
RO:WHAT — Verification helpers for individual audit records and chains.
RO:WHY — Integrity: enforce self_hash correctness and prev/self linkage invariants.
RO:INTERACTS — crate::hash, crate::errors, crate::dto::AuditRecord.
RO:INVARIANTS — no unsafe; verify_chain is scalar reference; verify_chain_soa is batch fast path with matching semantics.
RO:METRICS/LOGS — none here; callers may instrument latency/histograms externally.
RO:CONFIG — none.
RO:SECURITY — any tamper in the audit chain is surfaced as VerifyError.
RO:TEST HOOKS — unit tests (idempotency, multi_writer_ordering, verify_soa); benches/verify_chain.rs.
*/

mod chain;
mod record;

pub use chain::{verify_chain, verify_chain_soa, verify_link};
pub use record::verify_record;
