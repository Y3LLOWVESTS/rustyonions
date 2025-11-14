//! Privacy policy helpers for audit records.
//!
//! This is intentionally conservative for now: `validate` is a no-op that
//! always succeeds. Future iterations can add heuristics or policy hooks.

use crate::AuditRecord;

/// Errors raised by privacy policy checks.
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    /// Placeholder error for when PII or disallowed data is detected.
    #[error("privacy policy violation detected")]
    Violation,
}

/// Validate an `AuditRecord` against privacy policies.
///
/// At the moment this performs no checks and always returns `Ok(())`.
/// Hosts can add richer inspection logic later without breaking the
/// public function signature.
pub fn validate(_rec: &AuditRecord) -> Result<(), PrivacyError> {
    Ok(())
}
