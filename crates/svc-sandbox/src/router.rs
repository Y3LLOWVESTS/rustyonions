use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::AppState;

/// Verdict for the incoming request, used for telemetry/stickiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Clean,
    Suspicious,
}

/// Axum middleware: fingerprint each request, classify it, and
/// store sticky fingerprints for telemetry/diversion demos.
pub async fn deception_middleware(
    State(st): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let verdict = classify(req.headers(), req.uri().path());
    let fp = fingerprint(req.headers(), req.uri().path());

    if verdict == Verdict::Suspicious {
        st.sticky.write().insert(fp.clone());
    }
    // make fingerprint available to handlers if needed
    req.extensions_mut().insert(fp);

    next.run(req).await
}

/// Very simple fingerprint from headers + path (no IP dependency).
fn fingerprint(headers: &HeaderMap, path: &str) -> String {
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-");
    let accept = headers.get("accept").and_then(|v| v.to_str().ok()).unwrap_or("-");
    let lang = headers.get("accept-language").and_then(|v| v.to_str().ok()).unwrap_or("-");
    let mut h = DefaultHasher::new();
    ua.hash(&mut h);
    accept.hash(&mut h);
    lang.hash(&mut h);
    path.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Lightweight rules to flag obvious scanners/probes.
/// (This is *sandbox-side telemetry*, not security; ingress should still
/// apply real policy/limits.)
fn classify(headers: &HeaderMap, path: &str) -> Verdict {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let p = path.to_ascii_lowercase();

    // Known/typical scanners
    let scanners = ["curl", "wget", "nikto", "sqlmap", "nmap", "dirbuster", "gobuster", "zgrab", "masscan"];
    if ua.is_empty() || scanners.iter().any(|s| ua.contains(s)) {
        return Verdict::Suspicious;
    }

    // Suspicious paths common in drive-by scans
    let bad_paths = [
        "/.git", "/wp-admin", "/phpmyadmin", "/.env", "/server-status",
        "/etc/passwd", "../", "/admin", "/login", "/cgi-bin/",
    ];
    if bad_paths.iter().any(|s| p.contains(s)) {
        return Verdict::Suspicious;
    }

    // Odd header combos (very rough heuristic)
    if headers.get("x-forwarded-for").is_some() && headers.get("x-original-url").is_some() {
        return Verdict::Suspicious;
    }

    Verdict::Clean
}
