//! RO:WHAT — Minimal capability checks for svc-rewarder HTTP routes.
//! RO:WHY — Pillar 12; Concerns: SEC/GOV. Reward computation and inspection must be capability scoped.
//! RO:INTERACTS — http handlers and future ron-auth/macaroons.
//! RO:INVARIANTS — no ambient JWT trust; dev token is explicit; scopes are route-specific.
//! RO:METRICS — auth rejects counted by handlers.
//! RO:CONFIG — future macaroon path from ingress.macaroon_path.
//! RO:SECURITY — Authorization header is never logged.
//! RO:TEST — HTTP integration includes Bearer dev.

use axum::http::HeaderMap;

use crate::{Result, RewarderError};

/// Capability scope needed by a route.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Permission to compute an epoch.
    Run,
    /// Permission to inspect manifests.
    Inspect,
}

/// Verify batch-1 bearer token semantics.
pub fn require_scope(headers: &HeaderMap, scope: Scope) -> Result<()> {
    let Some(value) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Err(RewarderError::Unauthenticated(
            "missing authorization".into(),
        ));
    };
    let value = value
        .to_str()
        .map_err(|_| RewarderError::Unauthenticated("invalid authorization header".into()))?;
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(RewarderError::Unauthenticated(
            "authorization must be Bearer token".into(),
        ));
    };
    if token == "dev" {
        return Ok(());
    }
    let needed = match scope {
        Scope::Run => "rewarder.run",
        Scope::Inspect => "rewarder.inspect",
    };
    if token.split(',').any(|part| part.trim() == needed) {
        Ok(())
    } else {
        Err(RewarderError::Unauthorized(format!(
            "missing scope {needed}"
        )))
    }
}
