
---

````markdown
---
title: Security Notes — ron-kernel
crate: ron-kernel
owner: Stevan White
last-reviewed: 2025-09-26
status: draft
---

# Security Documentation — ron-kernel

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-kernel`.  
It complements the repo-wide Hardening Blueprint and Interop/OAP spec. The kernel is **lifecycle/orchestration only** (supervision, bus, config, health, metrics surface). **Protocol and policy enforcement live in services/libs**, not in the kernel.

---

## 1) Threat Model (STRIDE)

| Category | Threats (examples) | Relevant in `ron-kernel`? | Mitigation / Notes |
|---|---|:--:|---|
| **S**poofing | Fake peers, unauth callers | N (library) | Kernel has **no public network surface** by default. Demo bins bind **localhost** only. Auth/caps are enforced by services (`svc-gateway`, `svc-passport`). |
| **T**ampering | Mutated frames, corrupted state | Partial | Protocol integrity checks (BLAKE3, frame limits) are enforced by services/OAP libs. Kernel protects its own state via **single-writer discipline**, **bounded channels**, and **no lock across `.await`**. |
| **R**epudiation | Unattributable actions | Y | **Structured JSON logs** with `service`, `event`, `corr_id`, and **golden metrics** (`service_restarts_total`, `bus_lagged_total`). Bus events are typed (`KernelEvent`). |
| **I**nformation Disclosure | Leaking secrets or internal topology | Partial | Kernel is **crypto-neutral**; it **does not store long-term secrets**. In **amnesia mode**, any transient credentials/paths are RAM-only and logs are ephemeral. Avoid logging PII/keys; redact diffs on config updates. |
| **D**enial of Service | Flooding queues, slow consumers, restart storms | Y | **Bounded queues** everywhere, **drop/reject over buffer**, lag counters, restart caps with jittered backoff, and early `/readyz` degradation in demo bins. |
| **E**levation of Privilege | Unauthorized operations | N (library) | No privileged ops; no policy engine here. Capability enforcement happens in auth/policy services. Kernel **never** bypasses them. |

---

## 2) Security Boundaries

- **Inbound (kernel library):**
  - **Process boundary:** CLI flags, environment, `Config.toml`.
  - **Control signals:** `SIGHUP` for reload (bins), `wait_for_ctrl_c()` for shutdown.
  - **Bus subscriptions:** internal subscribers receive `KernelEvent` via **bounded broadcast**.
  - **Demo observability (optional):** localhost `/metrics|/healthz|/readyz` served via `ron-metrics`.

- **Outbound:**
  - **Bus events:** `Health`, `ConfigUpdated`, `ServiceCrashed`, `Shutdown`.
  - **Logs:** structured JSON to stdout/stderr only.
  - **Metrics:** Prometheus registry (counters/gauges/histograms) for scraping by ops surfaces.

- **Trust Zone:**
  - Kernel runs **inside the same tenant/process space** as services embedding it. It is **not Internet-exposed** by default.

- **Assumptions:**
  - Services enforce **capabilities/quotas/limits** and control ingress e.g., `svc-gateway`.
  - Filesystem/UDS isolation & permissions are set by the **embedding service**.
  - OAP protocol invariants are enforced in **oap/ron-proto/service layers**.

---

## 3) Key & Credential Handling

- **Default posture:** Kernel holds **no persistent secrets**.
- **Possible transients:** demo TLS for local observability, or service-passed handles/paths.
- **Storage:**
  - **Amnesia ON:** RAM-only; no on-disk spill; ephemeral logs; timed zeroization.
  - **Amnesia OFF:** secrets belong to embedding services (e.g., `ron-kms`), not the kernel.
- **Rotation:** Secrets are rotated by upstream services. Kernel only **observes posture** (e.g., `pq.mode`) and reflects it in health; it does **not** manage keys.
- **Zeroization:** Any transient secret data handled in-proc should be wrapped with `zeroize::Zeroizing` and dropped promptly.
- **Logging:** Never log raw secrets or tokens; log **key IDs**/fingerprints only.

---

## 4) Hardening Checklist (applies to the kernel’s scope)

> Items here are requirements to be validated later. Leave unchecked until verified.

- [ ] No public network surface by default (library). Demo binds `127.0.0.1:0` only.
- [ ] Bounded channels only; overflow increments counters; bus lag visible.
- [ ] No locks across `.await` in supervisory/hot paths; clippy lint gate.
- [ ] Restart caps + jittered backoff; quarantine after threshold.
- [ ] Early `/readyz` degrade (demo): fail writes/accepts first; expose degraded metrics.
- [ ] Amnesia mode flag surfaced; metrics labeled `amnesia="on|off"`.
- [ ] Structured JSON logs with correlation IDs.
- [ ] `forbid(unsafe_code)` and deny-wall in CI.
- [ ] Timeouts/body caps/decompress caps (delegated to services).
- [ ] Auth/capability enforcement (delegated).
- [ ] UDS perms/PEERCRED for service sockets (delegated).

---

## 5) Observability for Security

- **Metrics (kernel):**
  - `service_restarts_total{service}`
  - `bus_lagged_total`
  - `request_latency_seconds{op}`
  - `busy_rejections_total{queue}`
  - `queue_dropped_total{queue}`
  - Labels include `amnesia`, `version`, `runtime_flavor`.

- **Logs (structured JSON):**
  - Fields: `ts`, `level`, `service="ron-kernel"`, `event`, `corr_id`, `reason`, `child`, `queue`, `amnesia`.
  - **Config diffs:** redacted (no secrets), include `version` and changed keys.

- **Health gates (demo bins):**
  - `/healthz` = liveness; `/readyz` = capacity/backpressure. On degradation, `/readyz` returns `503` and **sheds load deterministically**.

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates:**
  - `tokio`, `tokio-rustls` (if demo TLS), `prometheus`, `serde` (+ `deny_unknown_fields` on DTOs).
- **Policies:**
  - Workspace-pinned versions; no ad-hoc upgrades in leaf crates.
  - `forbid(unsafe_code)` in crate root.
  - `cargo-deny`: licenses/bans/advisories/sources must pass.
  - Repro builds: CI caches + lockfiles; SBOM generated at release (`/docs/sbom/`).
  - No direct `rustls::ServerConfig`; only `tokio_rustls::rustls::ServerConfig`.

---

## 7) Formal & Destructive Validation

- **Property tests:** bus overflow never blocks kernel; restart storm caps enforced; readiness transitions monotonic.
- **Loom tests:** concurrency interleavings of shutdown/config/bus; assert safety/liveness.
- **Fuzzing:** any framing/parser paths in demo bins fuzzed against malformed inputs.
- **Chaos:** panic child tasks under load; supervisor enforces backoff, metrics reflect; `/readyz` degrades early.
- **TLA+:** Supervisor/Bus spec sketches prove no deadlocks; eventual drain under shutdown.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- *
- **Disclosure policy:** See repository root `SECURITY.md`.

---

## 9) Migration & Upgrades

- **Auth/Policy semantics:** N/A in kernel (delegated).  
- **Breaking changes:** Any change to `KernelEvent` shape or public API requires a **major version bump** and explicit migration notes.  
- **Deprecations:** Provide aliases for ≥1 minor; log warnings; update docs/IDB/CONFIG/CONCURRENCY accordingly.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Embedding Service] -->|caps/TLS/limits (enforced here)| B((ron-kernel))
  B -->|KernelEvent: Health/Config/Crash| C[Other Services]
  B -.->|Reject (Busy/Degraded) in demo| A
  B -->|Structured logs + Metrics| D[Ops/Observability]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text:** The embedding service enforces authentication, capability tokens, and protocol limits. `ron-kernel` orchestrates lifecycle, emits typed events, and exposes metrics/logs. In demo mode, it sheds load deterministically and degrades `/readyz` early; it never becomes a bypass for auth/policy.

---

```
