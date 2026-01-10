//! RO:WHAT — RBAC types for svc-admin (roles, permissions, effective access).
//! RO:WHY  — Pillar 3 Identity; Concerns: SEC/GOV. Least-privilege for admin actions.
//! RO:INTERACTS — crate::auth::local (session auth), HTTP handlers for /api/rbac/*.
//! RO:INVARIANTS — deny-by-default; permissions are server-enforced; no lock across .await.
//! RO:SECURITY — no PII beyond username; never store raw passwords (Argon2id PHC only).
//! RO:TEST — unit: role resolution; property: unknown perms rejected.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Stable permission IDs (stringly-typed, but centrally enumerated).
/// Keep these short and durable; the UI can group/display them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PermissionId(pub String);

impl PermissionId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: BTreeSet<PermissionId>,
    #[serde(default)]
    pub built_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    /// PHC string, Argon2id. Never store raw passwords.
    pub password_phc: String,
    #[serde(default)]
    pub roles: BTreeSet<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub created_at_unix_s: u64,
    #[serde(default)]
    pub last_login_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacData {
    #[serde(default)]
    pub roles: BTreeMap<String, Role>,
    #[serde(default)]
    pub users: BTreeMap<String, User>,
}

impl RbacData {
    pub fn empty() -> Self {
        Self {
            roles: BTreeMap::new(),
            users: BTreeMap::new(),
        }
    }

    pub fn ensure_builtins(&mut self) {
        // Admin: everything.
        let mut admin_perms = BTreeSet::new();
        for &p in builtin_permissions() {
            admin_perms.insert(PermissionId::new(p));
        }
        self.roles.entry("admin".to_string()).or_insert(Role {
            name: "admin".to_string(),
            description: "Full access".to_string(),
            permissions: admin_perms,
            built_in: true,
        });

        // Operator: operational actions, but no user/role mgmt by default.
        let mut operator = BTreeSet::new();
        for p in [
            "nodes:read",
            "nodes:write",
            "network:read",
            "config:read",
            "config:write",
            "policy:read",
            "policy:write",
        ] {
            operator.insert(PermissionId::new(p));
        }
        self.roles.entry("operator".to_string()).or_insert(Role {
            name: "operator".to_string(),
            description: "Operate nodes & configs".to_string(),
            permissions: operator,
            built_in: true,
        });

        // Viewer: read-only.
        let mut viewer = BTreeSet::new();
        for p in ["nodes:read", "network:read", "config:read", "policy:read"] {
            viewer.insert(PermissionId::new(p));
        }
        self.roles.entry("viewer".to_string()).or_insert(Role {
            name: "viewer".to_string(),
            description: "Read-only access".to_string(),
            permissions: viewer,
            built_in: true,
        });
    }

    pub fn effective_permissions_for_user(&self, user: &User) -> BTreeSet<PermissionId> {
        let mut out = BTreeSet::new();
        for role_name in &user.roles {
            if let Some(role) = self.roles.get(role_name) {
                out.extend(role.permissions.iter().cloned());
            }
        }
        out
    }
}

pub fn builtin_permissions() -> &'static [&'static str] {
    &[
        "nodes:read",
        "nodes:write",
        "network:read",
        "config:read",
        "config:write",
        "policy:read",
        "policy:write",
        "users:read",
        "users:write",
        "roles:read",
        "roles:write",
        "audit:read",
    ]
}
