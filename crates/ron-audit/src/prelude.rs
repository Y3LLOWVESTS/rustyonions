/*!
RO:WHAT — Convenience prelude for common ron-audit types and helpers.
RO:WHY — DX: allow hosts/tests/benches to import a stable surface with a single use line.
RO:INTERACTS — bounds, canon, hash, verify, sink, dto.
RO:INVARIANTS — stable re-export set; no heavy dependencies pulled in accidentally.
RO:METRICS/LOGS — none.
RO:CONFIG — none.
RO:SECURITY — re-exports only; no logic.
RO:TEST HOOKS — doc test in this file; tests/api_compat.rs.
*/

pub use crate::bounds::{check as check_bounds, DEFAULT_MAX_ATTRS_BYTES, DEFAULT_MAX_RECORD_BYTES};
pub use crate::canon::{canonicalize_without_self_hash, CanonError};
pub use crate::hash::{b3_no_self, dedupe_key};
pub use crate::sink::{AuditSink, AuditStream, ChainState};
pub use crate::verify::{verify_chain, verify_chain_soa, verify_link, verify_record};
pub use crate::AuditRecord;
pub use crate::{AppendError, BoundsError, VerifyError};
