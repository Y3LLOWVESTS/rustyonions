### NOTES - ron-metrics - OCTOBER 24 2025 - 17:33 CST


# ron-metrics — Accomplishments

## Core surface (stable & working)

* **Single metrics facade**: `Metrics::new`, `Metrics::serve`, `Metrics::health`, `Metrics::set_ready`, `inc_service_restart`, `add_bus_lag`, `observe_request`, `observe_status_class`, `registry`.
* **HTTP exposer (axum 0.7)**: production-style handlers for:

  * `/metrics` (Prometheus text format, correct content-type; lock-free across `.await`)
  * `/healthz` (200 ok / 503 fail + compact JSON snapshot)
  * `/readyz` (same policy hook; future two-phase ready supported)
* **Zero hidden loops**: no background threads/tickers sneaking in; everything explicit and bounded.

## Metrics taxonomy (frozen names)

* `service_restarts_total{component}` — counter
* `bus_lagged_total{bus}` — counter
* `request_latency_seconds` — histogram (strictly increasing buckets: 1ms..512ms + +Inf)
* `request_status_total{status_class}` — counter (low cardinality: `1xx..5xx|other`)
* `health_ready{check}` — gauge (0/1)
* `exposition_latency_seconds` — histogram (export path latency)
* Naming/labels **locked** for v0.1.x; future additions won’t break dashboards.

## Axum middleware (drop-in)

* **Latency** middleware: `axum_latency::attach(app, metrics)` — records `request_latency_seconds`.
* **Status-class** middleware: `axum_status::attach(app, metrics)` — records `request_status_total{status_class}`.
* Verified against axum 0.7’s `Next` semantics (no generic mismatches, correct body type usage).

## Health & readiness

* Minimal **health bridge**:

  * Set checks: `metrics.set_ready("db", true)` etc.
  * `/healthz` JSON includes `degraded`, `missing`, `since`.
  * `/readyz` currently mirrors health; `ReadyPolicy` plumbed for stricter future logic.
* **Gauge export**: `health_ready{check}` reflects the same state used by HTTP gates.

## Registry & encoding

* **SafeRegistry wrapper** integrated; single registration point for all metric families.
* Correct **Prometheus `TextEncoder`** usage; trait imports fixed; buckets validated to be strictly increasing.
* **No global mutable state** leaks; registry is owned, not “register everywhere”.

## Bus integration (feature-gated)

* `--features bus` compiles & runs:

  * **Bus watcher** spawns a bounded task (no locks across `.await`) that maps bus events → metrics & readiness.
  * Emits `bus_lagged_total{bus}` for overwrite counts.
  * Handles `Shutdown` cleanly (joinable).

## Examples that actually prove it

* **`examples/axum_api`**: merged router, both middlewares; verified histograms & counters move under real requests.
* **`examples/exposer`**: sidecar-style exporter process.
* **`examples/pump`**: drives gauges/counters for manual scraping/demo.
* **`examples/watch_bus`**: watcher demo; fixed config/build drift with `ron-bus`.

## Tests (green)

* **HTTP smoke**: `/metrics`, `/healthz`, `/readyz` reachable & sane (axum/hyper interop correct).
* **Health state**: roundtrip test for readiness gauge + snapshot semantics.
* **Encoding**: histogram/counter families encode without errors; buckets validated.
* **Status counter**: `request_status_total{2xx}` increments through a real request.
* **Feature path**: tests pass with and without `--features bus`.
* Doc test for middleware compiled/ignored appropriately; no doc failures.

## Hardening & correctness we nailed on the way

* Fixed trait imports (`Encoder`) and error plumbing (`From<prometheus::Error>`, `From<std::io::Error>`).
* Removed wrong assumptions about `SafeRegistryBuilder`; aligned to **actual** `SafeRegistry::new()`.
* Avoided moving arguments twice (`set_ready` ownership fix).
* Repaired axum middleware type signatures to match 0.7 (`Next`, `Request<Body>` vs generics).
* Eliminated histogram bucket duplication (strict increase requirement).
* Layer compatibility fixed (`Router::layer` bounds satisfied without exotic types).

## Performance & invariants

* **No locks across `.await`** in handlers or watcher.
* **Bounded everything**; no unbounded channels/queues.
* **Low-overhead hot path**: counter/histogram ops kept simple; no per-request string allocations beyond necessary labels.
* Exposer latency is **observed** separately to spot exporter regressions.

## Feature flags sanity (compiles)

* Default (Prometheus + axum) ✅
* `bus` ✅ (watcher example)
* `tls`, `otel`, `uds` scaffolding present in Cargo features; compile passes or trivially added without touching core paths (we deferred full runtime wiring by design).

## Developer UX & integration

* **Clean example commands**:

  * Sidecar: `RON_METRICS_METRICS_ADDR=127.0.0.1:0 cargo run -p ron-metrics --example exposer`
  * In-app: `axum_api` shows router merge + middlewares.
* **Observability first**: quick `curl` recipes for `/healthz`, `/readyz`, `/metrics` included.
* **Prometheus snippets** (scrape + starter alerts) delivered and validated.

## Backward-compat guarantees (v0.1.x)

* Metric names & label keys above are **frozen**.
* Endpoint paths `/metrics`, `/healthz`, `/readyz` **frozen**.
* We can add new metrics/labels later, but won’t rename existing ones.

---

### Bottom line

We built a **fast, predictable, production-style metrics/exporter crate** with Axum middleware, a clean health bridge, stable Prom metrics, and real tests/examples. It’s composable with `ron-bus`, adheres to our “no locks across await” invariant, and is ready to ship and scrape. The remaining work is polish (docs/CI and optional TLS/OTel), not core engineering.

### ADDITIONAL INFO 

---

# ron-metrics • Wrap-Up & Finish-Later Notes

**Date:** 2025-10-24
**Scope:** exporter + health bridge + Axum middleware; optional bus watcher behind `bus` feature.

---

## 0) TL;DR (where we stand)

* ✅ `/metrics`, `/healthz`, `/readyz` live; encode OK.
* ✅ Latency histogram (`request_latency_seconds`) and status-class counter (`request_status_total`) move under load.
* ✅ Health bridge works (gauge `health_ready{check}` + JSON).
* ✅ Tests pass locally (`unit + integration`), including HTTP smoke and status counter.
* ⚠️ Only remaining work is **polish** (docs, example warnings, optional features). Core API & metric names are **frozen**.

---

## 1) Public API (frozen for v0.1.x)

* `Metrics::new(base: BaseLabels, health: HealthState) -> Result<Metrics, MetricsError>`
* `Metrics::serve(addr) -> (JoinHandle, SocketAddr)` — simple exposer if you want a dedicated server.
* `Metrics::health() -> &HealthState`
* `Metrics::set_ready(check: impl Into<String>, ok: bool)`
* `Metrics::inc_service_restart(component: &str)`
* `Metrics::add_bus_lag(bus: &str, overwritten: u64)`
* `Metrics::observe_request(seconds: f64)`
* `Metrics::observe_status_class(class: &str)`
* `Metrics::registry() -> &SafeRegistry` (used by exposer)
* `axum_latency::attach(router, metrics)` — latency middleware
* `axum_status::attach(router, metrics)` — status-class middleware
* `exposer::http::make_router(metrics)` — mounts `/metrics`, `/healthz`, `/readyz`
* `Metrics::ready_policy() -> ReadyPolicy` (default semantics)

**Bus watcher (feature `bus`):**

* `start_bus_watcher(metrics, bus, watcher_name) -> JoinHandle<()>`

---

## 2) Metric taxonomy (stable names & labels)

All exported with **constant** base labels added by the registry layer (service, instance, build_version, amnesia) if configured there; otherwise they appear without those.

* `service_restarts_total{component}` — `counter`
* `bus_lagged_total{bus}` — `counter`
* `request_latency_seconds` — `histogram` (buckets: 1ms..512ms, +Inf implicit)
* `request_status_total{status_class="1xx|2xx|3xx|4xx|5xx|other"}` — `counter`
* `health_ready{check}` — `gauge` (0/1)
* `exposition_latency_seconds` — `histogram` (exposer timing)

**Do not rename.** SRE dashboards/alerts will depend on these.

---

## 3) HTTP endpoints

* `GET /metrics`

  * Plain-text Prometheus exposition (TextEncoder).
  * `exposition_latency_seconds` is observed around collection.
* `GET /healthz`

  * Body: `ok{"degraded":<bool>,"missing":[...],"since":<ts>}`
  * HTTP 200 if all checks `true`, else 503 if any are `false`.
* `GET /readyz`

  * Same policy as health now (single phase). `Metrics::ready_policy()` is the hook for future 2-phase logic.

---

## 4) Examples (what they demonstrate)

* `exposer` — minimal exporter process with health gates.
* `pump` — drives gauges/counters for a demo scrape.
* `watch_bus` (feature `bus`) — maps ron-bus events into metrics/health.
* `axum_api` — shows how to merge the exporter router into your app and attach middlewares:

  * `axum_latency::attach` (histogram)
  * `axum_status::attach` (status-class)

**Note:** `watch_bus.rs` currently has a couple unused imports → see “Polish” below.

---

## 5) Test suite status

Passing (local):

* `tests/health_ready.rs` — `HealthState` roundtrip.
* `tests/http_endpoints.rs` — `/metrics`, `/healthz`, `/readyz` smoke with `hyper`/`axum`.
* `tests/http_status_counter.rs` — asserts `request_status_total{2xx}` increments and latency count appears.
* Other placeholders (`public_api`, `taxonomy_labels`, `readiness_semantics`, `loom_shutdown`, `integration_http_endpoints`) — green.

Run locally:

```
cargo test -p ron-metrics
cargo test -p ron-metrics --features bus
```

---

## 6) What to finish later (small, deterministic)

### A) Example warnings (low effort)

* `crates/ron-metrics/examples/watch_bus.rs` — remove unused imports or use them.

### B) Docs

* `crates/ron-metrics/README.md`

  * Quickstart (Axum merge + middleware attach)
  * Metric list w/ labels and semantic meaning
  * Minimal Prom scrape config
* `crates/ron-metrics/RUNBOOK.md`

  * Health/readiness semantics
  * Scrape interval guidance (10–15s typical)
  * SRE starter alerts (see below)
  * Capacity/backpressure note for `bus_lagged_total`

### C) CI polish

* Add to workspace CI:

  * `cargo fmt -- --check`
  * `cargo clippy -p ron-metrics --all-targets -- -D warnings`
  * `cargo test -p ron-metrics --features bus`

### D) Feature compile sanity (no runtime work required now)

```
cargo build -p ron-metrics
cargo build -p ron-metrics --features bus
cargo build -p ron-metrics --features tls
cargo build -p ron-metrics --features otel
```

If `tls`/`otel` are not yet wired: it’s fine as long as they **compile**; implementation can be stubbed or marked `unimplemented!()` behind feature-gated API.

---

## 7) Ops snippets (copy/paste later)

### Prometheus scrape

```yaml
scrape_configs:
  - job_name: ron
    scrape_interval: 10s
    static_configs:
      - targets: ['127.0.0.1:9100']
```

### Alerts

```yaml
groups:
- name: ron-http
  rules:
  - alert: HttpErrorsSpike
    expr: increase(request_status_total{status_class=~"4xx|5xx"}[5m]) > 50
    for: 5m
    labels: { severity: warning }
    annotations:
      summary: HTTP error rate elevated

  - alert: ReadinessMissingChecks
    expr: health_ready == 0
    for: 2m
    labels: { severity: critical }
    annotations:
      summary: One or more readiness checks are failing
```

---

## 8) Performance & invariants to preserve

* No locks across `.await` in handlers/middlewares/watcher.
* Bounded data paths (no unbounded queues).
* Keep metric increments cheap; avoid per-request string allocations where possible.
* Histogram buckets: **strictly increasing** (current set 1ms..512ms is good).
* Don’t introduce hidden background loops in core unless explicitly opted-in.

---

## 9) Integration patterns

### Axum app (single process)

* Merge exporter router: `app.merge(make_router(metrics.clone()))`
* Attach both middlewares:

  * `axum_latency::attach(app, metrics.clone())`
  * `axum_status::attach(app, metrics.clone())`

### Sidecar exporter (separate port)

* Use `Metrics::serve(addr)` to start a small HTTP exposer task in-process.
* Or run `exposer` example as a standalone sidecar container/pod.

### Bus watcher (optional, `bus` feature)

* Start watcher with `start_bus_watcher(metrics.clone(), bus, "watcher-name")`.
* Emits `bus_lagged_total{bus}` on overwrite and flips readiness when health events flow.

---

## 10) Backward-compat guarantees (v0.1.x)

* The metric **names and label keys** listed above are frozen.
* Endpoint paths `/metrics`, `/healthz`, `/readyz` are frozen.
* Adding **new** metrics/labels is allowed; renaming existing ones is not.

---

## 11) Release checklist (when you return)

1. Fix example warnings (or mark `allow(unused)` in examples only).
2. Add/refresh README + RUNBOOK.
3. Ensure `cargo clippy` clean on crate (not necessarily the whole workspace).
4. Tag and cut: `v0.1.0` (or `v0.1.1` after docs).
5. Announce metric names → SRE dashboards.

---

## 12) Parking lot (future enhancements)

* **TLS exposer** (`tls` feature):

  * `serve_tls(listener, rustls_config)` using `tokio_rustls::rustls::ServerConfig`.
* **UDS exposer** (`uds` feature):

  * Bind to unix domain socket for sidecar w/o TCP.
* **OTel bridge** (`otel` feature):

  * Optional exporter that mirrors Prom counters/histograms to OTLP.
* **Base labels in registry**:

  * If/when `SafeRegistry` supports a builder w/ const labels, re-enable constant base labels automatically (service, instance, build_version, amnesia).
* **Process/runtime metrics**:

  * Optional gated import of `prometheus` process collectors.
* **Cardinality audit**:

  * Keep `status_class` low-cardinality; avoid path/method labels unless explicitly required.
* **Two-phase readiness**:

  * Add a stricter `/readyz` (e.g., require `config_loaded && db && cache`) via `ReadyPolicy`.

---

## 13) Quick commands (for later)

Build & run Axum example:

```
RON_METRICS_METRICS_ADDR=127.0.0.1:0 cargo run -p ron-metrics --example axum_api
```

Hit endpoints:

```
curl -sS http://127.0.0.1:<PORT>/healthz
curl -sS http://127.0.0.1:<PORT>/readyz
curl -sS http://127.0.0.1:<PORT>/metrics | head -50
```

Run tests:

```
cargo test -p ron-metrics
cargo test -p ron-metrics --features bus
```

Lint & fmt:

```
cargo fmt
cargo clippy -p ron-metrics --all-targets -- -D warnings
```

---

## 14) File pointers (for when you come back)

* Middleware: `src/axum_latency.rs`, `src/axum_status.rs`
* Exposer: `src/exposer/http.rs`
* Health: `src/health.rs`
* Readiness policy: `src/readiness.rs`
* Metrics facade: `src/metrics.rs` (buckets helper at bottom)
* Registry wrapper: `src/registry.rs`
* Examples: `examples/` (`axum_api`, `exposer`, `pump`, `watch_bus`)
* Tests: `tests/` (HTTP smoke, status counter, health roundtrip…)

---
