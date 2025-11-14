//! Per-record verification helpers.

use crate::errors::VerifyError;
use crate::hash::b3_no_self;
use crate::AuditRecord;

/// Verify a single `AuditRecord` by recomputing its canonical hash and
/// comparing it to `self_hash`.
pub fn verify_record(rec: &AuditRecord) -> Result<(), VerifyError> {
    let expected = b3_no_self(rec)?;
    if rec.self_hash == expected {
        Ok(())
    } else {
        Err(VerifyError::HashMismatch)
    }
}
