//! RO:WHAT — Normalizes labels/routes into bounded, non-PII Prometheus-safe values.
//! RO:WHY — Pillar 12; Concerns: PERF/SEC/GOV. Prevents cardinality and privacy blowups.
//! RO:INTERACTS — accounting::labels, metrics labels, future HTTP/OAP adapters.
//! RO:INVARIANTS — idempotent normalization; length capped; dynamic route IDs templated.
//! RO:METRICS — protects all accounting_* label dimensions from unbounded cardinality.
//! RO:CONFIG — label caps are constants in this module for Batch 1.
//! RO:SECURITY — strips obvious emails/tokens/long numeric IDs from labels.
//! RO:TEST — unit: recording_tests; prop: labels_prop.

/// Maximum bytes retained for a normalized label component.
pub const MAX_LABEL_LEN: usize = 64;

/// Normalize a generic service/region-style label component.
pub fn normalize_component(value: impl AsRef<str>) -> String {
    let mut out = String::with_capacity(value.as_ref().len().min(MAX_LABEL_LEN));
    for ch in value.as_ref().trim().chars() {
        let mapped = match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' => ch.to_ascii_lowercase(),
            '/' | ':' | ' ' | '\t' => '_',
            _ => '_',
        };
        if out.len() < MAX_LABEL_LEN {
            out.push(mapped);
        }
    }

    collapse_underscores(&out)
        .trim_matches('_')
        .to_string()
        .if_empty("unknown")
}

/// Normalize an HTTP method or synthetic method-like accounting source.
pub fn normalize_method(value: impl AsRef<str>) -> String {
    let upper = value.as_ref().trim().to_ascii_uppercase();
    if upper.is_empty()
        || upper.len() > 16
        || !upper.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        "UNKNOWN".to_string()
    } else {
        upper
    }
}

/// Normalize a route/path, replacing dynamic identifiers with stable templates.
pub fn normalize_route(route: impl AsRef<str>) -> String {
    let raw = route.as_ref().trim();
    if raw.is_empty() {
        return "/unknown".to_string();
    }

    let raw = raw.split('?').next().unwrap_or(raw);
    let mut parts = Vec::new();

    for seg in raw.trim_matches('/').split('/') {
        let raw_segment = seg.trim();
        if raw_segment.is_empty() {
            continue;
        }

        let normalized_segment = normalize_component(raw_segment);
        let part = normalize_route_segment(raw_segment, &normalized_segment);
        parts.push(part);
    }

    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

/// Return true when a string appears to contain obvious PII or high-cardinality secrets.
pub fn looks_like_pii(value: &str) -> bool {
    let value = value.trim();
    value.contains('@')
        || value.starts_with("Bearer ")
        || value.starts_with("bearer ")
        || value.len() > 96
        || value.chars().filter(|c| c.is_ascii_digit()).count() >= 12
}

fn normalize_route_segment(raw_segment: &str, normalized_segment: &str) -> String {
    if looks_like_template(raw_segment) {
        raw_segment.to_ascii_lowercase()
    } else if looks_like_content_id(raw_segment) {
        ":cid".to_string()
    } else if looks_like_pii(raw_segment) {
        ":redacted".to_string()
    } else if looks_like_dynamic_id(normalized_segment) {
        ":id".to_string()
    } else {
        normalized_segment.to_string()
    }
}

fn looks_like_template(value: &str) -> bool {
    value
        .strip_prefix(':')
        .map(|tail| !tail.is_empty() && tail.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(false)
}

fn looks_like_dynamic_id(value: &str) -> bool {
    let value = value.trim();
    let alnum_count = value.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    let digit_count = value.chars().filter(|c| c.is_ascii_digit()).count();

    value.chars().all(|c| c.is_ascii_digit())
        || (value.len() >= 8 && digit_count >= 4)
        || (value.len() >= 16 && alnum_count >= 12)
}

fn looks_like_content_id(value: &str) -> bool {
    let v = value.strip_prefix("b3:").unwrap_or(value);
    v.len() == 64 && v.chars().all(|c| c.is_ascii_hexdigit())
}

fn collapse_underscores(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_underscore = false;

    for ch in value.chars() {
        if ch == '_' {
            if !last_underscore {
                out.push(ch);
            }
            last_underscore = true;
        } else {
            out.push(ch);
            last_underscore = false;
        }
    }

    out
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
