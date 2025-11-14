/*!
RO:WHAT — Scalar and SoA-style helpers for verifying audit chains.
RO:WHY — Integrity/RES: cheaply validate self_hash and prev/self linkage over large batches.
RO:INTERACTS — super::record::verify_record; crate::errors::VerifyError; crate::AuditRecord.
RO:INVARIANTS — no unsafe; verify_chain is the scalar reference; verify_chain_soa must be
                 semantics-identical. The `simd` feature is currently a no-op hook reserved for
                 future optimizations; scalar equality remains the oracle.
RO:METRICS/LOGS — none here; callers may instrument latency externally.
RO:CONFIG — none; batch size is provided by callers.
RO:SECURITY — tamper-evident: any mismatch is surfaced as VerifyError::HashMismatch or LinkMismatch.
RO:TEST HOOKS — tests/idempotency.rs, tests/multi_writer_ordering.rs, tests/verify_soa.rs;
                 benches/verify_chain.rs.
*/

use crate::errors::VerifyError;
use crate::verify::verify_record;
use crate::AuditRecord;

/// Internal helper for comparing prev/self_hash.
///
/// For now this is a thin wrapper around `==` in all configurations. The `simd`
/// feature flag is reserved for future, stable SIMD-based implementations once
/// the ecosystem and MSRV make that a good trade-off.
///
/// Keeping this helper as a single choke point allows us to:
/// - keep scalar equality as the correctness oracle today;
/// - later drop in a feature-gated optimized implementation without touching
///   the rest of the verification code.
#[inline]
fn eq_hashes(a: &str, b: &str) -> bool {
    // NOTE: `simd` feature currently does not change behavior. When we add a
    // stable SIMD implementation (via an external crate or std support),
    // it will live behind this helper.
    #[cfg(feature = "simd")]
    {
        a == b
    }

    #[cfg(not(feature = "simd"))]
    {
        a == b
    }
}

/// Verify that `next` correctly links to `prev`.
///
/// At minimum we check:
/// - `next.prev == prev.self_hash`
pub fn verify_link(prev: &AuditRecord, next: &AuditRecord) -> Result<(), VerifyError> {
    if eq_hashes(&next.prev, &prev.self_hash) {
        Ok(())
    } else {
        Err(VerifyError::LinkMismatch)
    }
}

/// Scalar reference implementation: verify a full chain of audit records
/// provided as an iterator.
///
/// The iterator is consumed; each record is verified individually and
/// adjacency is checked via [`verify_link`].
pub fn verify_chain<I>(iter: I) -> Result<(), VerifyError>
where
    I: IntoIterator<Item = AuditRecord>,
{
    let mut last: Option<AuditRecord> = None;

    for rec in iter {
        verify_record(&rec)?;
        if let Some(prev) = &last {
            verify_link(prev, &rec)?;
        }
        last = Some(rec);
    }

    Ok(())
}

/// SoA-style batch verifier over a contiguous slice of records.
///
/// This is intended as a "fast path" for hosts that already have a `&[AuditRecord]`
/// in memory. It is kept separate from [`verify_chain`] so the scalar reference
/// implementation can remain small and obviously correct.
///
/// Semantics:
/// - If the slice is empty, returns `Ok(())`.
/// - For each record, recomputes and checks its `self_hash`.
/// - For each adjacent pair `(prev, next)` checks `next.prev == prev.self_hash`
///   via the `eq_hashes` helper (which is currently scalar in all modes).
pub fn verify_chain_soa(chain: &[AuditRecord]) -> Result<(), VerifyError> {
    if chain.is_empty() {
        return Ok(());
    }

    // First pass: verify each record's self_hash.
    for rec in chain {
        verify_record(rec)?;
    }

    // Second pass: SoA-style linkage check.
    for i in 1..chain.len() {
        let prev = &chain[i - 1];
        let next = &chain[i];

        if !eq_hashes(&next.prev, &prev.self_hash) {
            return Err(VerifyError::LinkMismatch);
        }
    }

    Ok(())
}
