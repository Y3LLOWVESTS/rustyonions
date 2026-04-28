// crates/svc-admin/src/config/auth.rs
//
// WHAT: Auth configuration for svc-admin.
// WHY:  Centralized, explicit knobs for securing the admin API + SPA.
// MODES:
//   - none     : dev-only synthetic identity (NOT SAFE for non-loopback binds)
//   - ingress  : trust identity headers set by a *trusted* reverse proxy (X-User, X-Groups)
//   - passport : reserved for token/JWT validation (future; keep fail-closed until wired)
//   - local    : username/password login + httpOnly cookie session + RBAC store
//
// NOTE: We keep fields serde-friendly (Strings/ints/bools) so config can be TOML + env-driven
// without extra duration/path serde helpers. Conversions happen in the auth layer.

use serde::{Deserialize, Serialize};

/// Auth config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCfg {
    /// Auth mode selector.
    ///
    /// Allowed:
    ///   "none" | "ingress" | "passport" | "local"
    pub mode: String,

    // ---- Passport / token validation (future) ----
    /// Optional passport / JWT issuer.
    pub passport_issuer: Option<String>,

    /// Optional passport / JWT audience.
    pub passport_audience: Option<String>,

    /// Optional JWKS URL for passport / JWT validation.
    pub passport_jwks_url: Option<String>,

    // ---- Local auth (cookie sessions + RBAC) ----
    /// Session cookie name (httpOnly).
    pub cookie_name: Option<String>,

    /// Whether to mark the session cookie as Secure.
    ///
    /// In production behind HTTPS, this should be true.
    pub cookie_secure: Option<bool>,

    /// Absolute session TTL in seconds (hard expiry).
    pub session_ttl_sec: Option<u64>,

    /// Idle session TTL in seconds (rolling expiry).
    pub session_idle_ttl_sec: Option<u64>,

    /// Path to RBAC JSON store (users/roles/permissions).
    pub rbac_path: Option<String>,

    /// Bootstrap admin username if RBAC store is empty.
    pub bootstrap_admin_username: Option<String>,

    /// Name of env var that holds the bootstrap admin password (plaintext).
    ///
    /// The password is immediately Argon2id-hashed and never persisted in raw form.
    pub bootstrap_admin_password_env: Option<String>,
}

impl AuthCfg {
    /// True if the config requests cookie-session local auth.
    pub fn is_local(&self) -> bool {
        self.mode == "local"
    }

    // ---- Default helpers (so other modules don't need to unwrap Options) ----

    pub fn cookie_name_or_default(&self) -> &str {
        self.cookie_name.as_deref().unwrap_or("svc_admin_sid")
    }

    pub fn cookie_secure_or_default(&self) -> bool {
        self.cookie_secure.unwrap_or(false)
    }

    pub fn session_ttl_sec_or_default(&self) -> u64 {
        self.session_ttl_sec.unwrap_or(3600)
    }

    pub fn session_idle_ttl_sec_or_default(&self) -> u64 {
        self.session_idle_ttl_sec.unwrap_or(900)
    }

    pub fn rbac_path_or_default(&self) -> &str {
        self.rbac_path
            .as_deref()
            .unwrap_or("data/svc-admin/rbac.json")
    }

    pub fn bootstrap_admin_username_or_default(&self) -> &str {
        self.bootstrap_admin_username.as_deref().unwrap_or("admin")
    }

    pub fn bootstrap_admin_password_env_or_default(&self) -> &str {
        self.bootstrap_admin_password_env
            .as_deref()
            .unwrap_or("SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD")
    }
}

impl Default for AuthCfg {
    fn default() -> Self {
        Self {
            // If your router has login/session wired, local-by-default is correct.
            // If not wired yet, you can temporarily set this back to "none" and
            // rely on SVC_ADMIN_AUTH_MODE=local once wiring is finished.
            mode: "local".to_string(),

            passport_issuer: None,
            passport_audience: None,
            passport_jwks_url: None,

            cookie_name: Some("svc_admin_sid".to_string()),
            cookie_secure: Some(false),
            session_ttl_sec: Some(3600),
            session_idle_ttl_sec: Some(900),
            rbac_path: Some("data/svc-admin/rbac.json".to_string()),
            bootstrap_admin_username: Some("admin".to_string()),
            bootstrap_admin_password_env: Some("SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD".to_string()),
        }
    }
}
