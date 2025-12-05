// crates/svc-admin/src/auth/none.rs
//
// WHAT: "none" auth backend.
// WHY:  Explicit dev-only mode that always returns a synthetic identity.

use super::Identity;

/// Return a synthetic dev identity for local development.
///
/// This is used whenever `auth.mode = "none"` and also as a generic
/// fallback identity in other modes when the console should stay usable
/// but clearly non-production.
pub fn identity() -> Identity {
    Identity::dev_fallback()
}
