//! RO:WHAT — Short-horizon facet metrics store (per-node, per-facet rolling RPS/error view).
//! RO:WHY  — Pillar 4 (Observability); Concerns PERF|RES — power an operator-facing dashboard without TSDB.
//! RO:INTERACTS — crate::metrics::sampler, crate::dto::metrics::FacetMetricsSummary
//! RO:INVARIANTS — in-memory only; bounded window; no .await inside locks; monotonically increasing counters only.
//! RO:METRICS — does not emit metrics itself; aggregates node-exposed `ron_facet_requests_total{facet=...,result=...}`.
//! RO:CONFIG — window duration typically derived from `Config.polling.metrics_window`.
//! RO:SECURITY — no secrets/PII; operates solely on already-scraped Prometheus text.
//! RO:TEST — covered indirectly via sampler parsing tests + higher-level HTTP/API contracts.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::dto::metrics::FacetMetricsSummary;

/// A point-in-time snapshot of counters for a single facet on a node.
///
/// Produced by the metrics sampler after parsing Prometheus text.
#[derive(Debug, Clone)]
pub struct FacetSnapshot {
    /// Logical facet name, e.g. `"gateway.app"`, `"overlay.connect"`.
    pub facet: String,
    /// Monotonic counter of total requests for this facet.
    pub requests_total: f64,
    /// Monotonic counter of *error* requests for this facet.
    pub errors_total: f64,
}

/// Internal key: `(node_id, facet_name)`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct FacetKey {
    node_id: String,
    facet: String,
}

#[derive(Debug, Clone)]
struct FacetPoint {
    ts: Instant,
    requests_total: f64,
    errors_total: f64,
}

#[derive(Debug)]
struct Inner {
    window: Duration,
    series: HashMap<FacetKey, VecDeque<FacetPoint>>,
}

/// Short-horizon facet metrics store.
///
/// This is intentionally small and in-memory only. It exists to serve
/// `/api/nodes/{id}/metrics/facets` and similar read APIs for the
/// admin UI – not long-term historical queries.
#[derive(Clone, Debug)]
pub struct FacetMetrics {
    inner: Arc<RwLock<Inner>>,
}

impl FacetMetrics {
    /// Create a new facet metrics store with the given rolling `window`.
    ///
    /// Samples older than this window will be pruned on each update.
    pub fn new(window: Duration) -> Self {
        FacetMetrics {
            inner: Arc::new(RwLock::new(Inner {
                window,
                series: HashMap::new(),
            })),
        }
    }

    /// Return the configured rolling window.
    pub fn window(&self) -> Duration {
        let inner = self.inner.read().expect("facet metrics store poisoned");
        inner.window
    }

    /// Push a new scrape worth of facet snapshots for a single node.
    ///
    /// The sampler should call this after:
    ///
    /// 1. Fetching `/metrics` from the node.
    /// 2. Parsing it into one [`FacetSnapshot`] per facet.
    ///
    /// This method:
    /// - appends a new `FacetPoint` for each facet key
    /// - prunes samples older than the configured window
    pub fn update_from_scrape<I>(&self, node_id: &str, snapshots: I)
    where
        I: IntoIterator<Item = FacetSnapshot>,
    {
        let now = Instant::now();

        let mut inner = self.inner.write().expect("facet metrics store poisoned");
        let cutoff = now
            .checked_sub(inner.window)
            // In the extremely unlikely case that `window` is larger than
            // the `Instant` epoch, fall back to "now" to avoid panics.
            .unwrap_or(now);

        for snapshot in snapshots {
            let key = FacetKey {
                node_id: node_id.to_owned(),
                facet: snapshot.facet.clone(),
            };

            let entry = inner
                .series
                .entry(key)
                .or_insert_with(VecDeque::new);

            entry.push_back(FacetPoint {
                ts: now,
                requests_total: snapshot.requests_total,
                errors_total: snapshot.errors_total,
            });

            // Drop samples that have fallen out of the rolling window.
            while let Some(front) = entry.front() {
                if front.ts >= cutoff {
                    break;
                }
                entry.pop_front();
            }
        }

        // Clean out any series that ended up empty (all points pruned).
        inner.series.retain(|_, deque| !deque.is_empty());
    }

    /// Build facet-level summaries for a single node.
    ///
    /// We derive approximate RPS and error-rate for each facet by looking
    /// at the oldest and newest points inside the rolling window and
    /// computing deltas over elapsed wall time.
    ///
    /// Latency fields are currently stubbed to `0.0` until we begin
    /// consuming facet latency metrics from nodes.
    pub fn summaries_for_node(&self, node_id: &str) -> Vec<FacetMetricsSummary> {
        let inner = self.inner.read().expect("facet metrics store poisoned");
        let mut out = Vec::new();

        for (key, deque) in inner.series.iter() {
            if key.node_id != node_id {
                continue;
            }

            // We need at least two points to form a delta.
            if deque.len() < 2 {
                continue;
            }

            let first = match deque.front() {
                Some(p) => p,
                None => continue,
            };
            let last = match deque.back() {
                Some(p) => p,
                None => continue,
            };

            let elapsed = last.ts.saturating_duration_since(first.ts);
            if elapsed.is_zero() {
                continue;
            }

            let window_secs = elapsed.as_secs_f64();
            if window_secs <= 0.0 {
                continue;
            }

            let req_delta = (last.requests_total - first.requests_total).max(0.0);
            let err_delta = (last.errors_total - first.errors_total).max(0.0);

            // If no requests passed through this facet in the observed
            // window, we skip it – it's effectively idle.
            if req_delta <= 0.0 {
                continue;
            }

            let rps = req_delta / window_secs;
            let error_rate = if req_delta > 0.0 {
                (err_delta / req_delta).clamp(0.0, 1.0)
            } else {
                0.0
            };

            out.push(FacetMetricsSummary {
                facet: key.facet.clone(),
                rps,
                error_rate,
                // TODO: once we start ingesting facet latency metrics
                // (e.g., histograms), derive these properly.
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
            });
        }

        // Stable sort order for UI determinism.
        out.sort_by(|a, b| a.facet.cmp(&b.facet));

        out
    }
}
