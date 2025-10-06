

---

````markdown
---
title: Invariant-Driven Blueprint — ron-metrics
version: 1.0.1
status: FINAL (canon-aligned)
last-updated: 2025-10-05
audience: contributors, reviewers, ops, auditors
pillar: 5 (Observability)
concerns: [PERF, RES, GOV]
crate-type: lib
msrv: 1.80.0
---

# 🪓 ron-metrics — Invariant-Driven Blueprint (IDB)

*A constitution for observability in RustyOnions.*

---

## 1. Invariants (MUST)

- **[I-1] Golden metrics everywhere.**  
  Every runtime service exposes `/metrics`, `/healthz`, and `/readyz` with Prometheus-compatible output. Minimum golden set (names are canonical):  
  - `bus_lagged_total`  
  - `service_restarts_total`  
  - `request_latency_seconds` (histogram)  
  - `rejected_total{reason}` (if HTTP ingress/edge)  
  - `health_ready{service,ok}` (may be derived from HealthState snapshot)

- **[I-2] HealthState truthfulness.**  
  `HealthState::snapshot()` reflects **only** durable readiness; `ready=true` ⇔ config + bus + all required deps are up. Degraded readiness must surface reasons:  
  `{"degraded":true,"missing":[...],"retry_after":<s>}`.

- **[I-3] Single registration rule.**  
  All metric families are **registered once** per process (e.g., via `OnceCell`/`OnceLock`). Callers clone handles; they **do not** re-register. Duplicate names are a CI failure.

- **[I-4] Readiness degrades before collapse.**  
  `/readyz` transitions to `503` **before** saturation or restart storms. Policy: fail-open reads, fail-closed writes.

- **[I-5] Amnesia awareness.**  
  All first-party metrics include the label `amnesia="on|off"`. When amnesia is ON (Micronode default) the observability layer performs **zero disk writes**.

- **[I-6] TLS type invariant.**  
  Any TLS configuration referenced by metrics/health surfaces **must** be `tokio_rustls::rustls::ServerConfig` (never `rustls::ServerConfig`), matching kernel/transport canon.

- **[I-7] Crash-only observability.**  
  Supervised restarts **increment** `service_restarts_total{service,...}`; bus overflow/lag increments `bus_lagged_total{service,...}`. No silent resets.

- **[I-8] PQ & ZK observability (forward-proofing).**  
  When PQ hybrids or ZK proofs are enabled anywhere in the node, `ron-metrics` MUST expose:  
  - `pq_kex_failures_total{algo,role}`  
  - `pq_verify_failures_total{algo,role}`  
  - `zk_proof_latency_seconds{scheme}` (histogram)  
  - `zk_verify_failures_total{scheme}`  
  These are **N/A** until features are enabled; families still exist (zeroed) to keep dashboards stable.

- **[I-9] Canonical taxonomy & labels.**  
  First-party metrics include labels: `service`, `instance`, `amnesia`, `build_version`. Units/suffixes must follow: `*_seconds`, `*_bytes`, `*_total`. Divergence is a release blocker.

---

## 2. Design Principles (SHOULD)

- **[P-1] Observable by default.** Telemetry is a feature, not an add-on.
- **[P-2] Cheap hot path.** Keep counters atomic; avoid locks/allocations; pre-register buckets.
- **[P-3] Uniform names.** Names/labels are repo-wide consistent and machine-checkable (regex gates).
- **[P-4] Readiness policy.** Reads are fail-open (degrade gracefully), writes are fail-closed with `Retry-After`.
- **[P-5] Tests prove invariants.** Metrics exist to *prove* behavior in unit/integration/chaos tests.
- **[P-6] Trace correlation.** Prefer exemplars/attributes so `request_latency_seconds` can link to trace IDs without exploding cardinality.
- **[P-7] Exporter portability.** Prometheus is the default; keep an **optional `otel` feature** for OpenTelemetry without changing the public API.

---

## 3. Implementation (HOW)

### 3.1 Public surface (stable)

```rust
// Public type shape (stable expectation in this crate)
pub struct Metrics {
    pub request_latency_seconds: prometheus::Histogram,
    pub bus_lagged_total: prometheus::IntCounterVec,
    pub service_restarts_total: prometheus::IntCounterVec,

    // PQ / ZK families are present even if unused to keep dashboards stable
    pub pq_kex_failures_total: prometheus::IntCounterVec,
    pub pq_verify_failures_total: prometheus::IntCounterVec,
    pub zk_verify_failures_total: prometheus::IntCounterVec,
    pub zk_proof_latency_seconds: prometheus::Histogram,
}

impl Metrics {
    /// Construct and register metric families once.
    pub fn new() -> std::sync::Arc<Self> { /* see full snippet below */ }

    /// Serve /metrics, /healthz, /readyz on the given addr.
    pub async fn serve(
        self: std::sync::Arc<Self>,
        addr: std::net::SocketAddr,
        health: std::sync::Arc<HealthState>,
    ) -> anyhow::Result<(tokio::task::JoinHandle<()>, std::net::SocketAddr)> { /* ... */ }
}
````

### 3.2 Reference implementation (imports included, copy-paste-ready)

```rust
use std::{net::SocketAddr, sync::Arc};
use axum::{routing::get, response::IntoResponse, http::StatusCode, Json, Router};
use once_cell::sync::OnceCell;
use prometheus::{
    Encoder, TextEncoder, Histogram, HistogramOpts, IntCounterVec, Opts, Registry,
    register_histogram_with_registry, register_int_counter_vec_with_registry, gather,
};
use tokio::{net::TcpListener, task::JoinHandle};

// Placeholder; use the real HealthState from ron-kernel in code.
pub struct HealthState;
impl HealthState {
    pub fn all_ready(&self) -> bool { false }
    pub fn snapshot(&self) -> serde_json::Value { serde_json::json!({ "degraded": true }) }
}

pub struct Metrics {
    pub registry: Registry,

    pub request_latency_seconds: Histogram,
    pub bus_lagged_total: IntCounterVec,
    pub service_restarts_total: IntCounterVec,

    pub pq_kex_failures_total: IntCounterVec,
    pub pq_verify_failures_total: IntCounterVec,
    pub zk_verify_failures_total: IntCounterVec,
    pub zk_proof_latency_seconds: Histogram,
}

static METRICS: OnceCell<Arc<Metrics>> = OnceCell::new();

impl Metrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();

        let request_latency_seconds = register_histogram_with_registry!(
            HistogramOpts::new("request_latency_seconds", "End-to-end request latency (seconds)")
                .buckets(vec![0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0]),
            registry
        ).expect("register request_latency_seconds");

        let labels = &["service","instance","amnesia","build_version"];

        let bus_lagged_total = register_int_counter_vec_with_registry!(
            Opts::new("bus_lagged_total","Broadcast bus lag/drop events observed"),
            labels,
            registry
        ).expect("register bus_lagged_total");

        let service_restarts_total = register_int_counter_vec_with_registry!(
            Opts::new("service_restarts_total","Supervisor restarts by service"),
            labels,
            registry
        ).expect("register service_restarts_total");

        // PQ / ZK: present even if features not used, to keep dashboards stable
        let pq_kex_failures_total = register_int_counter_vec_with_registry!(
            Opts::new("pq_kex_failures_total","PQ hybrid KEX failures"),
            &["algo","role","service","instance","amnesia","build_version"],
            registry
        ).expect("register pq_kex_failures_total");

        let pq_verify_failures_total = register_int_counter_vec_with_registry!(
            Opts::new("pq_verify_failures_total","PQ signature verify failures"),
            &["algo","role","service","instance","amnesia","build_version"],
            registry
        ).expect("register pq_verify_failures_total");

        let zk_verify_failures_total = register_int_counter_vec_with_registry!(
            Opts::new("zk_verify_failures_total","ZK proof verification failures"),
            &["scheme","service","instance","amnesia","build_version"],
            registry
        ).expect("register zk_verify_failures_total");

        let zk_proof_latency_seconds = register_histogram_with_registry!(
            HistogramOpts::new("zk_proof_latency_seconds","ZK proof generation latency (seconds)")
                .buckets(vec![0.01,0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0]),
            registry
        ).expect("register zk_proof_latency_seconds");

        Arc::new(Self {
            registry,
            request_latency_seconds,
            bus_lagged_total,
            service_restarts_total,
            pq_kex_failures_total,
            pq_verify_failures_total,
            zk_verify_failures_total,
            zk_proof_latency_seconds,
        })
    }

    pub fn global() -> Arc<Self> {
        METRICS.get_or_init(Self::new).clone()
    }

    pub async fn serve(
        self: Arc<Self>,
        addr: SocketAddr,
        health: Arc<HealthState>,
    ) -> anyhow::Result<(JoinHandle<()>, SocketAddr)> {
        let listener = TcpListener::bind(addr).await?;
        let local = listener.local_addr()?;

        let app = Router::new()
            .route("/metrics", get({
                move || async move {
                    let families = gather();
                    let mut buf = Vec::with_capacity(16 * 1024);
                    TextEncoder::new().encode(&families, &mut buf).unwrap();
                    (StatusCode::OK, buf)
                }
            }))
            .route("/healthz", get({
                let h = health.clone();
                move || async move {
                    if h.all_ready() { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE }
                }
            }))
            .route("/readyz", get({
                let h = health.clone();
                move || async move {
                    if h.all_ready() {
                        (StatusCode::OK, "ready").into_response()
                    } else {
                        (StatusCode::SERVICE_UNAVAILABLE, Json(h.snapshot())).into_response()
                    }
                }
            }));

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Ok((handle, local))
    }
}
```

> **Future-proof exporter seam:** Put an `otel` Cargo feature behind a tiny adapter that implements the same `Metrics` surface with OTEL meters; keep Prometheus as default. The public API does **not** change.

---

## 4. Acceptance Gates (PROOF)

| Gate       | Description                                                                   | Verification (examples)                                                                                                                                                                     |                       |            |
| :--------- | :---------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------- | ---------- |
| **[G-1]**  | Unit: single registration & exposition.                                       | `cargo test -p ron-metrics` (registry cleared per test run).                                                                                                                                |                       |            |
| **[G-2]**  | Integration: `/metrics`, `/healthz`, `/readyz` exist and respond.             | `curl -fsS :9600/metrics                                                                                                                                                                    | rg bus_lagged_total`. |            |
| **[G-3]**  | Chaos restart increments restarts; `/readyz` flips to 503 and recovers ≤ 30s. | `testing/chaos_restart.sh` + scrape asserts.                                                                                                                                                |                       |            |
| **[G-4]**  | Perf SLO: metrics endpoint p95 < 10 ms local.                                 | Criterion bench “exposition_latency”.                                                                                                                                                       |                       |            |
| **[G-5]**  | Taxonomy & labels audit.                                                      | CI script greps for required labels/suffixes (`service`, `amnesia`; `*_seconds                                                                                                              | *_bytes               | *_total`). |
| **[G-6]**  | Amnesia toggle reflected in labels.                                           | Run Micronode profile with amnesia=on; grep label presence.                                                                                                                                 |                       |            |
| **[G-7]**  | Sanitizers/Miri clean.                                                        | Mandatory TSan workflow + optional Miri on logic-heavy targets.                                                                                                                             |                       |            |
| **[G-8]**  | Formal: health DAG TLA+ checked per release.                                  | `specs/health_dag.tla` CI job prints spec SHA.                                                                                                                                              |                       |            |
| **[G-9]**  | Facet SLO coverage present where applicable.                                  | If a service implements Graph/Search/Media, ensure histograms exist: `graph_neighbors_latency_seconds`, `search_query_latency_seconds`, `media_range_start_latency_seconds` (N/A for libs). |                       |            |
| **[G-10]** | PQ/ZK stubs wired.                                                            | With `--features pq` or `--features zk`, `/metrics` includes the PQ/ZK families and they encode cleanly (zero counts allowed).                                                              |                       |            |

**CI snippet (taxonomy/labels sanity)**

```bash
OUT="$(curl -fsS localhost:9600/metrics)"
echo "$OUT" | rg -q 'request_latency_seconds' || { echo "missing request_latency_seconds"; exit 1; }
for lbl in service instance amnesia build_version; do
  echo "$OUT" | rg -q "${lbl}=\"" || { echo "missing label: $lbl"; exit 1; }
done
echo "$OUT" | rg -q '(_seconds|_bytes|_total)\b' || { echo "suffix taxonomy violation"; exit 1; }
```

---

## 5. Anti-Scope (Forbidden)

* ❌ Re-registering metric families (per-request or per-handler).
* ❌ Locks across `.await` in handlers or exporters.
* ❌ `static mut` or ad-hoc singletons (use `OnceCell` / `OnceLock`).
* ❌ SHA-256/MD5 anywhere in telemetry paths (use **BLAKE3** if hashing is required).
* ❌ Blocking I/O in `/metrics` path (keep it non-blocking).
* ❌ Shipping with PQ/ZK features enabled but **no** corresponding metrics families.
* ❌ Divergent names/suffixes (e.g., `latency_ms` → must be `_seconds`).

---

## 6. References

* **Full Project Blueprint v2.0** — Observability contract & kernel invariants.
* **Concurrency & Aliasing Blueprint v1.3** — register-once rule; no locks across `.await`.
* **Hardening Blueprint v2.0** — self-test, limits, taxonomy expectations.
* **Microkernel Blueprint (final)** — frozen kernel exports & golden metrics.
* **Scaling Blueprint v1.4** — SLOs + dashboarding; readiness degrade policy.
* **Six Concerns Spine** — `ron-metrics` maps to **PERF/RES/GOV**.

---

## 7. Definition of Done (ron-metrics — Pillar 5)

* [ ] `/metrics`, `/healthz`, `/readyz` implemented, p95 < 10 ms locally.
* [ ] `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds` present.
* [ ] Registration is single-pass; no duplicate families.
* [ ] Amnesia label present and accurate.
* [ ] PQ/ZK families exist and encode cleanly (even if zero).
* [ ] TSan green; (optional) Miri green.
* [ ] Health DAG spec checked; CI gates for taxonomy/labels pass.
* [ ] Docs/README cross-link Pillar 5 & Six Concerns.

---

```
```
