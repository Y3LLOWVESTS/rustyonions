// crates/svc-admin/src/config/auth.rs
//
// WHAT: Auth configuration (none / ingress / passport) for svc-admin.

use serde::{Deserialize, Serialize};

/// Auth config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCfg {
    /// "none" | "ingress" | "passport"
    pub mode: String,

    /// Optional passport / JWT issuer.
    pub passport_issuer: Option<String>,

    /// Optional passport / JWT audience.
    pub passport_audience: Option<String>,

    /// Optional JWKS URL for passport / JWT validation.
    pub passport_jwks_url: Option<String>,
}

impl Default for AuthCfg {
    fn default() -> Self {
        Self {
            mode: "none".to_string(),
            passport_issuer: None,
            passport_audience: None,
            passport_jwks_url: None,
        }
    }
}
