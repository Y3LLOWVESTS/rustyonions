---

title: Security Notes — ron-transport
crate: ron-transport
owner: Stevan White
last-reviewed: 2025-09-28
status: draft
-------------

# Security Documentation — `ron-transport`

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-transport`.
It complements the repo-wide Hardening, Interop, and IDB blueprints and the crate’s `docs/CONFIG.md`.

`ron-transport` is a **library** crate. It provides dialers/listeners and typed, timeout-guarded I/O over TCP/TLS and (optionally) Arti/Tor. It does **not** implement overlay/DHT/auth protocols and does not run a long-lived service loop.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats (examples)                            | Relevant in `ron-transport`? | Mitigation (in or around this crate)                                                                                                                                                                        |
| -------------------------- | --------------------------------------------- | :--------------------------: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Impersonated peers, unauthenticated endpoints |             **Y**            | TLS 1.3 via `tokio-rustls::rustls`; optional Tor/Arti circuit establishment; peer identity is proven by the **caller’s** policy (e.g., cert pinning) — transport exposes hooks, does not embed policy.      |
| **T**ampering              | On-path mutation, data integrity loss         |             **Y**            | TLS AEAD integrity; size ceilings and chunking prevent state desync via oversized frames; end-to-end content integrity (e.g., BLAKE3/CAS) belongs to overlay/service layers.                                |
| **R**epudiation            | Missing/auditable logs                        |             **Y**            | Structured logs (JSON) with `peer_addr`, `conn_id`, `corr_id` pass-through; stable error kinds for audit; metrics counters/histograms for dials, handshakes, rejects.                                       |
| **I**nformation Disclosure | Key leakage, secrets in logs, peer info leak  |             **Y**            | No logging of secrets; PEM handling with permission checks (0600) and in-memory **zeroization**; **amnesia mode** forbids persistent artifacts; TLS keys loaded by consumer and handed in as typed configs. |
| **D**enial of Service      | SYN flood, slow-loris, resource exhaustion    |             **Y**            | Hard **read/write/idle** timeouts; **global/per-conn caps**; **token-bucket rate limits** (ingress/egress); bounded channels; early **reject** over buffering; listener/dial semaphores.                    |
| **E**levation of Privilege | Unauthorized operations                       |            **N***            | This crate performs **no authorization**. Capability tokens (e.g., macaroons) and role checks are enforced by **consumers**. Transport must not bypass or cache auth.                                       |

*N at this layer because `ron-transport` doesn’t implement authorization. It must, however, avoid weakening upstream auth (e.g., never downgrading TLS, never exposing plaintext side-channels).

---

## 2) Security Boundaries

* **Inbound (library surface):** constructors for **listeners** (TCP/TLS/Arti, feature-gated), **dialers**, and **connection handles** (reader/writer split). No HTTP endpoints, no `/readyz` here.
* **Outbound:** OS sockets, TLS handshakes via `tokio-rustls`, optional Arti/Tor circuits (in-process; no external `tor` daemon).
* **Trust Zones:**

  * **Default:** public network exposure **via the consumer**; treat all peers as untrusted.
  * **UDS path:** local-trust with `SO_PEERCRED` allowlist (if consumer enables UDS).
* **Assumptions:**

  * Kernel facilities (config/metrics/health) function correctly.
  * Callers enforce **peer authentication** policy and any capability tokens.
  * Persistent storage (if any) is owned by the consumer; amnesia mode = RAM-only.

---

## 3) Key & Credential Handling

* **Key/Secret types in vicinity:**

  * **TLS private keys** (PEM) and **cert chains** (PEM) — *consumer-owned*, passed as `tokio_rustls::rustls::ServerConfig`/client configs.
  * **Arti/Tor state** — in-process; **no external daemon**; respect amnesia (no on-disk caches).
  * **Capability tokens (macaroons)** — *consumer-owned*; **not** parsed or validated by this crate.
* **Storage & lifecycle:**

  * TLS keys **must not** be world/group-readable (reject on Unix if perms ≠ `0600`).
  * Parse PEM in one pass; **zeroize** buffers after use.
  * In **amnesia mode**, no persistent secrets/artifacts; ephemeral material is zeroized on drop.
* **Rotation:**

  * TLS cert/key rotation is **consumer-driven** (hot-swap config + listener rebind as needed). Recommended max age: **≤ 30 days**.
  * Tokens/macaroons rotate under consumer policy; never log or echo.
* **Zeroization:**

  * Use `zeroize::Zeroizing` or explicit scrubbing for key buffers; avoid copies; prefer borrowing where safe.

---

## 4) Hardening Checklist (from Blueprint)

> Do **not** check boxes here—this section is used in PR reviews.

* [ ] 5s timeout on **read** and **write**; 60s **idle** cutoff (configurable).
* [ ] Global **max_conns** and per-conn inbox bounded; no unbounded queues.
* [ ] **Rate limits** (ingress/egress token-bucket) enabled in production profiles.
* [ ] **Request/frame cap**: 1 MiB (OAP/1 mirror); stream in ~64 KiB chunks.
* [ ] UDS sockets: dir `0700`, socket `0600`, `SO_PEERCRED` allowlist.
* [ ] TLS: only `tokio_rustls::rustls::*` types; PEM key perms `0600`; never log secrets.
* [ ] **Amnesia** honored: no on-disk caches/keys when enabled.
* [ ] Chaos test under load: restart, ensure callers’ `/readyz` flips appropriately (transport exposes readiness states, callers wire them).

---

## 5) Observability for Security

* **Metrics (examples):**

  * `transport_dials_total{backend,result}`
  * `transport_accepts_total{backend}`
  * `handshake_latency_seconds{backend}` (fixed buckets)
  * `rejected_total{reason="rate_limit"|"too_large"|"peer_limit"|"arti_bootstrap"}`
  * `tls_handshake_failures_total{backend}`
  * `conn_inflight{backend}`
* **Logs:** Structured JSON; minimally include `conn_id`, `peer_addr`, `backend`, `reason` (stable error kind), and `corr_id` if provided by caller. **Never** log secrets or raw key material.
* **Health/Readiness:** This crate exposes **readiness states** (`Ready | Degraded(reason) | NotReady(reason)`); consumers propagate to `/readyz` and must fail-closed on `NotReady` or `Degraded` per policy.

---

## 6) Dependencies & Supply Chain

* **Security-sensitive crates (feature-gated as noted):**

  * `tokio-rustls` / `rustls` (TLS 1.3; PQ-hybrid readiness via config/feature upstream).
  * `bytes` (zero-copy buffers on hot path).
  * `zeroize` (secret scrubbing).
  * `rustls-pemfile` (PEM parsing for validation-only paths).
  * `arti-client` (when `arti` feature on; in-process Tor).
  * `tokio` (runtime); `tokio-util` (cancel tokens, split).
* **Controls:**

  * **Pinned versions** at workspace root; dependabot or equivalent for updates.
  * **`cargo-deny`** (advisories, bans, licenses) in CI.
  * **`cargo-audit`** as a periodic job.
  * **SBOM** generated at release and stored under `docs/sbom/`.
  * Reproducible builds where feasible; avoid git dependencies in prod.

---

## 7) Formal & Destructive Validation

* **Property tests:** enforce size ceilings (`>1MiB ⇒ TooLarge`), rate-limit semantics (±20% pacing over 5s), and deadline compliance for dial/read/write.
* **Fuzzing (boundary I/O):** randomize chunk sizes near limits; malformed PEM inputs must fail safely without panic. (OAP parsing fuzzing lives in protocol crates.)
* **Loom (dev-only):** model single writer + bounded inbox + cancellation; assert no deadlocks/leaks.
* **Chaos tests (integration):** under load, kill a reader/writer and simulate dial/handshake failures; metrics remain coherent; no task leaks; readiness transitions correct (`Degraded("arti_bootstrap")` → `Ready` or `NotReady` on timeout).
* **TLA+ (optional):** if a consumer requires formalization of dial/backoff or writer-inbox delivery guarantees, include a spec sketch in that service’s repo.

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* 
* **Disclosure policy:** See repository root `SECURITY.md` (coordinated disclosure; private issue intake).

---

## 9) Migration & Upgrades

* **Auth or key-handling changes** (e.g., PQ-hybrid toggles that alter interop): require a **major** version bump and a clear CHANGELOG migration with rollback guidance.
* **Deprecations:** provide env/flag aliases for ≥ 1 minor release; emit structured warnings; define end-of-support dates.
* **Feature gates:** adding/removing `arti`/`pq` features must be explicitly documented in `docs/CONFIG.md` and release notes.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client / Peer] -->|TLS 1.3 (optionally via Tor/Arti)| B(ron-transport)
  B -->|typed errors + metrics| D[Reject: TooLarge / RateLimited / Timeout / ArtiBootstrap]
  B -->|Conn handles (read/write with deadlines)| C[Caller (overlay/service)]
  C -->|Auth / Capability checks| E[Downstream app logic]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text description:** A client connects over TLS (optionally via Tor/Arti). `ron-transport` enforces deadlines, caps, and rate limits, yielding typed errors and metrics on violation. The consumer performs capability/auth checks and application logic; transport provides no auth bypass.

---

### Notes for Reviewers

* This file is **library-scoped**: anything about overlay protocol semantics, capability validation, or HTTP endpoints belongs to the **consuming service**.
* Ensure changes in `docs/CONFIG.md` related to TLS/Arti/limits remain consistent with these security requirements.
* Prefer **rejecting early** and **measuring visibly** to prevent latent DoS amplification.
