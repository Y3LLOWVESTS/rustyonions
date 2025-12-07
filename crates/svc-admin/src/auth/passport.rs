// crates/svc-admin/src/auth/passport.rs
//
// WHAT: Placeholder for "passport" auth backend.
// WHY:  Reserve the shape for future JWT / passport-style auth integration.
//
// FUTURE DIRECTION:
//   - Accept Authorization: Bearer <jwt>.
//   - Validate JWT using JWKS from AuthCfg (issuer/audience/etc.).
//   - Map claims -> Identity (subject, display_name, roles).
//
// CURRENT BEHAVIOR (v0.1):
//   - Always returns `AuthError::Unimplemented`.
//   - Callers (router) decide how to degrade:
//
//       * `/api/me`    → metrics + log + dev_fallback()
//       * actions      → treated as 401 Unauthorized + rejection metrics.

use axum::http::HeaderMap;

use crate::config::AuthCfg;

use super::{AuthError, Identity};

/// Attempt to resolve identity from a future "passport" token.
///
/// Right now this is intentionally unimplemented; we just surface a clear
/// error so callers can meter/log it and choose their fallback behavior.
pub fn identity_from_headers(
    _cfg: &AuthCfg,
    _headers: &HeaderMap,
) -> Result<Identity, AuthError> {
    Err(AuthError::Unimplemented(
        "auth.mode=\"passport\" is not yet implemented",
    ))
}
