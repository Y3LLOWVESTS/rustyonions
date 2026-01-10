// crates/macronode/src/http_admin/middleware/auth.rs
//! RO:WHAT — Admin auth middleware.
//! RO:WHY  — Guard sensitive POST endpoints (`/api/v1/shutdown`, `/api/v1/reload`, bench start).
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

pub async fn layer(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Guard only specific POST endpoints that mutate or can load the node.
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let needs_guard = method == Method::POST
        && (path == "/api/v1/shutdown"
            || path == "/api/v1/reload"
            || path == "/api/v1/debug/crash"
            || path == "/api/v1/bench/run");

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
