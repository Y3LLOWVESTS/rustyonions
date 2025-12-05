// crates/svc-admin/src/auth/passport.rs
//
// WHAT: Placeholder for "passport" auth backend.
// WHY:  Reserve the shape for future JWT / passport-style auth integration.
//
// For now, this always returns `AuthError::Unimplemented`. Callers are
// expected to decide how to degrade (e.g., fallback to dev identity in
// dev-preview environments only).

use axum::http::HeaderMap;

use crate::config::AuthCfg;

use super::{AuthError, Identity};

/// Attempt to resolve identity from a future "passport" token.
///
/// Current behavior:
///   - Always returns `AuthError::Unimplemented`.
pub fn identity_from_headers(
    _cfg: &AuthCfg,
    _headers: &HeaderMap,
) -> Result<Identity, AuthError> {
    Err(AuthError::Unimplemented(
        "auth.mode=\"passport\" not yet implemented",
    ))
}
