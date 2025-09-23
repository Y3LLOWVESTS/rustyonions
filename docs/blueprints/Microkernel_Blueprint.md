
**`/docs/blueprints/Microkernel_Blueprint.md`**

````markdown
---
title: RustyOnions — Microkernel Blueprint (2025 Update, FINAL+)
status: FINAL (canon-aligned)
last-updated: 2025-09-22
audience: contributors, reviewers, ops
scope: Pillar 1 — Kernel & Orchestration
non-scope: app semantics, policy, economics, storage/DHT logic
crates-impacted: ron-kernel, ron-bus, ryker
six-concerns: [SEC, RES, PERF, GOV, DX]
references:
  - COMPLETECRATELIST.MD (canonical 33 crates)
  - 12_Pillars.md (Pillar 1 invariants & PR checklist)
  - CarryOverNotes.md (global deltas & amnesia mode)
  - Microkernel_Blueprint.md (prior kernel API & OAP constants)
---

# 0) Canon Alignment (Do Not Drift)

- **Canonical set: 33 crates**; Pillar 1 includes `ron-kernel`, `ron-bus`, `ryker`. Adding/removing crates is out of scope here. 
- **Global deltas to reflect in all docs:** `svc-dht` is first-class; `svc-overlay` excludes DHT logic; **Arti lives under `ron-transport`** as a feature. Kernel exposes a **Global Amnesia Mode** flag consumed by services. :contentReference[oaicite:1]{index=1}
- **Review gates (Pillar 1):** no locks across `.await`, bounded mailboxes/queues, crash-only supervision with jitter, bus backpressure metrics. :contentReference[oaicite:2]{index=2}

---

# 1) Kernel Contract (Frozen & Minimal)

## 1.1 Public API (re-exports)
The kernel crate **re-exports exactly** this public surface:

```rust
pub use {
  Bus,
  KernelEvent,
  Metrics,
  HealthState,
  Config,
  wait_for_ctrl_c,
};
````

`KernelEvent` includes:

```rust
enum KernelEvent {
  Health        { service: String, ok: bool },
  ConfigUpdated { version: u64 },
  ServiceCrashed{ service: String, reason: String },
  Shutdown,
}
```

This surface is **semver-frozen**; changes require a major bump and migration notes.&#x20;

## 1.2 Boundaries (what the kernel **is / is not**)

* **Is:** lifecycle/supervision, config hot-reload, health/readiness wiring, **in-process bus**, canonical observability hooks.&#x20;
* **Is not:** overlay/DHT/storage/ledger/gateway logic; those live in their respective services/libs.&#x20;

## 1.3 Hard Invariants

* **Monomorphic Bus**; broadcast-backed with bounded buffers; topic-style filtering via helpers; **lag/drop metrics present**.&#x20;
* **No locks across `.await`** on supervisory paths; **bounded** channels/queues (Ryker mailboxes, bus subscribers).&#x20;
* **TLS type** at kernel boundaries is `tokio_rustls::rustls::ServerConfig` (not `rustls::ServerConfig`).&#x20;
* **Golden metrics:** `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`.&#x20;
* **Amnesia Mode flag:** kernel-surfaced boolean; services honor it (RAM-only caches, zeroization) and it appears as a metrics label (`amnesia="on|off"`).&#x20;

---

# 2) Protocol & Addressing (Kernel exposes, does not enforce)

**OAP/1** is the framed protocol of record (HELLO/START/DATA/END/ACK/ERROR); default **`max_frame = 1 MiB`**. **Storage streaming chunk = 64 KiB** and must not be conflated with frame size. (Spec is external; local `/docs/specs/OAP-1.md` is a pointer to the normative doc.)&#x20;

**Content addressing (normative)**: `b3:<hex>` using **BLAKE3-256**; services **MUST verify full digest** before returning bytes; **OAP frames are not hashed**.
**Explicit rule (restored & bolded):** **`DATA` headers MUST include** `obj:"b3:<hex>"` **for object IDs**; frames above negotiated `max_frame` are rejected (by services), and kernel exposes observability for rejects.

Kernel role: **exposure only** (metrics/events/readiness), not policy or payload policing.&#x20;

---

# 3) Pillar-1 Architecture (in brief)

```
+----------------------+
|      ron-kernel      |  <- supervision, config, health/ready, Metrics, Bus
|  - supervisor/*      |
|  - bus.rs            |
|  - config.rs         |
|  - health.rs         |
+----------+-----------+
           |
           v (events)
+----------------------+      +------------------+
|      ron-bus         |<---->|       ryker      |
|  broadcast helpers   |      | bounded mailboxes|
+----------------------+      +------------------+
```

* **`ron-bus`**: broadcast abstraction + helpers (`recv_matching`, timeouts), bounded buffers, lag/drop metrics.&#x20;
* **`ryker`**: actor/mailbox utilities; used by kernel internally but not leaked in public API.&#x20;

---

# 4) Integrations (kept explicit as non-goals)

* **Transport:** Services use **`ron-transport`** (Arti/Tor under `arti` feature). Kernel does **not** run transport loops.&#x20;
* **Overlay/DHT:** `svc-overlay` handles sessions/gossip; **`svc-dht` owns Kademlia/Discv5**; kernel has no routing or k-bucket logic.&#x20;
* **Observability:** **`ron-metrics`** provides Prometheus & `/healthz`/`/readyz`; kernel emits the canonical counters/histograms only.&#x20;

---

# 5) Concurrency & Resilience

* **Crash-only**: supervised restarts with jittered backoff; crashes emit `KernelEvent::ServiceCrashed{service, reason}`.&#x20;
* **Bounded everything** (mailboxes, bus, internal queues); watch **bus lag**. `/readyz` degrades under restart storms or saturation.&#x20;
* **No locks across `.await`** (lint/review enforced).&#x20;

---

# 6) Configuration & Amnesia Mode

* **Config watcher** exists and is hot-reloadable (emit `KernelEvent::ConfigUpdated{version}`).&#x20;
* **Amnesia Mode** (kernel-level flag):

  * Services interpret as RAM-only, ephemeral logs, aggressive zeroization.
  * Kernel exposes a read-only snapshot API and labels metrics with `amnesia`.&#x20;

---

# 7) Observability & Health

* **Metrics (kernel):** `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds` (plus service-side `rejected_total{reason=...}`).&#x20;
* **Events:** health OK events from ingress observations; **crashes include a structured `reason`**.&#x20;
* **Readiness:** `/healthz` = liveness; `/readyz` = backpressure/supervisor state (fail on saturation or restart storms).&#x20;

---

# 8) Security & PQ Readiness (SEC)

* **TLS types:** `tokio_rustls::rustls::ServerConfig` wherever kernel touches TLS config; never `rustls::ServerConfig` directly.&#x20;
* **OAP limits:** frames over negotiated `max_frame` rejected by services; kernel ensures **observability** (rejected counters) rather than enforcement.&#x20;
* **PQ roadmap (expanded):**

  * **OAP/2** may introduce **hybrid X25519+Kyber**. The kernel remains **crypto-neutral** but MUST NOT block PQ adoption.
  * **Config/feature flags:** Kernel must **pass through** transport/proto PQ flags untouched (e.g., `pq_hybrid=true`), and surface them in health snapshots so ops can verify posture.

---

# 9) Developer Experience (DX)

* **Stable API** (1.1) + clear events reduce churn for services/SDKs.&#x20;
* **Hermetic tests**: bus/topic/load use `tokio::io::duplex` or in-proc channels; no network required.&#x20;
* **OAP spec policy:** keep local OAP file as a **stub linking to the external normative spec** to avoid drift.&#x20;

---

# 10) Verification Plan (Tests & CI Gates)

## Unit/Integration

* **Bus:** pub/sub, topic filtering (`recv_matching`), load/lag without panics.&#x20;
* **OAP/1 frames:** HELLO, START, **DATA(header+body with `obj:"b3:<hex>"`)**, END, ACK, ERROR (invalid type / quota).&#x20;

## Chaos / Resilience

* **Supervisor backoff jitter** under crash storms; **/readyz degradation** under sustained lag.&#x20;

## **Formal & Perf Hooks (added)**

* **TLA+ specs** for supervisory paths and bus semantics (`specs/supervisor.tla`, `specs/bus.tla`) — model restarts, bounded queues, and absence of deadlock.&#x20;
* **Perf simulation harness** for bus/OAP throughput in `testing/performance` (publish rates vs. lag, frame rate vs. ACK credit).&#x20;

## CI Label Routing

* PRs touching kernel/bus/ryker must run **RES** jobs (loom/fuzz if applicable) and **GOV** checks (readiness gates, runbook presence).&#x20;

---

# 11) Performance Defaults (Tunable)

* **Bus per-subscriber capacity:** dev ≥ 8; production 32–64 (watch `bus_lagged_total`).&#x20;
* **Supervisor backoff:** jittered, capped; track restarts via metrics/logs.&#x20;

---

# 12) Risks & Mitigations

| Risk                                 | Mitigation                                                                                   |
| ------------------------------------ | -------------------------------------------------------------------------------------------- |
| API drift on kernel surface          | Freeze re-exports & `KernelEvent` shape; CI denies public-API changes without version bump.  |
| Hidden unbounded queues              | Pillar-1 checklist + tests that force saturation and observe lag/drop counters.              |
| Transport/overlay coupling sneaks in | Keep all I/O loops in services; `ron-transport` only at type/config boundaries.              |
| Amnesia mode not uniformly honored   | Central kernel flag; assert via metrics labels and service integration tests.                |

---

# 13) Acceptance Checklist (PR Must Pass)

* [ ] **API Surface:** Re-exports & `KernelEvent` (incl. `reason`) unchanged.&#x20;
* [ ] **Concurrency:** No locks across `.await`; all channels bounded; Ryker mailboxes bounded.&#x20;
* [ ] **Metrics:** `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds` present & incremented in tests.&#x20;
* [ ] **Readiness:** `/readyz` fails under sustained backpressure or restart storms.&#x20;
* [ ] **TLS Type:** Any TLS in kernel uses `tokio_rustls::rustls::ServerConfig`.&#x20;
* [ ] **Amnesia:** Kernel flag surfaced; metrics label present; service tests assert honoring.&#x20;
* [ ] **OAP/1 Rule:** **`DATA` header includes `obj:"b3:<hex>"`**; no conflation of 1 MiB frame vs. 64 KiB storage chunk.
* [ ] **Formal/Perf Hooks:** TLA+ specs exist & perf harness runs smoke tests.&#x20;

---

# 14) Ops Notes (Runbook Hooks)

* Watch **restarts** (>5 in 5m) and **bus lag spikes**; wire alerts off the kernel metrics. Use `KernelEvent::ServiceCrashed{reason}` to triage.&#x20;
* `/readyz` should **fail early** under saturation/restart storms — this is by design to shed load.&#x20;
* Amnesia on? Expect higher RAM churn; alert when RSS crosses thresholds while `amnesia="on"`.&#x20;

---

# 15) What Changed vs. Prior Blueprint

* Elevated **PQ readiness** from a note to an explicit **config/feature-flag passthrough** requirement.&#x20;
* **Restated & bolded** the **`DATA` header rule** (`obj:"b3:<hex>"`) in the OAP section for clarity.
* Added **TLA+** and **perf harness** hooks as first-class verification items (linked to CI routing).&#x20;
* Kept the **frozen API surface**, OAP constants, content addressing, and Pillar-1 invariants unchanged.

