//! RO:WHAT — Permit vocabulary for Macronode admission control.
//! RO:WHY  — Give us a tiny, crate-local language for “may this caller do
//!           X?” without committing to any particular policy backend.
//!
//! RO:INVARIANTS —
//!   - Pure data types; no I/O, no global state.
//!   - Safe to round-trip through JSON/TOML if needed later.

#![allow(dead_code)]

/// Coarse-grained operation the caller is attempting.
///
/// This is intentionally small and MACRO-local; richer detail belongs in
/// the policy layer (e.g. ron-policy, auth/kms services).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitKind {
    /// POST /api/v1/shutdown on the admin plane.
    AdminShutdown,
    /// POST /api/v1/reload on the admin plane.
    AdminReload,
    /// Inbound request on the gateway plane (svc-gateway).
    GatewayIngress,
    /// Read from storage plane (svc-storage).
    StorageRead,
    /// Write to storage plane (svc-storage).
    StorageWrite,
    /// Delete from storage plane (svc-storage).
    StorageDelete,
    /// Catch-all hook for future callers (feature flags, experiements).
    Custom(&'static str),
}

/// Minimal request context passed to a permit evaluator.
///
/// Foundation cut keeps this tiny. Higher layers can wrap this with richer
/// context (capability tokens, tenant IDs, paths, etc.) as needed.
#[derive(Debug, Clone)]
pub struct PermitRequest {
    /// Which logical operation is being attempted.
    pub kind: PermitKind,
    /// Optional identity for the caller (e.g. tenant/user ID).
    pub subject: Option<String>,
    /// Optional opaque capability/macaroon token (already parsed at edges).
    pub capability: Option<String>,
    /// Optional resource hint (e.g. HTTP path, bucket name).
    pub resource: Option<String>,
}

impl PermitRequest {
    #[must_use]
    pub fn new(kind: PermitKind) -> Self {
        Self {
            kind,
            subject: None,
            capability: None,
            resource: None,
        }
    }

    #[must_use]
    pub fn with_subject<S: Into<String>>(mut self, subject: S) -> Self {
        self.subject = Some(subject.into());
        self
    }

    #[must_use]
    pub fn with_capability<S: Into<String>>(mut self, cap: S) -> Self {
        self.capability = Some(cap.into());
        self
    }

    #[must_use]
    pub fn with_resource<S: Into<String>>(mut self, res: S) -> Self {
        self.resource = Some(res.into());
        self
    }
}

/// Result of a permit check.
///
/// This remains deliberately simple; a future slice could add structured
/// denial reasons or audit codes.
#[derive(Debug, Clone)]
pub enum PermitDecision {
    /// Operation is allowed to proceed.
    Allow,
    /// Operation is denied, with a human-readable reason.
    Deny { reason: String },
}

impl PermitDecision {
    /// Convenience helper to construct a denial.
    #[must_use]
    pub fn deny<S: Into<String>>(reason: S) -> Self {
        PermitDecision::Deny {
            reason: reason.into(),
        }
    }

    /// Returns true if the decision is an allow.
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        matches!(self, PermitDecision::Allow)
    }
}
