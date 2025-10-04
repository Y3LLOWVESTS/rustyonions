

**Path:** `crates/ron-bus/docs/SECURITY.md`

````markdown
---
title: Security Notes — ron-bus
crate: ron-bus
owner: Stevan White
last-reviewed: 2025-09-27
status: draft
---

# Security Documentation — ron-bus

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-bus`.  
It complements the repo-wide Hardening Blueprint and Interop Blueprint.

> Scope reminder: `ron-bus` is a **pure in-process broadcast bus** (Tokio `broadcast`), with **no network I/O, no persistence, and no background tasks**. Hosts own I/O, authn/z, and metrics exposition.

---

## 1) Threat Model (STRIDE)

| Category | Threats | Relevant in `ron-bus`? | Mitigation |
|----------|---------|-------------------------|------------|
| **S**poofing | Fake identities, unauthenticated peers over the network | **No** (library has **no network**) | N/A in this crate. Hosts terminate TLS, authenticate (macaroons/OIDC), and then publish events. |
| **T**ampering | Mutating messages on the wire or disk | **Low** (in-proc only) | Types are owned/immutable once published; no persistence; hosts validate inputs **before** publishing. Prefer owned `Bytes`/`Arc<T>` payloads. |
| **R**epudiation | Missing/inauditable logs for security events | **Low / Host** | Library emits **no logs of payloads**. Hosts log security decisions (authz denials) and can log event *types* (not bodies). |
| **I**nformation Disclosure | PII or secrets in event payloads | **Medium (policy)** | **Do not publish secrets** via the bus. Payload logging is forbidden. Amnesia neutrality (metrics label only). Consider DTO lint/schemas at the host boundary. |
| **D**enial of Service | Slow subscribers, log amplification, unbounded queues | **High (core)** | **Bounded, lossy** broadcast; non-blocking publishers. Count `Lagged(n)`, update `bus_queue_depth`, and **rate-limit WARNs in host**. No unbounded queues. No blocking syscalls. |
| **E**levation of Privilege | Unauthorized operations or kernel bypass via bus | **Low (in-proc)** | Bus is not a policy gateway. Hosts enforce capability checks **before** publishing/consuming. Keep event variants non-privileged (no “execute” orders). |

**Notes**
- Primary risk surface is **availability** (DoS) due to misbehaving subscribers. Security posture maximizes liveness: **lossy + observable** overflow.
- Treat **payload contents** as potentially sensitive; prohibit dumping payloads to logs or metrics.

---

## 2) Security Boundaries

- **Inbound (this crate):** `Bus::new(capacity)`, `Bus::sender().send(Event)`, `Bus::subscribe().recv().await`.  
  No endpoints, no CLI, no file reads.
- **Outbound (this crate):** None. `ron-bus` updates **host-registered** metrics via callbacks/handles; does not emit logs itself.
- **Trust Zone:** Same-process, same-address-space. Trust is inherited from the host process.
- **Assumptions:**  
  - Kernel invariants hold (bounded bus, health, metrics exported by host).  
  - Hosts implement `/readyz` gating & quotas, and sanitize/log inputs.  
  - No direct DB/filesystem coupling in this crate.

---

## 3) Key & Credential Handling

- **Types of keys:** **N/A** (no TLS, no macaroons, no KMS in this crate).
- **Storage:** N/A (crate does not store keys).  
- **Rotation policy:** N/A here; handled by hosts or dedicated security crates.  
- **Zeroization:** N/A (no secrets). If a host accidentally carries sensitive data in events, that is a **policy violation**—fix at the edge.

---

## 4) Hardening Checklist (from Blueprint, adapted to library)

- [x] **No network/timeouts inside `ron-bus`.** I/O and timeouts belong to hosts.  
- [x] **Bounded broadcast capacity** set at construction; never unbounded.  
- [x] **Non-blocking publishers**; slow subscribers cannot wedge publishers.  
- [x] **Overflow accounting:** increment drop counter, update queue-depth gauge on `Lagged(n)`.  
- [x] **No payload logging** in library. Hosts must not log payload bodies.  
- [x] **Clippy deny:** `await_holding_lock`, `unwrap_used`, `expect_used`.  
- [x] **No background tasks** spawned by this crate.  
- [ ] **Host rate-limits WARNs** for overflow (library provides counters/gauges only).  
- [ ] **Host chaos test:** subscriber kill/restart under load—no publisher impact; `/readyz` degrades appropriately (host).  

*(Service-only items like request caps, decompression limits, UDS perms are **N/A** here.)*

---

## 5) Observability for Security

- **Metrics (host-exported, library-updated):**  
  - `bus_overflow_dropped_total` — Counter; `+n` on `RecvError::Lagged(n)`.  
  - `bus_queue_depth` — Gauge; set each loop tick and on lag to current backlog estimate.  
  - *(Optional host)* `bus_subscribers_total` — Gauge; avoid high-cardinality per-task labels.
- **Logs:** This crate does **not** log payloads. Hosts may log event **variant names** (not bodies) and context (service, corr_id), and should **rate-limit** overflow WARNs.
- **Health gates:** Library has no `/readyz`. Hosts should fail-closed when overflow counters climb continuously or depth remains high over a window.

---

## 6) Dependencies & Supply Chain

- **Core deps:** `tokio` (broadcast), `tokio-util` (CancellationToken for examples), metrics facade (via host).  
- **Security profile:** No crypto, no parsers, no alloc-heavy codecs in this crate.  
- **Pinned versions:** via workspace root `Cargo.toml`.  
- **Controls:**  
  - `cargo-deny` (advisories, licenses, bans, sources) in CI.  
  - Repro builds on CI; lockfile committed.  
  - **SBOM** generated at release (`/docs/sbom/`)—library shows minimal surface.

---

## 7) Formal & Destructive Validation

- **Property tests:**  
  - Fan-out correctness without lag; publishers remain non-blocking under forced lag.  
  - Receiver-sharing negative test (enforces **one receiver per task**).
- **Loom (lite):** 2 pub × 2 sub × capacity=2; assertions: no deadlocks; `Lagged(n)` reported; send never blocks; close semantics respected.  
- **Chaos (host-level):** Kill/restart a subscriber under load; publishers unaffected; remaining subscribers progress; metrics reflect behavior.  
- **Reload migration (host-level):** Cut over to a new `Bus` (different capacity) under steady publish load; assert no message gaps for non-lagging subscribers and no artificial spike in drop counter.  
- **TLA+ (optional, host/kernel):** If the kernel models health/event DAGs, the bus assumptions (bounded, lossy, observable) appear as environment constraints.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- * 
- **Disclosure policy:** See repo root `SECURITY.md`.

---

## 9) Migration & Upgrades

- **Event surface changes** (e.g., adding variants) are **additive** in minor versions.  
- **Breaking changes** to bus semantics (capacity behavior, overflow policy) require a **major version** and an explicit migration guide.  
- **Deprecations** must include timelines and host-side mitigation steps.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  subgraph Host Process (trusted zone)
    P[Publisher code] -->|send(Event)| BUS[(ron-bus: broadcast(capacity))]
    BUS --> RX1[Subscriber A (unique receiver)]
    BUS --> RX2[Subscriber B (unique receiver)]
    BUS -->|lag| M[metrics: overflow_dropped_total++, queue_depth set]
  end
  style BUS fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text description:** Within a single trusted process, publisher code sends events into the bounded `ron-bus`. Each subscriber has a unique receiver. If a subscriber lags, the bus drops older messages **for that receiver**; the library updates drop counters and queue depth via host-registered metrics. No payloads are logged; no network I/O occurs in this crate.

```
```
