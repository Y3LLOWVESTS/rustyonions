//! Size bounds checks for audit records.

use crate::errors::BoundsError;
use crate::AuditRecord;

/// Default maximum serialized size (in bytes) for `attrs`.
pub const DEFAULT_MAX_ATTRS_BYTES: usize = 1024;

/// Default maximum serialized size (in bytes) for a full record.
pub const DEFAULT_MAX_RECORD_BYTES: usize = 4096;

/// Check size bounds on an `AuditRecord`.
///
/// Hosts can call this prior to hashing/append to enforce their SLOs.
pub fn check(
    rec: &AuditRecord,
    max_attrs_bytes: usize,
    max_record_bytes: usize,
) -> Result<(), BoundsError> {
    let attrs_bytes = serde_json::to_vec(&rec.attrs)
        .map(|v| v.len())
        .unwrap_or_default();

    if attrs_bytes > max_attrs_bytes {
        return Err(BoundsError::AttrsTooLarge {
            actual: attrs_bytes,
            max: max_attrs_bytes,
        });
    }

    let record_bytes = serde_json::to_vec(rec).map(|v| v.len()).unwrap_or_default();

    if record_bytes > max_record_bytes {
        return Err(BoundsError::RecordTooLarge {
            actual: record_bytes,
            max: max_record_bytes,
        });
    }

    Ok(())
}
