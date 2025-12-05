// crates/svc-admin/src/dto/me.rs
//
// WHAT: DTO for `/api/me`.
// WHY:  SPA uses this to understand who the current operator is and how the
//       console is authenticated (if at all).
//
// JSON shape (from ALL_DOCS):
//
// {
//   "subject": "stevan@example.com",
//   "displayName": "Stevan White",
//   "roles": ["admin"],
//   "authMode": "passport",
//   "loginUrl": null
// }
//
// In `auth.mode = "none"`, we return a synthetic dev identity. In `ingress`
// and future `passport` modes, this reflects the real identity.

use serde::{Deserialize, Serialize};

use crate::auth::Identity;
use crate::config::AuthCfg;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    /// Stable subject identifier (e.g. email, username).
    pub subject: String,
    /// Human-readable display name for UI.
    pub display_name: String,
    /// Roles associated with this operator (e.g. ["admin", "ops"]).
    pub roles: Vec<String>,
    /// Auth mode currently in effect ("none", "ingress", "passport", ...).
    pub auth_mode: String,
    /// Optional login URL for interactive flows (primarily passport mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_url: Option<String>,
}

impl MeResponse {
    /// Build a response from an `Identity` + auth config.
    ///
    /// For now, we don't surface a `loginUrl`; future passport integration may
    /// populate this when returning 401s.
    pub fn from_identity(identity: Identity, auth_cfg: &AuthCfg) -> Self {
        MeResponse {
            subject: identity.subject,
            display_name: identity.display_name,
            roles: identity.roles,
            auth_mode: auth_cfg.mode.clone(),
            login_url: None,
        }
    }

    /// Legacy dev-only helper used before auth modes existed.
    ///
    /// This is kept for tests or standalone usage, but `/api/me` now goes
    /// through `from_identity`.
    pub fn dev_default() -> Self {
        MeResponse {
            subject: "dev-operator".to_string(),
            display_name: "Dev Operator".to_string(),
            roles: vec!["admin".to_string()],
            auth_mode: "none".to_string(),
            login_url: None,
        }
    }
}
