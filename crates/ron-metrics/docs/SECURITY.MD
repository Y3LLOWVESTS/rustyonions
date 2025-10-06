

---

````markdown
---
title: Security Notes — ron-metrics
crate: ron-metrics
owner: Stevan White
last-reviewed: 2025-10-05
status: draft
---

# Security Documentation — ron-metrics

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-metrics`.  
It complements the repo-wide [Hardening Blueprint](../../docs/Hardening_Blueprint.md) and [Interop Blueprint](../../docs/Interop_Blueprint.md), and this crate’s `IDB.md`, `CONFIG.md`, and `CONCURRENCY.md`.

`ron-metrics` is a **library** that can host an optional, minimal HTTP exposer for `/metrics`, `/healthz`, and `/readyz`.  
It **does not** implement application-level auth; deployments should rely on **network boundaries** (loopback/UDS) and/or a **front proxy** (gateway) for access control.

---

## 1) Threat Model (STRIDE)

| Category | Threats | Relevant? | Mitigation (this crate or host) |
|---|---|---:|---|
| **S**poofing | Fake scrapers, spoofed localhost clients | **Yes (low)** | Bind loopback/UDS by default; optional TLS (`tokio_rustls`); if exposed, put behind gateway auth (macaroons/OIDC). |
| **T**ampering | Metrics text mutation, request smuggling | **Yes (low)** | Axum/hyper; `GET` only; deny bodies & upgrades; no admin endpoints; immutable registry (single registration). |
| **R**epudiation | No audit trail for access/errors | **Yes** | Structured JSON logs (`service`,`peer_addr`,`user_agent`,`corr_id`); counters for rejects/restarts; time-bounded clocks. |
| **I**nfo Disclosure | Secrets/PII in labels/logs | **Yes (med)** | Forbid secrets/PII in labels; `labels.extra` allowlisted; amnesia mode (no persistence); never log secrets; optional label sanitizer. |
| **D**oS | Flood/slow-loris; cardinality bombs | **Yes (med)** | Tight timeouts (read/write/idle); host caps concurrency/RPS; loopback/UDS; reject bodies; cardinality guardrails; backpressure + Busy. |
| **E**oP | Using exposer to escalate | **No (by design)** | Read-only surface; no mutating endpoints; no dynamic code paths; TLS config restricted to `tokio_rustls::rustls::ServerConfig`. |
| **PQ/ZK** (forward-proofing) | PQ hybrid KEX failure storms; ZK verify abuse | **Yes (low)** | Hot-path counters are atomic & lock-free; expose `pq_*_failures_total`, `zk_*_failures_total`; no retries; rate-limit at gateway if public. |

> Notes: The exposer is **read-only** and intentionally minimal. Public exposure must pass a gateway policy review.

---

## 2) Security Boundaries

- **Inbound (surface):**
  - `GET /metrics` — Prometheus text, version 0.
  - `GET /healthz` — liveness.
  - `GET /readyz` — readiness (may return small JSON with degraded reasons).
  - **Transport:** TCP loopback **or** **UDS** (preferred). Optional TLS for TCP.
  - **Accepted methods:** **GET only**. Return `405` for others; **reject any request body**.

- **Outbound (egress):**
  - None. This crate **does not** dial out.

- **Trust Zone:**
  - Same tenant/namespace as host service; not for direct Internet without a gateway.

- **Assumptions:**
  - Kernel invariants hold (Bus/Config/Supervisor/Health).
  - Host services set quotas & backpressure and own ingress security.
  - File system paths for UDS/TLS are per-service and not world-readable/symlinked.

---

## 3) Key & Credential Handling

- **Handled by this crate (optional):** TLS certificate + private key (PEM) for the metrics listener.
- **Storage & access:**
  - Host provides paths; `CONFIG.md` validation enforces existence and perms.
  - **Amnesia mode:** no persistence; keys live only in memory for process lifetime.
  - **Symlink safety:** refuse symlinked key paths (`symlink_metadata()` checks).
- **Lifetime & zeroization:**
  - Keep only as required by `tokio_rustls::rustls::ServerConfig`.
  - Use `zeroize` for transient buffers; never log secrets or cert subjects.
- **Rotation (ops policy):**
  - Rotate TLS materials **≤ 30 days**. Reload requires **rebind** (disruptive); emit `ConfigUpdated` and drain within deadline.

> Non-goals: This crate does **not** manage macaroons/API keys/DB creds. Add auth in a **front proxy** or host middleware if required.

---

## 4) Hardening Checklist (crate-scoped)

- [ ] **Bind scope:** default `127.0.0.1:0` **or UDS**; public binds require review.
- [ ] **Methods:** allow only `GET`; reject bodies/upgrades (WS, h2 prior-knowledge).
- [ ] **Timeouts:** `read=2s`, `write=2s`, `idle=15s` (see `CONFIG.md`).
- [ ] **Concurrency cap:** host caps inflight (e.g., 256–512) via listener limits/semaphores.
- [ ] **RPS cap:** enforced upstream (gateway/sidecar/WAF).
- [ ] **Compression:** inbound decompression **disabled** (no bodies). If enabling response gzip, bound size & fuzz `Accept-Encoding`.
- [ ] **Cardinality guardrails:** only static labels (`service`,`instance`,`build_version`,`amnesia`, optional low-cardinality `extra`). No per-request/user IDs.
- [ ] **UDS security:** dir `0700`, sock `0600`, no symlinks; consumers should use **SO_PEERCRED** allowlists at host.
- [ ] **TLS type invariant:** only `tokio_rustls::rustls::ServerConfig`; never raw `rustls::ServerConfig`.
- [ ] **Hashing:** only **BLAKE3** for incidental hashing (per IDB anti-scope).
- [ ] **Chaos test:** restart under scrape; `/readyz` flips to 503 before exit; `service_restarts_total` increments on return.
- [ ] **Headers:** send `Cache-Control: no-store` and `X-Content-Type-Options: nosniff` on all endpoints.

---

## 5) Observability for Security

- **Security-relevant metrics** (some owned by host):
  - `rejected_total{reason="method"|"body"|"oversize"|"cardinality"|"unauth"}` (host/gateway).
  - `tls_handshake_failures_total`.
  - `io_timeouts_total{op="read"|"write"}`.
  - `service_restarts_total{service}` (supervisor).
  - `bus_lagged_total` (broadcast lag observed).
  - **PQ/ZK:** `pq_kex_failures_total{algo,role}`, `pq_verify_failures_total{algo,role}`, `zk_verify_failures_total{scheme}` (families exist even if zero).
- **Logs:** structured JSON with `ts`,`level`,`service`,`endpoint`,`status`,`reason`,`peer_addr`,`user_agent`,`corr_id`. Never log secrets/headers.
- **Health gates:** `/readyz` **fail-closed** on degraded state (invalid config, bind/TLS errors, repeated handshake failures). `/healthz` may stay OK for liveness.

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates:** `tokio`, `hyper`, `axum`, `tokio-rustls`/`rustls`, `prometheus`, `serde`, `once_cell`, `zeroize`, `blake3`.
- **Pinning & policy:** versions pinned at workspace root; MSRV per front-matter.
- **CI controls:**
  - `cargo-deny` (bans, licenses, advisories) — **required**.
  - `cargo audit` (RUSTSEC) on every PR + nightly schedule — **required**.
  - SBOM (CycloneDX or `cargo auditable`) on release → `/docs/sbom/` — **required**.
  - Provenance (SLSA attestations) — **recommended** at pipeline level.
- **Procedure:** A critical advisory → pin patch; if no patch, vendor temporary deny + mitigation note in `SECURITY.md` and `CHANGELOG.md`.

---

## 7) Formal & Destructive Validation

- **Property tests:** reject non-GET or requests with bodies; enforce method whitelist; assert timeouts observed; verify label cardinality stays ≤ configured allowlist.
- **Fuzzing (negative paths):** if enabling response compression, fuzz `Accept-Encoding`, content negotiation, and header size; fuzz label sanitizer (cardinality bomb defense).
- **Loom tests:** shutdown order; no missed notification or deadlock (see `CONCURRENCY.md`).
- **Chaos tests:** kill/restart loops under steady scrape (1–5 rps); `/readyz` degrades before exit; restart counter increments.
- **TLA+:** host health DAG lives outside this crate; not required here.

---

## 8) Data Classification & Privacy

- **PII/Secrets policy:** No PII or secrets in metrics/labels or logs.  
  `labels.extra` must be code-reviewed; allowed examples: `region`, `az`, `tier` (low cardinality).
- **Build metadata:** `build_version` in metrics is intentional and **not** sensitive.
- **Traces/Exemplars (optional):** If OTEL/exemplars enabled, ensure IDs are opaque and non-identifying; forbid URLs/emails/tokens.

---

## 9) Migration & Upgrades

- Changes to **TLS/auth posture** (e.g., mandating TLS, adding auth hooks) are **breaking** → require a **major** bump and migration notes in `CHANGELOG.md`.
- Key path renames require deprecation aliases ≥ 1 minor with warnings.
- Exporter changes (Prometheus ↔ OTEL) are disruptive operations; roll out behind readiness checks.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client/Scraper] -->|TCP/UDS; GET /metrics| B(ron-metrics exposer)
  B -->|200 text/plain| A
  B -.->|405/400/503 (reject)| A
  B -->|Counters & JSON logs| M[(Security Telemetry)]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text:** Clients issue `GET` over loopback/UDS (or TLS). Exposer returns metrics/health or rejects unsafe requests. Security events emit metrics/logs.

---

## 11) Appendix — Operational Guardrails

* **Network placement:** keep `/metrics` on **127.0.0.1** or **UDS**; for cluster exposure, use **cluster-local** addresses only.
* **Gateway policy (if public):** require auth, IP allowlists, and rate limits; terminate TLS at gateway or use end-to-end TLS.
* **Headers:** set `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`. Do not enable permissive CORS.
* **HTTP versions:** prefer HTTP/1.1; disable protocol upgrades and websockets.
* **Symlink safety:** refuse symlinked paths when creating sockets/reading keys; avoid TOCTOU via atomic create.

## 12) Security Contacts

* **Maintainer:** Stevan White (see CODEOWNERS)
* **Backup:** Observability Team (org directory)
* **Disclosure policy:** See repository root `SECURITY.md`. Responsible disclosure encouraged.

```
---
```
