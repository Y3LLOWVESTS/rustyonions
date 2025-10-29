//! Range validation helpers (kept minimal – routes/impls already check).

use crate::errors::StorageError;

#[allow(dead_code)]
pub fn validate_range(start: u64, end_inclusive: u64, total: u64) -> Result<(), StorageError> {
    if start > end_inclusive || end_inclusive >= total {
        return Err(StorageError::RangeNotSatisfiable);
    }
    Ok(())
}
