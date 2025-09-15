
---

crate: micronode
path: crates/micronode
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Thin, one-command “developer node” that exposes local app-plane endpoints plus standardized ops surfaces (`/healthz`, `/readyz`, `/metrics`) for SDK/dev workflows.

## 2) Primary Responsibilities

* Provide a minimal, always-on local host for app/SDK experiments and offline testing.
* Expose standard ops endpoints for golden metrics and health/ready checks.
* (Optional) Consult `ron-policy` for simple allow/deny hooks during development.

## 3) Non-Goals

* Not a production traffic gateway, scheduler, or orchestrator.
* No persistent storage, indexing, or transport responsibilities.
* No authn/z beyond optional policy checks.

## 4) Public API Surface

* Re-exports: none.
* HTTP (Axum):

  * `GET /` → service status (ok+uptime)
  * `GET /status` → readiness+uptime
  * `GET /version` → `{service, version}`
  * Ops: `GET /healthz`, `GET /readyz`, `GET /metrics`
* CLI: none (configured via env).

## 5) Dependencies & Coupling

* Internal: optional `ron-policy` (loose; replaceable: **yes**).
* External (top):

  * `axum 0.7` (HTTP server; stable, permissive) — medium risk surface.
  * `tokio 1.x` (async runtime) — low risk, core infra.
  * `prometheus 0.14` (metrics) — low risk.
  * `tracing`, `tracing-subscriber` (logs) — low risk.
  * `serde/serde_json` (payloads) — low risk.
* Runtime services: Network (TCP listener). No DB/OS/Crypto bindings.

## 6) Config & Feature Flags

* Env:

  * `MICRONODE_ADDR` (default `127.0.0.1:3001`).
* Features:

  * `policy` → enables `ron-policy` integration (allow/deny hook).

## 7) Observability

* `/metrics` (Prometheus text), default registry.
* `/healthz` (liveness) and `/readyz` (AtomicBool-gated readiness).
* Structured logs via `tracing` + `RUST_LOG`.

## 8) Concurrency Model

* One `axum::serve` task, graceful shutdown on `ctrl_c`.
* No channels; internal readiness via `Arc<AtomicBool>`.
* Backpressure: Hyper’s defaults; no explicit timeouts or concurrency limits (future: tower `Timeout`/`LoadShed`/`Buffer`).

## 9) Persistence & Data Model

* None (stateless). No artifacts/retention.

## 10) Errors & Security

* Error taxonomy: currently “bubbling” server errors; no typed app errors.
* Security: HTTP only; no TLS/mTLS; no authn/z by default.
* PQ-readiness: N/A at this layer (defer to transport/gateway).

## 11) Performance Notes

* Hot path: trivial JSON handlers and metrics scrape.
* Targets: 1–2k rps on dev laptops easily; latency \~sub-ms for `/healthz`.

## 12) Tests

* Unit: handlers return expected payloads/HTTP codes.
* Integration: bind ephemeral port, probe ops endpoints.
* E2E: run alongside other services not required.
* Loom/fuzz: not necessary.

## 13) Improvement Opportunities

* **Bug:** `main() -> anyhow::Result<()>` but `anyhow` not listed; either add `anyhow` (workspace) or change to `Result<(), Box<dyn std::error::Error>>`.
* Add tower middlewares: `Timeout`, `ConcurrencyLimit`, request metrics histogram.
* Config struct (serde) + env bridging; expose build info (/version detail).
* Optional TLS listener (tokio-rustls) behind a feature.

## 14) Change Log (recent)

* 2025-09-14 — Initial skeleton with ops endpoints and optional policy hook.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 2
* Observability: 4
* Config hygiene: 3
* Security posture: 2
* Performance confidence: 4
* Coupling (lower is better): 4

---
