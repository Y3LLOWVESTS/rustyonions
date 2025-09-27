
---

**File:** `crates/ron-kernel/docs/INTEROP.md`

````markdown
# 🔗 INTEROP.md — ron-kernel

*Audience:* developers, auditors, external SDK authors  
*msrv:* 1.80.0  
*concerns:* Microkernel/Interop focus → **[SEC, RES, PERF, GOV, DX]**【turn1file0†MICROKERNEL_BLUEPRINT.MD†L12-L16】

---

## 0) Purpose

Declare the **interop surface** of `ron-kernel`—what the kernel exposes and guarantees—and what it explicitly does **not** own.

**Kernel scope (Pillar-1):** lifecycle/supervision, config hot-reload, health/readiness wiring, **in-process bus**, and canonical observability hooks; it is **not** overlay/DHT/storage/ledger/gateway logic【turn1file4†MICROKERNEL_BLUEPRINT.MD†L28-L32】.

**Global deltas reflected here:** canonical 33-crate alignment, Arti under `ron-transport` feature, and **Global Amnesia Mode** surfaced by the kernel for services to honor【turn1file0†MICROKERNEL_BLUEPRINT.MD†L22-L69】.

Kernel is **crypto/protocol-neutral**: it **exposes** OAP/1 constants and content-addressing rules via metrics/events, but **does not enforce** wire behavior (services do)【turn1file2†MICROKERNEL_BLUEPRINT.MD†L5-L11】.

---

## 1) Protocols & Endpoints

**Kernel-owned ingress endpoints:** **None.** Services (e.g., gateway/storage/overlay) terminate HTTP/OAP/QUIC. Kernel provides types, supervision, health/ready wiring, and metrics【turn1file6†MICROKERNEL_BLUEPRINT.MD†L5-L9】.

**Normative protocol facts (observability surfaced by kernel):**
- **OAP/1 framing:** default **`max_frame = 1 MiB`**; **storage streaming chunk = 64 KiB** (not the same thing). Enforcement in services; kernel records rejects/latency【turn1file2†MICROKERNEL_BLUEPRINT.MD†L5-L10】.
- **Content addressing:** object IDs are **`b3:<hex>`** (BLAKE3-256). Services **MUST verify full digest** before serving bytes; kernel exposes counters/latency only【turn1file2†MICROKERNEL_BLUEPRINT.MD†L7-L10】.
- **DATA header rule (explicit):** `DATA` frames include `obj:"b3:<hex>"` for object IDs; frames above negotiated `max_frame` rejected **by services**【turn1file2†MICROKERNEL_BLUEPRINT.MD†L7-L10】.
- **TLS type at boundaries:** any TLS touched by kernel uses `tokio_rustls::rustls::ServerConfig` (never `rustls::ServerConfig`)【turn1file4†MICROKERNEL_BLUEPRINT.MD†L33-L38】.
- **PQ readiness posture:** kernel remains **crypto-neutral**; **passes through** PQ feature flags (e.g., `pq_hybrid=true`) and surfaces them in health snapshots so ops can verify posture【turn1file6†MICROKERNEL_BLUEPRINT.MD†L41-L45】【turn1file1†MICROKERNEL_BLUEPRINT.MD†L7-L11】.

> **Spec policy:** Keep local OAP file as a **stub** pointing to the external normative spec to avoid drift【turn1file6†MICROKERNEL_BLUEPRINT.MD†L50-L53】.

---

## 2) DTOs / Schemas

### 2.1 Kernel Public API (frozen surface)

The kernel re-exports **exactly** this set (semver-frozen; major bump required for changes)【turn1file4†MICROKERNEL_BLUEPRINT.MD†L1-L12】【turn1file4†MICROKERNEL_BLUEPRINT.MD†L25-L26】:

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

`KernelEvent` shape (internal enum)【turn1file4†MICROKERNEL_BLUEPRINT.MD†L15-L24】:

```rust
enum KernelEvent {
  Health        { service: String, ok: bool },
  ConfigUpdated { version: u64 },
  ServiceCrashed{ service: String, reason: String },
  Shutdown,
}
```

### 2.2 Polyglot Audit/Export Shape (JSON)

For cross-language auditors/SDK tooling, kernel events exported to logs/streams MUST serialize as **flat JSON** with variant in `"type"`:

```json
{ "type":"Health",        "service":"svc-storage", "ok":true }
{ "type":"ConfigUpdated", "version":42 }
{ "type":"ServiceCrashed","service":"svc-index","reason":"panic: sled iterator misuse" }
{ "type":"Shutdown" }
```

*Wire DTOs (OAP envelopes, manifests, etc.) live in `ron-proto` with `#[serde(deny_unknown_fields)]` to prevent drift; the kernel does **not** define wire payloads*【turn1file2†MICROKERNEL_BLUEPRINT.MD†L10-L11】.

---

## 3) Bus Topics

The kernel ships a **monomorphic**, broadcast-backed bus with **bounded** buffers and explicit **lag/drop metrics**【turn1file4†MICROKERNEL_BLUEPRINT.MD†L33-L39】. Supervisory paths obey “no locks across `.await`”; each task owns its own receiver【turn1file4†MICROKERNEL_BLUEPRINT.MD†L35-L36】.

### 3.1 Published Topics (canonical, stable)

* `kernel.health` → `KernelEvent::Health { service, ok }`
* `kernel.config.updated` → `KernelEvent::ConfigUpdated { version }`
* `kernel.service.crashed` → `KernelEvent::ServiceCrashed { service, reason }`
* `kernel.shutdown` → `KernelEvent::Shutdown`

Overflow **must not block**; bus saturation increments `bus_lagged_total` (and drop counters, if implemented)【turn1file5†MICROKERNEL_BLUEPRINT.MD†L14-L16】【turn1file4†MICROKERNEL_BLUEPRINT.MD†L37-L39】.

### 3.2 Subscriptions

Kernel doesn’t subscribe to app/service topics; services subscribe to kernel lifecycle/config. Readiness gates use **`HealthState` snapshots** to guard `/readyz` in services【turn1file6†MICROKERNEL_BLUEPRINT.MD†L31-L34】.

---

## 4) Canonical Test Vectors (kernel-focused)

> Location: `crates/ron-kernel/tests/vectors/`

1. **Bus overflow visibility**
   **Setup:** Publish N ≫ capacity.
   **Expect:** `bus_lagged_total` (and drop counters where present) increase; no deadlocks; supervisor still emits `ServiceCrashed` on induced panic【turn1file5†MICROKERNEL_BLUEPRINT.MD†L12-L16】【turn1file6†MICROKERNEL_BLUEPRINT.MD†L13-L15】.

2. **Readiness degradation**
   **Setup:** Keep a dependent service “not ready” past deadline.
   **Expect:** `/readyz` (service endpoint) returns **503** with explicit **degraded** semantics; kernel `HealthState` reflects non-ready snapshot【turn1file6†MICROKERNEL_BLUEPRINT.MD†L31-L34】.

3. **OAP/1 observability (non-enforcing)**
   **Setup:** Service injects frames > 1 MiB.
   **Expect:** Service rejects (400/413 per hardening); kernel counters reflect rejects; kernel never enforces wire rules【turn1file2†MICROKERNEL_BLUEPRINT.MD†L5-L11】.

4. **Content-address sanity**
   **Setup:** Known payload ⇒ `b3:<hex>`; service GET round-trip.
   **Expect:** Service verifies full digest; kernel exposes latency/error metrics only【turn1file2†MICROKERNEL_BLUEPRINT.MD†L7-L10】.

> OAP/manifest wire-vectors live with `oap`/`ron-proto`; kernel vectors center on **bus**, **health/readiness**, and **observability**.

---

## 5) Error Taxonomy (kernel-adjacent observability)

Services own the HTTP/OAP **envelope**; kernel standardizes **observability**:

* **400 BadVersion / 413 FrameTooLarge** → OAP enforcement in services; kernel increments reject counters only【turn1file2†MICROKERNEL_BLUEPRINT.MD†L5-L11】.
* **429 QuotaExceeded** → ingress quota (service); kernel surfaces reject/latency signals.
* **503 NotReady / Degraded** → readiness gate fail-closed (service); kernel’s `HealthState` drives readiness semantics【turn1file6†MICROKERNEL_BLUEPRINT.MD†L31-L34】.

---

## 6) Interop Guarantees

* **No Kernel Drift:** Public re-exports + `KernelEvent` shape are **frozen**; breaking changes require **major** version and migration notes【turn1file4†MICROKERNEL_BLUEPRINT.MD†L25-L26】.
* **SemVer Discipline:** Wire DTOs in `ron-proto` have schema-compat tests with `deny_unknown_fields` (kernel never mirrors them)【turn1file2†MICROKERNEL_BLUEPRINT.MD†L10-L11】.
* **Backward Compatibility (polyglot logs):** Unknown JSON fields in **exported audit** lines are ignored by consumers; field names are stable.
* **Auditability:** Canonical kernel vectors (overflow, degraded readiness) live under `/tests/vectors/`; OAP wire vectors live with `oap`.

---

## 7) References

* **Microkernel Blueprint (FINAL+)** — scope, frozen API, OAP/constants, amnesia label【turn1file0†MICROKERNEL_BLUEPRINT.MD†L28-L41】【turn1file4†MICROKERNEL_BLUEPRINT.MD†L33-L40】
* **Protocol & Addressing (Pillar-1)** — 1 MiB frame; 64 KiB storage chunk; `b3:<hex>`; DATA header rule【turn1file2†MICROKERNEL_BLUEPRINT.MD†L5-L10】
* **Integrations Non-Goals** — transport/overlay/DHT owned by services; kernel doesn’t run I/O loops【turn1file6†MICROKERNEL_BLUEPRINT.MD†L5-L11】
* **Security & PQ Readiness** — PQ flag pass-through; crypto-neutral kernel posture【turn1file6†MICROKERNEL_BLUEPRINT.MD†L37-L45】
* **Verification Plan** — loom/fuzz/TLA+, perf harness; CI label routing【turn1file1†MICROKERNEL_BLUEPRINT.MD†L33-L41】【turn1file5†MICROKERNEL_BLUEPRINT.MD†L1-L9】

---

## 8) Bus Flow (visual, not normative)

```mermaid
flowchart LR
  subgraph Kernel
    K[supervisor] -->|emit| B(broadcast bus)
    C[config watcher] -->|ConfigUpdated| B
    H[health/readiness] -->|Health| B
  end

  B -->|ServiceCrashed| Ops[operators/alerts]
  B --> SvcA[svc-gateway]
  B --> SvcB[svc-storage]
  B --> SvcC[svc-overlay]

  note right of Ops: watch restarts & bus lag metrics
```

*Bounded buffers; lag/drop metrics; no locks across `.await` on supervisory paths【turn1file4†MICROKERNEL_BLUEPRINT.MD†L33-L36】.*

---

## 9) Amnesia & PQ in Interop

* **Amnesia Mode (kernel flag):** surfaced as a read-only snapshot and as metrics label `amnesia="on|off"`; services must interpret as RAM-only caches, ephemeral logs, aggressive zeroization【turn1file2†MICROKERNEL_BLUEPRINT.MD†L53-L60】【turn1file0†MICROKERNEL_BLUEPRINT.MD†L68-L69】.
* **PQ Flags (pass-through):** kernel must not block PQ adoption; **passes through** transport/proto flags untouched and surfaces posture in health snapshots【turn1file1†MICROKERNEL_BLUEPRINT.MD†L7-L11】.

---

## 10) Verification Hooks (CI Teeth)

* **Hermetic tests**: bus/topic/load (no network)【turn1file6†MICROKERNEL_BLUEPRINT.MD†L49-L52】
* **Loom/Fuzz (RES)** where applicable; **TLA+ specs** for supervisor/bus (`specs/supervisor.tla`, `specs/bus.tla`)【turn1file1†MICROKERNEL_BLUEPRINT.MD†L33-L36】
* **Perf harness** in `testing/performance` (publish rates vs lag; frame rate vs ACK credit)【turn1file1†MICROKERNEL_BLUEPRINT.MD†L35-L37】
* **CI label routing**: kernel/bus/ryker PRs trigger RES jobs + GOV checks【turn1file5†MICROKERNEL_BLUEPRINT.MD†L6-L9】

---

## 11) Acceptance Gates (Definition of Done)

* [ ] **API surface frozen:** re-exports + `KernelEvent{..reason}` unchanged【turn1file3†MICROKERNEL_BLUEPRINT.MD†L7-L13】
* [ ] **Concurrency:** no locks across `.await`; all channels **bounded**【turn1file3†MICROKERNEL_BLUEPRINT.MD†L8-L8】
* [ ] **Metrics present:** `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds` validated by tests【turn1file3†MICROKERNEL_BLUEPRINT.MD†L9-L10】
* [ ] **Readiness:** `/readyz` degrades/fails under saturation or restart storms (service-side)【turn1file3†MICROKERNEL_BLUEPRINT.MD†L10-L10】
* [ ] **TLS type invariant:** `tokio_rustls::rustls::ServerConfig` only【turn1file3†MICROKERNEL_BLUEPRINT.MD†L11-L11】
* [ ] **Amnesia:** kernel flag surfaced; metrics labeled `amnesia`【turn1file3†MICROKERNEL_BLUEPRINT.MD†L12-L12】
* [ ] **OAP/1 rule:** `DATA` header includes `obj:"b3:<hex>"`; no conflation of frame vs chunk sizes【turn1file3†MICROKERNEL_BLUEPRINT.MD†L13-L13】
* [ ] **Formal/Perf hooks:** TLA+ specs exist; perf harness smoke passes【turn1file3†MICROKERNEL_BLUEPRINT.MD†L14-L14】

---

```

