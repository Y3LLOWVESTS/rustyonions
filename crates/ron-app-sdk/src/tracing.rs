//! RO:WHAT — Span/field helper utilities for SDK calls.
//! RO:WHY  — Keep span naming/redaction/correlation consistent across
//!           storage/edge/mailbox/index planes without hardwiring to
//!           any particular tracing backend.
//! RO:INTERACTS — Uses `TracingCfg` + `Redaction` from `config`; callers
//!                may translate `SpanFields` into actual `tracing` spans.
//! RO:INVARIANTS —
//!   - Does not depend on external tracing crates (pure data only).
//!   - Redaction rules centralized and testable.
//!   - Endpoint field is stable and low-cardinality (path-like).
//! RO:METRICS — None directly; fields are reused by metrics labels.
//! RO:CONFIG — Reads `TracingCfg { spans, metrics, redaction }`.
//! RO:SECURITY — Redaction mode aims to strip query strings and obvious
//!               identifiers from endpoints when `Redaction::Safe`.
//! RO:TEST — Unit tests for redaction + field shaping.

use std::borrow::Cow;
use std::time::Duration;

use crate::config::{Redaction, TracingCfg};

/// Data model for span fields that SDK callers may attach to a tracing span.
///
/// This keeps the *shape* of our tracing consistent without forcing a
/// particular tracing backend into the SDK crate.
#[derive(Debug, Clone)]
pub struct SpanFields<'a> {
    /// Stable endpoint identifier (e.g., `/storage/put`).
    pub endpoint: Cow<'a, str>,
    /// Retry attempt number (0 == first attempt).
    pub attempt: u32,
    /// Deadline in milliseconds from "now" when the call started.
    pub deadline_ms: u64,
    /// Optional correlation ID propagated from the caller.
    pub corr_id: Option<Cow<'a, str>>,
}

/// Convenience function to build span fields for an SDK call.
///
/// Callers can use this to populate a `tracing::Span`, log entry, or
/// metrics label set; the SDK itself remains backend-agnostic.
pub fn build_span_fields<'a>(
    cfg: &TracingCfg,
    endpoint: &'a str,
    attempt: u32,
    overall_deadline: Duration,
    corr_id: Option<&'a str>,
) -> Option<SpanFields<'a>> {
    if !cfg.spans {
        // Spans disabled at config level — callers can short-circuit.
        return None;
    }

    let endpoint_field = match cfg.redaction {
        Redaction::Safe => redact_endpoint(endpoint),
        Redaction::None => Cow::Borrowed(endpoint),
    };

    let deadline_ms = overall_deadline.as_millis() as u64;
    let corr_field = corr_id
        .map(|v| Cow::Owned(v.to_string()));

    Some(SpanFields {
        endpoint: endpoint_field,
        attempt,
        deadline_ms,
        corr_id: corr_field,
    })
}

/// Apply a conservative redaction policy to endpoint strings.
///
/// Current policy:
/// - Strip query strings (`?…`).
/// - Collapse multiple consecutive slashes.
/// - Leave path segments untouched (gateway should avoid PII in paths).
fn redact_endpoint(raw: &str) -> Cow<'_, str> {
    let mut s = raw;

    if let Some(idx) = raw.find('?') {
        s = &raw[..idx];
    }

    // Simple collapse of "//" → "/" to avoid noisy paths.
    if s.contains("//") {
        let collapsed = s
            .split('/')
            .filter(|seg| !seg.is_empty())
            .collect::<Vec<_>>()
            .join("/");
        Cow::Owned(format!("/{}", collapsed))
    } else {
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_disabled_returns_none() {
        let cfg = TracingCfg {
            spans: false,
            metrics: true,
            redaction: Redaction::Safe,
        };

        let out = build_span_fields(&cfg, "/storage/put", 0, Duration::from_millis(5000), None);
        assert!(out.is_none());
    }

    #[test]
    fn strips_query_and_collapses_slashes() {
        let cfg = TracingCfg {
            spans: true,
            metrics: true,
            redaction: Redaction::Safe,
        };

        let out = build_span_fields(
            &cfg,
            "//storage//put?id=123&foo=bar",
            1,
            Duration::from_millis(1000),
            Some("corr-123"),
        )
        .unwrap();

        assert_eq!(out.endpoint.as_ref(), "/storage/put");
        assert_eq!(out.attempt, 1);
        assert_eq!(out.corr_id.as_deref(), Some("corr-123"));
    }

    #[test]
    fn redaction_none_keeps_query() {
        let cfg = TracingCfg {
            spans: true,
            metrics: true,
            redaction: Redaction::None,
        };

        let out = build_span_fields(
            &cfg,
            "/storage/put?id=123",
            0,
            Duration::from_millis(1000),
            None,
        )
        .unwrap();

        assert_eq!(out.endpoint.as_ref(), "/storage/put?id=123");
    }
}
