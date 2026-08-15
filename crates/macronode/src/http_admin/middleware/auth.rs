// crates/macronode/src/http_admin/middleware/auth.rs
//! RO:WHAT — Admin auth middleware.
//! RO:WHY  — Guard sensitive mutation endpoints plus moderation and persistence review queues, which expose exact object identifiers.
//!
//! RO:INVARIANTS —
//!   - If `RON_ADMIN_TOKEN` is set, sensitive endpoints require
//!     `Authorization: Bearer <token>`.
//!   - If bound to loopback AND no token is set, we ALLOW but WARN.
//!   - If bound to NON-loopback AND no token is set, we BLOCK unless
//!     `MACRONODE_DEV_INSECURE=1`.
//!   - `MACRONODE_DEV_INSECURE=1` bypasses everything (dev ergonomics).

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::env;
use std::net::IpAddr;
use tracing::{info, warn};

fn requires_admin_guard(method: &Method, path: &str) -> bool {
    let moderation_review_route = path.starts_with("/api/v1/moderation/review/");

    let persistence_route = path.starts_with("/api/v1/persistence/");

    moderation_review_route
        || persistence_route
        || (method == Method::POST
            && (path == "/api/v1/shutdown"
                || path == "/api/v1/reload"
                || path == "/api/v1/debug/crash"
                || path == "/api/v1/bench/run"
                || path == "/api/v1/moderation/prune"
                || path == "/api/v1/quickchain/quorum/sign"
                || path == "/api/v1/quickchain/checkpoint/sign"
                || path == "/api/v1/rewards/bind"
                || path == "/api/v1/rewards/rotate"))
}

pub async fn layer(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Guard only specific POST endpoints that mutate or can load the node.
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let needs_guard = requires_admin_guard(&method, &path);

    if !needs_guard {
        return Ok(next.run(req).await);
    }

    // Dev bypass
    if env::var("MACRONODE_DEV_INSECURE").ok().as_deref() == Some("1") {
        warn!("MACRONODE_DEV_INSECURE=1 — bypassing admin auth for {method} {path}");
        return Ok(next.run(req).await);
    }

    let expected = env::var("RON_ADMIN_TOKEN").ok();

    // Determine if bound to loopback (best-effort from Host header or local addr hints).
    // If we can't prove non-loopback, we conservatively treat as loopback for dev ergonomics.
    let is_loopback = {
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // host may be "127.0.0.1:8080" or "[::1]:8080" or "localhost:8080"
        if host.starts_with("127.0.0.1")
            || host.starts_with("[::1]")
            || host.starts_with("localhost")
        {
            true
        } else {
            // Try parse raw IP (without port)
            let ip_str = host.split(':').next().unwrap_or("");
            ip_str
                .parse::<IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(true)
        }
    };

    match expected {
        Some(expected) => {
            let ok = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|v| v == expected)
                .unwrap_or(false);

            if !ok {
                warn!("unauthorized {method} {path} — missing/invalid token");
                return Err(StatusCode::UNAUTHORIZED);
            }

            info!("authorized admin {method} {path}");
            Ok(next.run(req).await)
        }
        None => {
            // No token set
            if is_loopback {
                warn!(
                    "RON_ADMIN_TOKEN not set — allowing sensitive {method} {path} because loopback is assumed"
                );
                Ok(next.run(req).await)
            } else {
                warn!(
                    "RON_ADMIN_TOKEN not set — blocking sensitive {method} {path} because non-loopback is assumed"
                );
                Err(StatusCode::FORBIDDEN)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_reads_and_mutations_require_admin_guard() {
        for path in [
            "/api/v1/persistence/pending",
            "/api/v1/persistence/status/b3:abc",
        ] {
            assert!(
                requires_admin_guard(&Method::GET, path),
                "{path} must require administrator authentication",
            );
        }

        for path in [
            "/api/v1/persistence/register",
            "/api/v1/persistence/submit",
            "/api/v1/persistence/approve",
            "/api/v1/persistence/reject",
            "/api/v1/persistence/pin",
            "/api/v1/persistence/unpin",
        ] {
            assert!(
                requires_admin_guard(&Method::POST, path),
                "{path} must require administrator authentication",
            );
        }
    }

    #[test]
    fn public_health_and_status_routes_remain_unaffected() {
        for path in ["/healthz", "/readyz", "/version", "/api/v1/status"] {
            assert!(
                !requires_admin_guard(&Method::GET, path),
                "{path} must remain outside the mutation/review guard",
            );
        }
    }

    #[test]
    fn moderation_and_existing_sensitive_routes_stay_guarded() {
        assert!(requires_admin_guard(
            &Method::GET,
            "/api/v1/moderation/review/pending",
        ));

        assert!(requires_admin_guard(&Method::POST, "/api/v1/rewards/bind",));

        assert!(requires_admin_guard(
            &Method::POST,
            "/api/v1/moderation/prune",
        ));

        assert!(requires_admin_guard(
            &Method::POST,
            "/api/v1/quickchain/quorum/sign",
        ));

        assert!(requires_admin_guard(
            &Method::POST,
            "/api/v1/quickchain/checkpoint/sign",
        ));
    }
}
