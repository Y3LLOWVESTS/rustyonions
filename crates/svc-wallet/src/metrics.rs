//! RO:WHAT — Minimal in-process metrics registry and Prometheus text renderer for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/PERF/RES/GOV. Wallet economics need visible successes, rejects, replays, and readiness.
//! RO:INTERACTS — routes, readiness, middleware, future ron-metrics bridge.
//! RO:INVARIANTS — metrics are derivative only; never replace ron-ledger truth; no locks across .await.
//! RO:METRICS — wallet_requests_total, wallet_rejects_total, wallet_idempotency_replays_total, wallet_ops_total.
//! RO:CONFIG — none; labels derive from stable enums only.
//! RO:SECURITY — never records bearer tokens, memos, account ids, or raw request bodies.
//! RO:TEST — render_includes_core_series.

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use parking_lot::Mutex;

use crate::dto::responses::WalletOp;

/// Cloneable wallet metrics handle.
#[derive(Clone, Debug, Default)]
pub struct WalletMetrics {
    inner: Arc<WalletMetricsInner>,
}

#[derive(Debug, Default)]
struct WalletMetricsInner {
    requests_total: AtomicU64,
    successes_total: AtomicU64,
    idempotency_replays_total: AtomicU64,
    inflight: AtomicU64,
    rejects_by_reason: Mutex<BTreeMap<String, u64>>,
    ops_by_name: Mutex<BTreeMap<String, u64>>,
}

impl WalletMetrics {
    /// Increment total route request count.
    pub fn inc_request(&self) {
        self.inner.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment successful request count.
    pub fn inc_success(&self) {
        self.inner.successes_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment operation counter by wallet operation kind.
    pub fn inc_op(&self, op: WalletOp) {
        let mut guard = self.inner.ops_by_name.lock();
        let entry = guard.entry(op.as_str().to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Increment reject counter by stable reason code.
    pub fn inc_reject(&self, reason: &str) {
        let mut guard = self.inner.rejects_by_reason.lock();
        let entry = guard.entry(reason.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Increment idempotent replay counter.
    pub fn inc_idempotency_replay(&self) {
        self.inner
            .idempotency_replays_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Mark one in-flight request.
    pub fn begin_request(&self) -> InFlightGuard {
        self.inc_request();
        self.inner.inflight.fetch_add(1, Ordering::Relaxed);
        InFlightGuard {
            metrics: self.clone(),
        }
    }

    fn end_request(&self) {
        self.inner.inflight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Render a Prometheus-compatible text exposition.
    pub fn render_prometheus(&self, ready: bool) -> String {
        let mut out = String::new();

        out.push_str("# HELP wallet_requests_total Total svc-wallet route requests.\n");
        out.push_str("# TYPE wallet_requests_total counter\n");
        out.push_str(&format!(
            "wallet_requests_total {}\n",
            self.inner.requests_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP wallet_successes_total Total successful svc-wallet requests.\n");
        out.push_str("# TYPE wallet_successes_total counter\n");
        out.push_str(&format!(
            "wallet_successes_total {}\n",
            self.inner.successes_total.load(Ordering::Relaxed)
        ));

        out.push_str(
            "# HELP wallet_idempotency_replays_total Total idempotent response replays.\n",
        );
        out.push_str("# TYPE wallet_idempotency_replays_total counter\n");
        out.push_str(&format!(
            "wallet_idempotency_replays_total {}\n",
            self.inner.idempotency_replays_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP wallet_inflight Current in-flight wallet requests.\n");
        out.push_str("# TYPE wallet_inflight gauge\n");
        out.push_str(&format!(
            "wallet_inflight {}\n",
            self.inner.inflight.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP wallet_ready Wallet readiness as 1 or 0.\n");
        out.push_str("# TYPE wallet_ready gauge\n");
        out.push_str(&format!("wallet_ready {}\n", u8::from(ready)));

        out.push_str("# HELP wallet_rejects_total Wallet rejects by stable reason.\n");
        out.push_str("# TYPE wallet_rejects_total counter\n");
        for (reason, count) in self.inner.rejects_by_reason.lock().iter() {
            out.push_str(&format!(
                "wallet_rejects_total{{reason=\"{}\"}} {}\n",
                escape_label(reason),
                count
            ));
        }

        out.push_str("# HELP wallet_ops_total Wallet committed operations by kind.\n");
        out.push_str("# TYPE wallet_ops_total counter\n");
        for (op, count) in self.inner.ops_by_name.lock().iter() {
            out.push_str(&format!(
                "wallet_ops_total{{op=\"{}\"}} {}\n",
                escape_label(op),
                count
            ));
        }

        out
    }
}

/// RAII in-flight metric guard.
#[derive(Debug)]
pub struct InFlightGuard {
    metrics: WalletMetrics,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.metrics.end_request();
    }
}

fn escape_label(value: &str) -> String {
    value.replace('\\', r"\\").replace('"', r#"\""#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_core_series() {
        let metrics = WalletMetrics::default();
        metrics.inc_request();
        metrics.inc_success();
        metrics.inc_op(WalletOp::Issue);
        metrics.inc_reject("BAD_REQUEST");

        let rendered = metrics.render_prometheus(true);
        assert!(rendered.contains("wallet_requests_total 1"));
        assert!(rendered.contains("wallet_successes_total 1"));
        assert!(rendered.contains("wallet_ops_total{op=\"issue\"} 1"));
        assert!(rendered.contains("wallet_rejects_total{reason=\"BAD_REQUEST\"} 1"));
        assert!(rendered.contains("wallet_ready 1"));
    }
}
