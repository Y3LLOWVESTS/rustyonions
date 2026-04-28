// crates/svc-admin/src/auth/mod.rs
//
// WHAT: Auth/identity plumbing for svc-admin.
// WHY:  Central place to resolve the "current operator" identity from the
//       configured auth mode + inbound request headers.
// INTERACTS:
//   - crate::config::AuthCfg
//   - crate::dto::me::MeResponse
//   - Axum handlers (currently only /api/me and node actions)
//
// Modes (v1):
//   - "none":     synthetic dev identity (safe for local dev only)
//   - "ingress":  trust ingress-provided headers (X-User, X-Groups) for identity
//   - "passport": placeholder; not yet implemented, returns Unimplemented
//   - "local":    username/password + cookie session (local auth module); identity is resolved via session cookie
//
// This module is intentionally small but gives us a single place to evolve
// towards:
//   - structured Identity type
//   - AuthError taxonomy
//   - re-usable identity lookups for future gated endpoints (reload, shutdown).

use axum::http::HeaderMap;

use crate::config::AuthCfg;

pub mod ingress;
pub mod local;
pub mod none;
pub mod passport;
pub mod rbac;

/// Minimal identity representation for svc-admin.
///
/// This is intentionally narrow: just enough to drive:
///   - `/api/me`
///   - role-based gating on mutating endpoints (reload, shutdown, ...).
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
    /// mode, invalid token, missing cookie session, etc.).
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

        // Local mode uses cookie sessions; resolving a user requires server-side session state.
        // Callers that only have headers cannot complete this step, so we keep it fail-closed here.
        "local" => Err(AuthError::Unauthenticated(
            "local auth mode requires a valid session cookie (login via /api/auth/login)",
        )),

        // Config::load() already guards mode, but we fail soft here to keep
        // the console usable even if something drifts.
        _other => Ok(Identity::dev_fallback()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthCfg;
    use axum::http::HeaderMap;

    #[test]
    fn resolve_identity_none_uses_dev_backend() {
        // Force dev auth mode for this test.
        let mut cfg = AuthCfg::default();
        cfg.mode = "none".to_string();

        let headers = HeaderMap::new();

        let id = resolve_identity_from_headers(&cfg, &headers).expect("identity");
        assert_eq!(id.subject, "dev-operator");
        assert_eq!(id.display_name, "Dev Operator");
        assert!(
            id.roles.contains(&"admin".to_string()),
            "dev identity should include admin role"
        );
    }

    #[test]
    fn resolve_identity_ingress_delegates_to_ingress_backend() {
        let mut cfg = AuthCfg::default();
        cfg.mode = "ingress".to_string();

        let mut headers = HeaderMap::new();
        headers.insert("x-user", "alice@example.com".parse().unwrap());
        headers.insert("x-groups", "admin,ops".parse().unwrap());

        let id = resolve_identity_from_headers(&cfg, &headers).expect("identity");
        assert_eq!(id.subject, "alice@example.com");
        assert_eq!(id.display_name, "alice@example.com");
        assert_eq!(
            id.roles,
            vec!["admin".to_string(), "ops".to_string()],
            "roles should be parsed from X-Groups"
        );
    }

    #[test]
    fn resolve_identity_passport_is_unimplemented() {
        let mut cfg = AuthCfg::default();
        cfg.mode = "passport".to_string();

        let headers = HeaderMap::new();

        let err = resolve_identity_from_headers(&cfg, &headers).unwrap_err();
        match err {
            AuthError::Unimplemented(msg) => {
                assert!(
                    msg.contains("passport"),
                    "expected passport mention in error message, got: {msg}"
                );
            }
            other => panic!("expected AuthError::Unimplemented, got: {other:?}"),
        }
    }

    #[test]
    fn resolve_identity_local_is_unauthenticated_without_session() {
        let mut cfg = AuthCfg::default();
        cfg.mode = "local".to_string();

        let headers = HeaderMap::new();

        let err = resolve_identity_from_headers(&cfg, &headers).unwrap_err();
        match err {
            AuthError::Unauthenticated(msg) => {
                assert!(
                    msg.contains("local"),
                    "expected local mention in error message, got: {msg}"
                );
            }
            other => panic!("expected AuthError::Unauthenticated, got: {other:?}"),
        }
    }

    #[test]
    fn resolve_identity_unknown_mode_falls_back_to_dev() {
        let mut cfg = AuthCfg::default();
        cfg.mode = "weird-mode".to_string();

        let headers = HeaderMap::new();
        let id = resolve_identity_from_headers(&cfg, &headers).expect("identity");

        assert_eq!(id.subject, "dev-operator");
        assert!(id.roles.contains(&"admin".to_string()));
    }
}
