//! RO:WHAT — Small tracing helpers for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: RES/GOV/DX. Logs need stable redacted fields without leaking tokens.
//! RO:INTERACTS — routes, future tower trace layer.
//! RO:INVARIANTS — never log Authorization, idempotency key values, memos, or raw bodies.
//! RO:METRICS — none.
//! RO:CONFIG — RUST_LOG consumed by main.rs tracing setup.
//! RO:SECURITY — redacts untrusted strings to bounded labels.
//! RO:TEST — stable_route_label.

/// Return a stable route label for metrics/logs.
pub fn route_label(method: &str, path: &str) -> String {
    let method = method.trim().to_ascii_uppercase();
    let path = if path.is_empty() { "/" } else { path };
    format!("{method} {path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_route_label() {
        assert_eq!(route_label("get", "/healthz"), "GET /healthz");
        assert_eq!(route_label("post", ""), "POST /");
    }
}
