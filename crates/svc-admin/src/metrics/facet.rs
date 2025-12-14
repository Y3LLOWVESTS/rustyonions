// crates/svc-admin/src/metrics/facet.rs
//
// RO:WHAT — Short-horizon facet metrics store (per-node, per-facet rolling RPS/error view).
// RO:WHY  — Pillar 4 (Observability); Concerns PERF|RES — power an operator-facing dashboard without TSDB.
// RO:INTERACTS — crate::metrics::sampler, crate::dto::metrics::FacetMetricsSummary
// RO:INVARIANTS — in-memory only; bounded window; no .await inside locks; monotonically increasing counters only.
// RO:METRICS — does not emit metrics itself; aggregates node-exposed
//              `ron_facet_requests_total{facet=...,result=...}`.
// RO:CONFIG — window duration typically derived from `Config.polling.metrics_window`.
// RO:SECURITY — no secrets/PII; operates solely on already-scraped Prometheus text.

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

            let entry = inner.series.entry(key).or_default();

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
    /// We *prefer* to derive approximate RPS and error-rate per facet by
    /// looking at the oldest and newest points inside the rolling window
    /// and computing deltas over elapsed wall time.
    ///
    /// However, for operator ergonomics we **always** surface facets as
    /// soon as we have at least one sample for them:
    ///
    /// - If we have 2+ samples and a non-zero window, we compute true
    ///   rate-based stats.
    /// - If we only have a single sample (or zero delta), we show:
    ///     * `rps = 0.0`
    ///     * `error_rate` derived from the latest totals (or `0.0`).
    ///
    /// Latency fields are currently stubbed to `0.0` until we begin
    /// consuming facet latency metrics from nodes.
    ///
    /// NEW:
    /// - We also expose sampler health via `last_sample_age_secs`, which
    ///   is the age (in seconds) of the most recent sample for each facet.
    pub fn summaries_for_node(&self, node_id: &str) -> Vec<FacetMetricsSummary> {
        let now = Instant::now();
        let inner = self.inner.read().expect("facet metrics store poisoned");
        let mut out = Vec::new();

        for (key, deque) in inner.series.iter() {
            if key.node_id != node_id {
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
            let window_secs = elapsed.as_secs_f64();

            // Default: static view derived from the most recent totals.
            let mut rps = 0.0;
            let mut error_rate = {
                let req = last.requests_total.max(0.0);
                let err = last.errors_total.clamp(0.0, req);
                if req > 0.0 {
                    (err / req).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            };

            // If we have a real window (2+ points and non-zero duration),
            // upgrade to a rolling rate-based view.
            if deque.len() >= 2 && window_secs > 0.0 {
                let req_delta = (last.requests_total - first.requests_total).max(0.0);
                let err_delta = (last.errors_total - first.errors_total).max(0.0);

                if req_delta > 0.0 {
                    rps = req_delta / window_secs;
                    error_rate = (err_delta / req_delta).clamp(0.0, 1.0);
                } else {
                    // No traffic during the observed window – keep rps=0
                    // and fall back to the latest error_rate we computed
                    // above (typically 0.0).
                }
            }

            // Age of the most recent sample for this facet, as observed by
            // the sampler. This drives Metrics: Fresh/Stale UI.
            let last_sample_age_secs = now
                .checked_duration_since(last.ts)
                .map(|d| d.as_secs_f64());

            out.push(FacetMetricsSummary {
                facet: key.facet.clone(),
                rps,
                error_rate,
                // TODO: once we start ingesting facet latency metrics
                // (e.g., histograms), derive these properly.
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                last_sample_age_secs,
            });
        }

        // Stable sort order for UI determinism.
        out.sort_by(|a, b| a.facet.cmp(&b.facet));

        out
    }
}
