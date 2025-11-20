//! RO:WHAT — Admin auth middleware.
//! RO:WHY  — Guard sensitive POST endpoints (`/api/v1/shutdown`, `/api/v1/reload`).
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
use std::net::IpAddr;
use tracing::{info, warn};

pub async fn layer(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Only guard POST /shutdown & POST /reload
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let needs_guard =
        method == Method::POST && (path == "/api/v1/shutdown" || path == "/api/v1/reload");

    if !needs_guard {
        return Ok(next.run(req).await);
    }

    // Explicit dev bypass
    if dev_insecure() {
        warn!("MACRONODE_DEV_INSECURE=1 — bypassing admin auth for {method} {path}");
        return Ok(next.run(req).await);
    }

    // Determine if the admin listener is loopback-only
    let is_loopback = match req.headers().get("host").and_then(|h| h.to_str().ok()) {
        Some(host) => host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(true),
        None => true,
    };

    // Determine token
    let expected_token = std::env::var("RON_ADMIN_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    match expected_token {
        Some(expected) => {
            // Token required — validate header
            let auth_header = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|h| h.to_str().ok());

            let ok = auth_header
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
                warn!("RON_ADMIN_TOKEN is not set — allowing admin action on loopback {method} {path}");
                Ok(next.run(req).await)
            } else {
                warn!("BLOCKED admin action — RON_ADMIN_TOKEN missing + non-loopback bind {method} {path}");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}

fn dev_insecure() -> bool {
    matches!(
        std::env::var("MACRONODE_DEV_INSECURE").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
    )
}
