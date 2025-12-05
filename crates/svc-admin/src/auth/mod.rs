// crates/svc-admin/src/auth/mod.rs
//
// WHAT: Auth/identity plumbing for svc-admin.
// WHY:  Central place to resolve the "current operator" identity from the
//       configured auth mode + inbound request headers.
// INTERACTS:
//   - crate::config::AuthCfg
//   - crate::dto::me::MeResponse
//   - Axum handlers (currently only /api/me)
//
// Modes (v1):
//   - "none":   synthetic dev identity (safe for local dev only)
//   - "ingress":trust ingress-provided headers (X-User, X-Groups) for identity
//   - "passport":placeholder; not yet implemented, falls back to dev identity
//
// This module is intentionally small but gives us a single place to evolve
// towards:
//   - structured Identity type
//   - AuthError taxonomy
//   - re-usable identity lookups for future gated endpoints (reload, shutdown).

use axum::http::HeaderMap;

use crate::config::AuthCfg;

pub mod ingress;
pub mod none;
pub mod passport;

/// Minimal identity representation for svc-admin.
///
/// This is intentionally narrow: just enough to drive:
///   - `/api/me`
///   - role-based gating on future mutating endpoints.
#[derive(Debug, Clone)]
pub struct Identity {
    pub subject: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

impl Identity {
    /// Synthetic dev identity used for:
    ///   - auth.mode = "none"
    ///   - any auth failure where we prefer to stay read-only but usable.
    pub fn dev_fallback() -> Self {
        Self {
            subject: "dev-operator".to_string(),
            display_name: "Dev Operator".to_string(),
            roles: vec!["admin".to_string()],
        }
    }
}

/// Coarse-grained auth errors.
///
/// For now this is only used internally; future work can map these into proper
/// HTTP responses (401/403) and metrics.
#[derive(Debug)]
pub enum AuthError {
    /// No usable identity could be derived (e.g. missing headers in a strict
    /// mode, invalid token, etc.).
    Unauthenticated(&'static str),
    /// Inputs were present but malformed (e.g. invalid UTF-8 in headers).
    Invalid(&'static str),
    /// Auth mode declared in config but not yet implemented.
    Unimplemented(&'static str),
}

/// Resolve an `Identity` from the configured auth mode + request headers.
///
/// Design constraints for v1:
///   - Always returns *some* identity on success paths.
///   - Never panics on header parsing issues; falls back to dev identity on
///     unexpected errors.
///   - Does NOT enforce 401/403 yet; `/api/me` is informational only.
///
/// Future evolution:
///   - Return `Result<Identity, AuthError>` all the way to handlers and map
///     to HTTP status.
///   - Emit `svc_admin_auth_failures_total` metrics per mode.
pub fn resolve_identity_from_headers(
    cfg: &AuthCfg,
    headers: &HeaderMap,
) -> Result<Identity, AuthError> {
    match cfg.mode.as_str() {
        "none" => Ok(none::identity()),
        "ingress" => ingress::identity_from_headers(headers),
        "passport" => passport::identity_from_headers(cfg, headers),
        // Config::load() already guards mode, but we fail soft here to keep
        // the console usable even if something drifts.
        _other => Ok(Identity::dev_fallback()),
    }
}
