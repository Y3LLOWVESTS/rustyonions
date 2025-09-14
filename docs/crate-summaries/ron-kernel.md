---

crate: ron-kernel
path: crates/ron-kernel
role: core
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny, stable microkernel that provides the project’s event bus, metrics/health surfaces, transport glue, and shutdown/supervision hooks—while exporting a frozen public API for the rest of the system.&#x20;

## 2) Primary Responsibilities

* Define and **freeze** the kernel’s public API (Bus, events, metrics/health, config, shutdown helper).&#x20;
* Offer **lossy broadcast** bus primitives and light supervision/backpressure behaviors that keep core paths non-blocking.&#x20;
* Expose **observability** endpoints (`/metrics`, `/healthz`, `/readyz`) that other services rely on for SLOs and runbooks.&#x20;

## 3) Non-Goals

* No app/business semantics (Mailbox, quotas, SDK ergonomics, payment). Those live outside the kernel.&#x20;
* No protocol spec surface (OAP/1 codec lives in its own crate; kernel only exports stable re-exports & helpers).&#x20;
* No persistent storage logic (DHT/Index/Storage are services; kernel stays stateless beyond in-memory counters).&#x20;

## 4) Public API Surface

* **Re-exports (frozen):** `Bus`, `KernelEvent::{Health {service, ok}, ConfigUpdated {version}, ServiceCrashed {service, reason}, Shutdown}`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.&#x20;
* **Key types / functions / traits:**

  * `Bus`: broadcast-based event channel; `new(capacity)`, `publish`, `subscribe` (+ helpers like `recv_with_timeout`, `recv_matching`).&#x20;
  * `KernelEvent`: canonical system events including structured crash reasons.&#x20;
  * `Metrics`: Prometheus counters/histograms + HTTP service for `/metrics`, `/healthz`, `/readyz`.&#x20;
  * `HealthState`: shared registry behind `/healthz`/`/readyz`.&#x20;
  * `Config` + `wait_for_ctrl_c()`: configuration holder and graceful shutdown helper (public re-export).&#x20;
  * **Transport hook (exposed at kernel root):** uses `tokio_rustls::rustls::ServerConfig`—type choice is part of the contract.&#x20;
* **Events / HTTP / CLI:** kernel itself exposes HTTP for `/metrics`, `/healthz`, `/readyz`; events published on `Bus` include `Health`, `ConfigUpdated`, `ServiceCrashed{reason}`, `Shutdown`.&#x20;

## 5) Dependencies & Coupling

* **Internal crates:** None required at runtime; kernel is intentionally standalone to avoid coupling creep. It coordinates with sibling crates (oap, gateway, sdk, ledger) via the frozen API.&#x20;
* **External crates (top focus):**

  * **tokio** (async runtime, channels) — core concurrency; widely maintained.
  * **axum 0.7** (HTTP for metrics/health) — aligned with workspace stack; handlers `.into_response()` pattern is enforced across services.&#x20;
  * **prometheus** (metrics registry/types) — counters/histograms exposed at `/metrics`.&#x20;
  * **tokio-rustls 0.26** (TLS types) — **contractually** the kernel’s TLS type for transports.&#x20;
  * **serde** (DTOs for JSON health/readiness replies).
    *Risk posture:* all are mature, widely used crates; the TLS type selection is explicitly locked to avoid drift.&#x20;
* **Runtime services:** Network (TCP/TLS listeners), OS (signals for shutdown). No direct DB or crypto beyond TLS type selection.&#x20;

## 6) Config & Feature Flags

* **Config:** The crate exports a `Config` type and `/readyz` capacity gates are expected at the service layer; kernel provides the plumbing and health surfaces.&#x20;
* **Features:** Kernel stays minimal; optional adapters (e.g., accounting) are feature-gated in **other crates** (ledger), not here—by design.&#x20;

## 7) Observability

* **Endpoints:** `/metrics`, `/healthz`, `/readyz` exposed by the kernel HTTP server; overload paths return **429/503** downstream when integrated.&#x20;
* **Golden metrics:** kernel-level `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`; service-level tie-ins for `rejected_total{reason}`.&#x20;

## 8) Concurrency Model

* **Bus:** `tokio::sync::broadcast` with **bounded capacity** (default \~4096) to remain lossy under pressure; **never blocks** kernel critical paths. On overflow, increment a counter and emit a throttled `ServiceCrashed{service:"bus-overflow", reason:...}`.&#x20;
* **Service control:** request/response via bounded `mpsc` + oneshot replies (pattern guidance in blueprint).&#x20;
* **Backpressure:** enforced at boundaries; **no unbounded queues** anywhere.&#x20;

## 9) Persistence & Data Model

* **None in kernel.** State is process-local and ephemeral (metrics, health map). Durable data (index/storage/DHT) is strictly out-of-kernel.&#x20;

## 10) Errors & Security

* **Typed events, not app errors:** kernel emits `ServiceCrashed{reason}` and `Health` for downstream operators; app-facing error envelopes (JSON with `{code,message,retryable,corr_id}`) are implemented in services.&#x20;
* **TLS type discipline:** transports must use `tokio_rustls::rustls::ServerConfig`. This guard prevents integration drift.&#x20;
* **E2E privacy boundary:** kernel treats app payloads as opaque; quotas/DoS are enforced outside the kernel (Gateway), keeping kernel unburdened.&#x20;

## 11) Performance Notes

* **Defaults:** OAP/1 `max_frame = 1 MiB` (spec), storage streaming chunk **64 KiB** (implementation detail, not a protocol limit). Avoid conflating these.&#x20;
* **Throughput posture:** bus buffers tuned ≥8 per subscriber; recommended 64 for active nodes.&#x20;

## 12) Tests

* **Unit/Integration (present):** `bus_basic`, `bus_topic`, `bus_load`, `event_snapshot`, plus overlay/index HTTP integration checks referenced in project status. &#x20;
* **Planned/CI gates:** property/fuzz tests for OAP frames live in `oap` crate; chaos/TLA+/loom are tracked as validation gates to reach “perfect”.&#x20;

## 13) Improvement Opportunities

* **Formal/destructive validation:** Land loom/fuzz/TLA+ for the kernel bus and supervision loops to lift M0→perfect.&#x20;
* **Metric completeness:** Ensure `bus_overflow_dropped_total` is exposed and documented; wire tight coupling with `ServiceCrashed{reason}` labels.&#x20;
* **Docs guardrails in CI:** Keep the grep suite that enforces `b3:<hex>` addressing and `max_frame = 1 MiB` vs. **64 KiB** chunking to prevent drift.&#x20;
* **Config watcher polish:** kernel re-exports a `Config`; add a no-surprises watcher stub and document reload semantics (currently implied, not codified).&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Status marked **“Microkernel complete for M0 (\~95%)”** with bus tests green and observability endpoints in place.&#x20;
* **2025-08-30** — **Public API freeze** restated in Final Blueprint; `ServiceCrashed{reason}` and TLS type contract emphasized. &#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 5 — explicit, frozen re-exports and variants.&#x20;
* **Test coverage:** 4 — kernel tests green; formal/chaos pending.&#x20;
* **Observability:** 4 — metrics/health/ready are present; dashboards/runbooks still evolving. &#x20;
* **Config hygiene:** 3 — contract is present; hot-reload/documented watcher semantics to tighten.&#x20;
* **Security posture:** 3 — TLS type fixed; app E2E boundary is honored; capability/quotas live in services. &#x20;
* **Performance confidence:** 3 — sane defaults; bus tuning guidance present; broader perf sims tracked at service layer. &#x20;
* **Coupling (lower is better):** **2** — intentionally minimal, app-agnostic core with stable surface.&#x20;


