//! RO:WHAT — Background sampler tasks that scrape node `/metrics` and feed facet aggregates.
//! RO:WHY  — Pillar 4 (Observability); Concerns PERF|RES — live health without overloading nodes or blocking shutdown.
//! RO:INTERACTS — crate::metrics::facet::{FacetMetrics, FacetSnapshot}, NodeCfg-derived targets, Prometheus text endpoints.
//! RO:INVARIANTS — sampler loops observe shutdown; no blocking I/O under shared locks; degrade gracefully on parse/HTTP errors.
//! RO:METRICS — does not expose Prometheus metrics directly; it drives `FacetMetrics` which backs admin API/UX.
//! RO:CONFIG — interval/timeout/window typically derived from `Config.polling.*` and per-node `NodeCfg`.
//! RO:SECURITY — only reads public `/metrics`; no credentials/PII; rely on upstream nodes for any auth.
//! RO:TEST — unit: `parse_facet_snapshots_aggregates_by_facet`; integration: future HTTP/admin API tests.

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, warn};

use crate::metrics::facet::{FacetMetrics, FacetSnapshot};

/// A concrete node target for the sampler.
///
/// This is intentionally decoupled from `NodeCfg` so that we can
/// construct targets from configuration, tests, or ad-hoc callers
/// without hard-coding config types here.
#[derive(Clone, Debug)]
pub struct NodeMetricsTarget {
    /// Logical node id (e.g., `"example-node"`).
    pub node_id: String,
    /// Full URL to the node’s Prometheus `/metrics` endpoint.
    pub metrics_url: String,
    /// Optional per-request timeout for polling this node.
    pub timeout: Option<Duration>,
}

/// Spawn one sampler task per target.
///
/// Each sampler will:
/// - Align to the current time.
/// - Poll `<metrics_url>` every `interval`.
/// - Parse facet metrics and push them into `facet_metrics`.
/// - Exit promptly when `shutdown` flips to `true`.
///
/// Returns a vector of `JoinHandle<()>` so the caller can await or
/// detach them as part of the broader server lifecycle.
pub fn spawn_samplers(
    targets: Vec<NodeMetricsTarget>,
    interval: Duration,
    facet_metrics: FacetMetrics,
    shutdown: watch::Receiver<bool>,
) -> Vec<JoinHandle<()>> {
    let mut handles = Vec::with_capacity(targets.len());

    for target in targets {
        let metrics_clone = facet_metrics.clone();
        let mut shutdown_clone = shutdown.clone();

        let handle = tokio::spawn(async move {
            run_sampler_for_target(target, interval, metrics_clone, &mut shutdown_clone).await;
        });

        handles.push(handle);
    }

    handles
}

async fn run_sampler_for_target(
    target: NodeMetricsTarget,
    interval: Duration,
    facet_metrics: FacetMetrics,
    shutdown: &mut watch::Receiver<bool>,
) {
    let client = reqwest::Client::new();

    debug!(
        node_id = %target.node_id,
        url = %target.metrics_url,
        "starting facet metrics sampler for node",
    );

    // Seed at least one sample as soon as possible.
    if let Err(err) = sample_once(&client, &target, &facet_metrics).await {
        warn!(
            node_id = %target.node_id,
            url = %target.metrics_url,
            error = ?err,
            "initial metrics sample failed (will retry on interval)",
        );
    }

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                // If the sender has been dropped, we treat that as a
                // shutdown signal as well.
                if changed.is_err() || *shutdown.borrow() {
                    debug!(
                        node_id = %target.node_id,
                        url = %target.metrics_url,
                        "facet metrics sampler shutting down",
                    );
                    break;
                }
            }
            _ = time::sleep(interval) => {
                if let Err(err) = sample_once(&client, &target, &facet_metrics).await {
                    // We do not fail the sampler permanently on transient
                    // errors; they show up as gaps / stale data in the UI.
                    warn!(
                        node_id = %target.node_id,
                        url = %target.metrics_url,
                        error = ?err,
                        "facet metrics sample failed",
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)] // Fields/variants are only observed via Debug logging in the current slice.
enum SamplerError {
    Http(reqwest::Error),
    Parse(String),
}

impl From<reqwest::Error> for SamplerError {
    fn from(err: reqwest::Error) -> Self {
        SamplerError::Http(err)
    }
}

async fn sample_once(
    client: &reqwest::Client,
    target: &NodeMetricsTarget,
    facet_metrics: &FacetMetrics,
) -> Result<(), SamplerError> {
    let mut request = client.get(&target.metrics_url);

    if let Some(timeout) = target.timeout {
        request = request.timeout(timeout);
    }

    let response = request.send().await?.error_for_status()?;
    let body = response.text().await?;

    let snapshots = parse_facet_snapshots(&body)?;
    if !snapshots.is_empty() {
        facet_metrics.update_from_scrape(&target.node_id, snapshots);
    }

    Ok(())
}

/// Parse facet counters from a Prometheus text exposition.
///
/// We currently look for lines of the form:
///
/// ```text
/// ron_facet_requests_total{facet="overlay.connect",result="ok"} 123
/// ron_facet_requests_total{facet="overlay.connect",result="error"} 4
/// ron_facet_requests_total{facet="overlay.jobs",result="ok"} 42
/// ```
///
/// For each `facet`, we aggregate all matching series into:
/// - `requests_total` = sum(all results)
/// - `errors_total`   = sum(result in {"error","err","failure","5xx"})
fn parse_facet_snapshots(body: &str) -> Result<Vec<FacetSnapshot>, SamplerError> {
    let mut counters: HashMap<String, (f64, f64)> = HashMap::new();

    for raw_line in body.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // We only care about the facet requests counter for now.
        if !line.starts_with("ron_facet_requests_total") {
            continue;
        }

        let (metric_and_labels, value_str) = match line.split_once(' ') {
            Some((left, right)) => (left.trim(), right.trim()),
            None => continue,
        };

        let value: f64 = match value_str.parse() {
            Ok(v) => v,
            Err(_) => {
                // Degrade on parse errors; don't abort the whole scrape.
                continue;
            }
        };

        // Split `name{labels}` into `name` and `labels`.
        let labels_str = match metric_and_labels.find('{') {
            Some(start) => {
                let end = match metric_and_labels.rfind('}') {
                    Some(e) if e > start => e,
                    _ => continue,
                };
                &metric_and_labels[start + 1..end]
            }
            None => "",
        };

        let mut facet: Option<String> = None;
        let mut result: Option<String> = None;

        for pair in labels_str.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            let (key, raw_val) = match pair.split_once('=') {
                Some(kv) => kv,
                None => continue,
            };

            // Values are quoted: key="value"
            let val = raw_val.trim().trim_matches('"');

            match key {
                "facet" => facet = Some(val.to_owned()),
                "result" | "outcome" => result = Some(val.to_owned()),
                _ => {}
            }
        }

        let facet = match facet {
            Some(f) => f,
            None => continue,
        };

        let entry = counters.entry(facet).or_insert((0.0_f64, 0.0_f64));

        // Any series contributes to total requests; some are also
        // interpreted as errors.
        entry.0 += value;

        let is_error = match result.as_deref() {
            Some("error") | Some("err") | Some("failure") | Some("5xx") => true,
            _ => false,
        };

        if is_error {
            entry.1 += value;
        }
    }

    let snapshots = counters
        .into_iter()
        .map(|(facet, (requests_total, errors_total))| FacetSnapshot {
            facet,
            requests_total,
            errors_total,
        })
        .collect();

    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::parse_facet_snapshots;

    #[test]
    fn parse_facet_snapshots_aggregates_by_facet() {
        let body = r#"
# HELP ron_facet_requests_total Total facet requests
# TYPE ron_facet_requests_total counter
ron_facet_requests_total{facet="overlay.connect",result="ok"} 100
ron_facet_requests_total{facet="overlay.connect",result="error"} 4
ron_facet_requests_total{facet="overlay.jobs",result="ok"} 42
"#;

        let snapshots = parse_facet_snapshots(body).expect("parse should succeed");

        assert_eq!(snapshots.len(), 2);

        let mut overlay_connect = None;
        let mut overlay_jobs = None;

        for snap in snapshots {
            match snap.facet.as_str() {
                "overlay.connect" => overlay_connect = Some(snap),
                "overlay.jobs" => overlay_jobs = Some(snap),
                _ => {}
            }
        }

        let overlay_connect = overlay_connect.expect("overlay.connect present");
        assert!((overlay_connect.requests_total - 104.0).abs() < f64::EPSILON);
        assert!((overlay_connect.errors_total - 4.0).abs() < f64::EPSILON);

        let overlay_jobs = overlay_jobs.expect("overlay.jobs present");
        assert!((overlay_jobs.requests_total - 42.0).abs() < f64::EPSILON);
        assert!((overlay_jobs.errors_total - 0.0).abs() < f64::EPSILON);
    }
}
