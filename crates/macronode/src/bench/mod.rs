// crates/macronode/src/bench/mod.rs
//
// RO:WHAT — Node-executed benchmarking engine (bounded, safe loadgen).
// RO:WHY  — Provide “real world” measurements from inside the node environment
//           (stable timing, no browser jitter, no remote clock skew).
// RO:INVARIANTS —
//   - Bounded duration, bounded concurrency, bounded memory.
//   - No locks held across .await.
//   - Only targets curated endpoints (no arbitrary URL cannon).
// RO:SECURITY —
//   - Server-enforced limits prevent accidental overload.
//   - Intended to be auth-gated at the HTTP layer.

#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use rand::{rngs::StdRng, Rng, SeedableRng};
use reqwest::Client;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunReq {
    /// Suite name. v1 supports: "admin_plane".
    pub suite: String,
    /// Total runtime (seconds). Hard bounded by server.
    pub duration_secs: u64,
    /// Worker count. Hard bounded by server.
    pub concurrency: u32,
    /// Placeholder for future suites (kept for forward-compat).
    #[serde(default)]
    pub payload_size: u64,
    /// Determinism seed (optional). If 0, the node picks a seed.
    #[serde(default)]
    pub seed: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunResp {
    pub run_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunStatusDto {
    pub run_id: String,
    /// queued | running | done | failed
    pub status: String,
    /// 0.0..1.0
    pub progress: f32,
    pub phase: String,
    pub started_at: Option<String>,
    pub updated_at: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchEndpointResultDto {
    pub name: String,
    pub method: String,
    pub path: String,

    pub requests: u64,
    pub errors: u64,
    pub rps: f64,

    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunResultDto {
    pub run_id: String,
    pub suite: String,
    pub started_at: String,
    pub ended_at: String,
    pub results: Vec<BenchEndpointResultDto>,
    pub notes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("invalid suite: {0}")]
    InvalidSuite(String),
    #[error("bench run rejected: {0}")]
    Rejected(String),
    #[error("internal bench failure: {0}")]
    Internal(String),
}

struct RunRecord {
    status: BenchRunStatusDto,
    result: Option<BenchRunResultDto>,
}

pub struct BenchManager {
    base_url: String,
    http: Client,
    // IMPORTANT: must be Arc-backed so spawned tasks update the *same* run map.
    runs: Arc<RwLock<HashMap<String, RunRecord>>>,

    // Hard safety limits
    max_duration: Duration,
    max_concurrency: u32,
    max_samples_per_endpoint: usize,
    max_active_runs: usize,
}

impl BenchManager {
    pub fn new(base_url: String) -> Self {
        let http = Client::builder()
            .user_agent("macronode-bench/0.1.0")
            .build()
            .expect("reqwest client build must not fail");

        Self {
            base_url,
            http,
            runs: Arc::new(RwLock::new(HashMap::new())),
            max_duration: Duration::from_secs(120),
            max_concurrency: 64,
            max_samples_per_endpoint: 50_000,
            max_active_runs: 1,
        }
    }

    pub async fn start(&self, req: BenchRunReq) -> Result<BenchRunResp, BenchError> {
        let suite = req.suite.trim().to_string();
        if suite != "admin_plane" {
            return Err(BenchError::InvalidSuite(suite));
        }

        let duration = Duration::from_secs(req.duration_secs.clamp(1, self.max_duration.as_secs()));
        let concurrency = req.concurrency.clamp(1, self.max_concurrency);
        let payload_size = req.payload_size; // reserved (used for notes / forward-compat)

        // enforce active run cap
        let active = {
            let runs = self.runs.read().await;
            runs.values()
                .filter(|r| r.status.status == "queued" || r.status.status == "running")
                .count()
        };
        if active >= self.max_active_runs {
            return Err(BenchError::Rejected(format!(
                "another benchmark is already active (max_active_runs={})",
                self.max_active_runs
            )));
        }

        let run_id = Uuid::new_v4().to_string();
        let now_iso = iso_now();

        let record = RunRecord {
            status: BenchRunStatusDto {
                run_id: run_id.clone(),
                status: "queued".to_string(),
                progress: 0.0,
                phase: "queued".to_string(),
                started_at: None,
                updated_at: now_iso,
                error: None,
            },
            result: None,
        };

        {
            let mut runs = self.runs.write().await;
            runs.insert(run_id.clone(), record);
        }

        // Spawn the run. No locks held across awaits inside the workload.
        // NOTE: clone shares the same runs map (Arc<RwLock<...>>).
        let mgr = Arc::new(self.clone());
        let run_id_spawn = run_id.clone();
        tokio::spawn(async move {
            mgr.run_admin_plane_suite(run_id_spawn, duration, concurrency, req.seed, payload_size)
                .await;
        });

        Ok(BenchRunResp { run_id })
    }

    pub async fn status(&self, run_id: &str) -> Option<BenchRunStatusDto> {
        let runs = self.runs.read().await;
        runs.get(run_id).map(|r| r.status.clone())
    }

    pub async fn result(&self, run_id: &str) -> Option<BenchRunResultDto> {
        let runs = self.runs.read().await;
        runs.get(run_id).and_then(|r| r.result.clone())
    }

    async fn run_admin_plane_suite(
        self: Arc<Self>,
        run_id: String,
        duration: Duration,
        concurrency: u32,
        seed: u64,
        payload_size: u64,
    ) {
        let started_at = iso_now();
        self.update_status(&run_id, |s| {
            s.status = "running".to_string();
            s.phase = "warming".to_string();
            s.progress = 0.05;
            s.started_at = Some(started_at.clone());
            s.updated_at = iso_now();
        })
        .await;

        // Curated endpoint set (real control-plane “paths people hit”).
        let endpoints: Vec<EndpointSpec> = vec![
            EndpointSpec::get("healthz", "/healthz"),
            EndpointSpec::get("readyz", "/readyz"),
            EndpointSpec::get("status", "/api/v1/status"),
            EndpointSpec::get("system_summary", "/api/v1/system/summary"),
            EndpointSpec::get("storage_summary", "/api/v1/storage/summary"),
        ];

        // Small warmup (reduces first-hit jitter).
        let warm_deadline = Instant::now() + Duration::from_millis(500);
        let _ = self
            .worker_loop(
                &endpoints,
                warm_deadline,
                1,
                1,
                /*store_samples*/ false,
            )
            .await;

        self.update_status(&run_id, |s| {
            s.phase = "running".to_string();
            s.progress = 0.10;
            s.updated_at = iso_now();
        })
        .await;

        let deadline = Instant::now() + duration;
        let base_seed: u64 = if seed == 0 { rand::random() } else { seed };

        // Spawn workers and join.
        let mut joins = Vec::with_capacity(concurrency as usize);
        for w in 0..concurrency {
            let mgr = self.clone();
            let eps = endpoints.clone();
            let run_deadline = deadline;
            let wseed = base_seed ^ (w as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);

            joins.push(tokio::spawn(async move {
                mgr.worker_loop(
                    &eps,
                    run_deadline,
                    w as u32,
                    wseed,
                    /*store_samples*/ true,
                )
                .await
            }));
        }

        // Aggregate.
        let mut agg: Vec<Stats> = endpoints.iter().map(|_| Stats::new()).collect();
        let mut total_errs: u64 = 0;

        // If any worker task panics/cancels, fail the run (internal failure).
        let mut internal_failure: Option<String> = None;

        for j in joins {
            match j.await {
                Ok(worker_stats) => {
                    for (i, ws) in worker_stats.into_iter().enumerate() {
                        total_errs = total_errs.saturating_add(ws.errors);
                        agg[i].merge(ws, self.max_samples_per_endpoint);
                    }
                }
                Err(e) => {
                    total_errs = total_errs.saturating_add(1);
                    let be = BenchError::Internal(format!("worker_join_failed: {e}"));
                    internal_failure.get_or_insert_with(|| be.to_string());
                }
            }
        }

        if let Some(err) = internal_failure {
            self.fail_run(&run_id, err).await;
            return;
        }

        self.update_status(&run_id, |s| {
            s.progress = 0.95;
            s.updated_at = iso_now();
        })
        .await;

        let ended_at = iso_now();
        let mut results = Vec::with_capacity(endpoints.len());

        for (i, ep) in endpoints.iter().enumerate() {
            let stats = &mut agg[i];
            results.push(stats.to_result(ep, duration));
        }

        let mut notes = vec![
            "Suite=admin_plane targets curated admin endpoints (healthz/readyz/status/system/storage)."
                .to_string(),
            "Numbers include full HTTP roundtrip to the node admin listener (loopback if run locally)."
                .to_string(),
            "This is a v1 harness; later suites will add storage PUT/GET, overlay streaming, DHT ops."
                .to_string(),
        ];

        // Forward-compat breadcrumb: payload_size is reserved for future suites.
        if payload_size != 0 {
            notes.push(format!(
                "payloadSize={} (reserved for future suites; ignored by admin_plane).",
                payload_size
            ));
        }

        if total_errs > 0 {
            notes.push(format!(
                "Observed {total_errs} errors (see per-endpoint errors)."
            ));
        }

        let final_result = BenchRunResultDto {
            run_id: run_id.clone(),
            suite: "admin_plane".to_string(),
            started_at,
            ended_at: ended_at.clone(),
            results,
            notes,
        };

        self.finish_run(&run_id, final_result, ended_at).await;
    }

    async fn worker_loop(
        &self,
        endpoints: &[EndpointSpec],
        deadline: Instant,
        worker_id: u32,
        seed: u64,
        store_samples: bool,
    ) -> Vec<Stats> {
        let mut rng: StdRng = StdRng::seed_from_u64(seed);
        let mut stats: Vec<Stats> = endpoints.iter().map(|_| Stats::new()).collect();

        while Instant::now() < deadline {
            // rand 0.9: prefer random_range/random_bool (no SliceRandom/choose needed)
            let pick = rng.random_range(0..endpoints.len());
            let ep = &endpoints[pick];

            let url = format!("{}{}", self.base_url, ep.path);

            let t0 = Instant::now();
            let ok = match ep.method.as_str() {
                "GET" => self
                    .http
                    .get(&url)
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false),
                _ => false,
            };
            let dur = t0.elapsed();

            let s = &mut stats[pick];
            s.record(dur, ok, store_samples, &mut rng);
        }

        let _ = worker_id; // reserved for future per-worker debug

        stats
    }

    async fn update_status<F: FnOnce(&mut BenchRunStatusDto)>(&self, run_id: &str, f: F) {
        let mut runs = self.runs.write().await;
        if let Some(r) = runs.get_mut(run_id) {
            f(&mut r.status);
        }
    }

    async fn finish_run(&self, run_id: &str, result: BenchRunResultDto, ended_at_iso: String) {
        let mut runs = self.runs.write().await;
        if let Some(r) = runs.get_mut(run_id) {
            r.result = Some(result);
            r.status.status = "done".to_string();
            r.status.phase = "done".to_string();
            r.status.progress = 1.0;
            r.status.updated_at = ended_at_iso;
        }
    }

    async fn fail_run(&self, run_id: &str, err: String) {
        let ended_at_iso = iso_now();
        let mut runs = self.runs.write().await;
        if let Some(r) = runs.get_mut(run_id) {
            r.result = None;
            r.status.status = "failed".to_string();
            r.status.phase = "failed".to_string();
            r.status.progress = 1.0;
            r.status.updated_at = ended_at_iso;
            r.status.error = Some(err);
        }
    }
}

impl Clone for BenchManager {
    fn clone(&self) -> Self {
        Self {
            base_url: self.base_url.clone(),
            http: self.http.clone(),
            runs: self.runs.clone(),
            max_duration: self.max_duration,
            max_concurrency: self.max_concurrency,
            max_samples_per_endpoint: self.max_samples_per_endpoint,
            max_active_runs: self.max_active_runs,
        }
    }
}

#[derive(Clone)]
struct EndpointSpec {
    name: String,
    method: String,
    path: String,
}
impl EndpointSpec {
    fn get(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            method: "GET".to_string(),
            path: path.to_string(),
        }
    }
}

// Stats collected per endpoint.
#[derive(Clone)]
struct Stats {
    requests: u64,
    errors: u64,
    samples_ms: Vec<f64>,
}
impl Stats {
    fn new() -> Self {
        Self {
            requests: 0,
            errors: 0,
            samples_ms: Vec::new(),
        }
    }

    fn record<R: Rng>(&mut self, dur: Duration, ok: bool, store_samples: bool, rng: &mut R) {
        self.requests = self.requests.saturating_add(1);
        if !ok {
            self.errors = self.errors.saturating_add(1);
        }

        if !store_samples {
            return;
        }

        // Bounded memory: keep at most N samples, with a cheap random drop strategy.
        let ms = dur.as_secs_f64() * 1000.0;

        if self.samples_ms.len() < 10_000 {
            self.samples_ms.push(ms);
            return;
        }

        // Once we’re “full”, occasionally replace a random slot.
        if rng.random_bool(0.05) {
            let idx = rng.random_range(0..self.samples_ms.len());
            self.samples_ms[idx] = ms;
        }
    }

    fn merge(&mut self, other: Stats, max_samples: usize) {
        self.requests = self.requests.saturating_add(other.requests);
        self.errors = self.errors.saturating_add(other.errors);

        if self.samples_ms.len() < max_samples {
            self.samples_ms.extend(other.samples_ms);
            if self.samples_ms.len() > max_samples {
                self.samples_ms.truncate(max_samples);
            }
        }
    }

    fn to_result(&mut self, ep: &EndpointSpec, duration: Duration) -> BenchEndpointResultDto {
        // compute quantiles from samples (best-effort)
        self.samples_ms
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50 = quantile(&self.samples_ms, 0.50);
        let p95 = quantile(&self.samples_ms, 0.95);
        let p99 = quantile(&self.samples_ms, 0.99);

        let rps = if duration.as_secs_f64() > 0.0 {
            (self.requests as f64) / duration.as_secs_f64()
        } else {
            0.0
        };

        BenchEndpointResultDto {
            name: ep.name.clone(),
            method: ep.method.clone(),
            path: ep.path.clone(),
            requests: self.requests,
            errors: self.errors,
            rps,
            p50_ms: p50,
            p95_ms: p95,
            p99_ms: p99,
        }
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let q = q.clamp(0.0, 1.0);
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn iso_now() -> String {
    // Avoid extra chrono dependency. This is “good enough” ISO-ish for logs/UI.
    // svc-admin already treats timestamps as display strings.
    let now = std::time::SystemTime::now();
    let dur = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    format!("unix:{}.{:03}", dur.as_secs(), dur.subsec_millis())
}
