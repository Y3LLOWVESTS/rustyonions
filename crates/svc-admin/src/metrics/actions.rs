// crates/svc-admin/src/metrics/actions.rs
//
// WHAT: Prometheus counters for svc-admin governance / security events.
// WHY:  Make control-plane behavior observable: which actions are being
//       rejected, where auth is failing, and how often upstream nodes are
//       misbehaving.
// METRICS:
//   - ron_svc_admin_rejected_total{reason}
//       * Counts rejected control-plane actions (reload/shutdown/etc.).
//       * Example reasons: "disabled", "unauth", "forbidden", "node_not_found",
//         "upstream_error".
//   - ron_svc_admin_auth_failures_total{scope}
//       * Counts auth resolution failures, grouped by scope:
//         "ui"  → /api/me
//         "node"→ per-node actions.
//   - ron_svc_admin_upstream_errors_total{kind}
//       * Counts upstream I/O errors talking to nodes, grouped by kind:
//         "timeout", "connect", "status", "http", "parse", etc.

use std::sync::OnceLock;

use prometheus::{IntCounterVec, Opts};

fn rejected_total() -> &'static IntCounterVec {
    static REJECTED_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

    REJECTED_TOTAL.get_or_init(|| {
        let vec = IntCounterVec::new(
            Opts::new(
                "svc_admin_rejected_total",
                "Total number of rejected node actions in svc-admin.",
            )
            .namespace("ron"),
            &["reason"],
        )
        .expect("svc_admin_rejected_total counter must be constructible");

        prometheus::default_registry()
            .register(Box::new(vec.clone()))
            .expect("svc_admin_rejected_total must register successfully");

        vec
    })
}

fn auth_failures_total() -> &'static IntCounterVec {
    static AUTH_FAILURES_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

    AUTH_FAILURES_TOTAL.get_or_init(|| {
        let vec = IntCounterVec::new(
            Opts::new(
                "svc_admin_auth_failures_total",
                "Total number of auth resolution failures in svc-admin.",
            )
            .namespace("ron"),
            &["scope"],
        )
        .expect("svc_admin_auth_failures_total counter must be constructible");

        prometheus::default_registry()
            .register(Box::new(vec.clone()))
            .expect("svc_admin_auth_failures_total must register successfully");

        vec
    })
}

fn upstream_errors_total() -> &'static IntCounterVec {
    static UPSTREAM_ERRORS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

    UPSTREAM_ERRORS_TOTAL.get_or_init(|| {
        let vec = IntCounterVec::new(
            Opts::new(
                "svc_admin_upstream_errors_total",
                "Total number of upstream node I/O errors observed by svc-admin.",
            )
            .namespace("ron"),
            &["kind"],
        )
        .expect("svc_admin_upstream_errors_total counter must be constructible");

        prometheus::default_registry()
            .register(Box::new(vec.clone()))
            .expect("svc_admin_upstream_errors_total must register successfully");

        vec
    })
}

/// Increment the rejected actions counter for a given reason.
///
/// Typical reasons:
///   - "disabled"        → action disabled in config
///   - "unauth"          → auth resolution failed
///   - "forbidden"       → caller lacks required role
///   - "node_not_found"  → unknown node id
///   - "upstream_error"  → upstream node rejected/failed the action
pub fn inc_rejection(reason: &str) {
    rejected_total().with_label_values(&[reason]).inc();
}

/// Increment the auth failures counter for a given scope.
///
/// Typical scopes:
///   - "ui"   → `/api/me`
///   - "node" → node-level actions
pub fn inc_auth_failure(scope: &str) {
    auth_failures_total().with_label_values(&[scope]).inc();
}

/// Increment the upstream errors counter for a given error kind.
///
/// Typical kinds:
///   - "timeout" → request timed out
///   - "connect" → TCP connect failed
///   - "status"  → non-2xx HTTP status from upstream
///   - "http"    → other HTTP/protocol error
///   - "parse"   → invalid/malformed metrics body
pub fn inc_upstream_error(kind: &str) {
    upstream_errors_total()
        .with_label_values(&[kind])
        .inc();
}
