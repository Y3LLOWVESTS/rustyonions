//! RO:WHAT — Public DTO privacy review helpers for svc-dht.
//! RO:WHY — BUILD_PLAN_Z Phase 7 requires DHT/provider public surfaces to avoid
//!   residential IPs, socket addresses, and transport-specific routes.
//! RO:INTERACTS — tests/privacy_no_ip_leak.rs, rpc::http JSON outputs, provider debug snapshots.
//! RO:INVARIANTS — crab://node identities are allowed; raw IP/socket/transport routes are findings.
//! RO:SECURITY — catches accidental source_ip/viewer_ip/socket_addr/raw route drift in public JSON.
//! RO:TEST — tests/privacy_no_ip_leak.rs.

use serde_json::Value;

const FORBIDDEN_PUBLIC_IP_KEYS: &[&str] = &[
    "source_ip",
    "viewer_ip",
    "creator_ip",
    "residential_ip",
    "lan_ip",
    "peer_ip",
    "client_ip",
    "remote_ip",
    "remote_addr",
    "socket_addr",
    "ip_addr",
];

const FORBIDDEN_PUBLIC_ROUTE_SCHEMES: &[&str] = &[
    "http://",
    "https://",
    "tcp://",
    "udp://",
    "ws://",
    "wss://",
    "local://",
    "relay://",
    "onion://",
    "service://",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicLeakFinding {
    pub path: String,
    pub reason: &'static str,
    pub value: Option<String>,
}

impl PublicLeakFinding {
    fn new(path: String, reason: &'static str, value: Option<String>) -> Self {
        Self { path, reason, value }
    }
}

pub fn review_public_json(surface: &str, value: &Value) -> Vec<PublicLeakFinding> {
    let mut findings = Vec::new();
    visit_json(surface, value, &mut findings);
    findings
}

pub fn is_forbidden_public_ip_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    FORBIDDEN_PUBLIC_IP_KEYS.contains(&normalized.as_str())
}

pub fn contains_forbidden_public_route_or_ip(value: &str) -> bool {
    contains_forbidden_route_scheme(value) || contains_ipv4_literal(value)
}

fn visit_json(path: &str, value: &Value, findings: &mut Vec<PublicLeakFinding>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}.{key}");

                if is_forbidden_public_ip_key(key) {
                    findings.push(PublicLeakFinding::new(
                        child_path.clone(),
                        "forbidden_ip_field_name",
                        Some(key.clone()),
                    ));
                }

                visit_json(&child_path, child, findings);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                let child_path = format!("{path}[{idx}]");
                visit_json(&child_path, child, findings);
            }
        }
        Value::String(text) => {
            if contains_forbidden_public_route_or_ip(text) {
                findings.push(PublicLeakFinding::new(
                    path.to_string(),
                    "forbidden_route_or_ip_literal",
                    Some(text.clone()),
                ));
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn contains_forbidden_route_scheme(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    FORBIDDEN_PUBLIC_ROUTE_SCHEMES.iter().any(|scheme| normalized.contains(scheme))
}

fn contains_ipv4_literal(value: &str) -> bool {
    let mut token = String::new();

    for ch in value.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() || ch == '.' {
            token.push(ch);
            continue;
        }

        if looks_like_ipv4_literal(&token) {
            return true;
        }

        token.clear();
    }

    false
}

fn looks_like_ipv4_literal(candidate: &str) -> bool {
    let mut parts_seen = 0usize;

    for part in candidate.split('.') {
        parts_seen += 1;

        if part.is_empty() || part.len() > 3 {
            return false;
        }

        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }

        if part.parse::<u8>().is_err() {
            return false;
        }
    }

    parts_seen == 4
}
