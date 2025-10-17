title: Security Notes — svc-edge
crate: svc-edge
owner: Stevan White
last-reviewed: 2025-10-17
status: draft

# Security Documentation — svc-edge

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-edge`.
It complements the repo-wide Hardening and Interop blueprints, and aligns with `docs/IDB.md` and `docs/CONFIG.md`.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats                                                            | Relevant in `svc-edge`? | Mitigation (normative)                                                                                                                                                                                                         |
| -------------------------- | ------------------------------------------------------------------ | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **S**poofing               | Client identity spoofing, forged upstream origin on live fills     | **Y**                   | TLS 1.3 termination (gateway or rustls); deny-by-default CORS; allow-list for upstream hosts; optional capability tokens (macaroons) on protected paths; log `peer_addr` + `corr_id`.                                          |
| **T**ampering              | Response/asset corruption, cache poisoning, PMTiles/MBTiles tamper | **Y**                   | **Strong ETag** = BLAKE3 of bytes; CAS content-addressing; pack verification at mount; conditional requests validated; immutable headers; no speculative writes.                                                               |
| **R**epudiation            | Disputed actions (fills, rejects) lacking audit trail              | **Y**                   | Structured JSON logs; stable `corr_id` propagation; optional **audit hook** for live fills/rejects (records source host, content hash, size).                                                                                  |
| **I**nformation Disclosure | Cross-origin leakage, verbose error details, secret/key exposure   | **Y**                   | Deny-by-default CORS; `Referrer-Policy: no-referrer`; redact errors; never log secrets/macaroons; amnesia mode blocks persistence; minimal headers on 4xx/5xx.                                                                 |
| **D**enial of Service      | Slowloris, flood, compression bombs, range abuse                   | **Y**                   | Global timeout (≤5s), inflight cap (≤512), RPS cap (≤500), body cap (1 MiB), **decompress ratio ≤10× + absolute cap 10 MiB**, `Range` validation, bounded queues (`mpsc(512)`), `try_send` + 429 shed, retry budget w/ jitter. |
| **E**levation of Privilege | Anonymous access to protected paths, bypassing policy              | **Y**                   | No ambient trust; protected routes require capability verification upstream; allow-list enforcement for live egress; no token minting here (delegated to auth service).                                                        |

**Assumptions:** TLS termination may occur at an ingress gateway; when termination happens in-process, `tokio-rustls` is used exclusively.

---

## 2) Security Boundaries

* **Inbound (externally reachable):**

  * `GET /edge/assets/{*path}` (public assets; governed by caps)
  * `GET /healthz`, `GET /readyz`, `GET /metrics` (ops-only; prefer localhost or gateway-auth scrape)
* **Inbound (internal/control):**

  * SIGHUP / bus `KernelEvent::ConfigUpdated` (config reload)
* **Outbound:**

  * HTTPS to **allow-listed** upstreams for live fills (e.g., fonts.gstatic.com, api.maptiler.com)
  * Local filesystem **read-only** for packs; CAS writes (respect amnesia)
* **Trust Zone:**

  * **Public-facing** on `/edge/assets/*` (zero trust; quotas apply)
  * Metrics/health endpoints are **operator** trust zone (bind to loopback or protect at gateway)
* **Assumptions:**

  * Global kernel invariants (metrics/health/config) hold.
  * Downstream resources expose their own `/readyz` + quotas.
  * Per-service filesystem isolation; read-only mounts for packs; no shared writable volumes.

---

## 3) Key & Credential Handling

* **Key/secret types:**

  * TLS certificate/key (if terminating TLS locally)
  * Optional **macaroon**/capability tokens for protected subtrees (validated upstream; not minted here)
  * Optional audit signing key (if audit hook emits signed records)
* **Storage:**

  * Disk paths only when **amnesia=false** and with strict perms (`0600`); never log contents.
  * In **amnesia=true**, secrets must remain in-memory only; attempting to persist is a violation (reject & degrade readiness).
* **Rotation:**

  * Keys/tokens rotated ≤ 30 days (policy-level); process SIGHUP or hot-reload to apply.
  * Revocation: deny-list specific macaroons/tokens at gateway/policy layer; reject on sight.
* **Zeroization:**

  * Wrap sensitive materials with `zeroize::Zeroizing`; drop promptly; avoid long-lived buffers.

---

## 4) Hardening Checklist (from Blueprint)

* [x] Request timeout ≤ **5s** (listener + upstream fills).
* [x] Concurrency cap: **512 inflight**.
* [x] RPS cap: **500/instance** (token-bucket).
* [x] Request body cap: **1 MiB**.
* [x] Decompression caps: ratio ≤ **10×** and **absolute 10 MiB**.
* [x] Range requests strictly validated; **multi-range disabled** by default.
* [x] Public endpoints set **HSTS** (when TLS), **Referrer-Policy: no-referrer**; CORS default deny.
* [x] UDS (if used): dir `0700`, socket `0600`, `SO_PEERCRED` allowlist.
* [x] No blocking syscalls on async workers; use `spawn_blocking` for heavy hashing if profiling demands.
* [x] Chaos test under load: restart workers; `/readyz` flips degraded during shed; recovers predictably.
* [x] Amnesia: **no disk writes**; violations reject + degrade readiness.

---

## 5) Observability for Security

* **Metrics (required):**

  * `edge_rejects_total{reason="amnesia"|"not_allowed"|"body_cap"|"decompress_cap"|"ratio_cap"|"timeout"|"rate_limit"|"invalid_range"|"etag_invalid"}`
  * `busy_rejections_total{endpoint}` (backpressure)
  * `io_timeouts_total{op="read"|"write"|"connect"|"fill"}`
  * `backoff_retries_total{op="live_fill"}`
  * `tls_handshake_failures_total` (if TLS local)
* **Logs (JSON fields):** `ts`, `level`, `service="svc-edge"`, `corr_id`, `peer_addr`, `route`, `status`, `reason`, `duration_ms`, `bytes`, `etag`, `hash`, `mode` (`offline|live`).

  * **Do not** log macaroon contents or secrets.
* **Health gates:** `/readyz` is **fail-closed** when quotas tripping, packs not verified, CAS unreachable, or amnesia constraint would be violated.

---

## 6) Dependencies & Supply Chain

* **Security-sensitive crates:**

  * `tokio-rustls` (TLS 1.3 only; system trust roots or pinned bundle)
  * `axum`/`tower` (middleware ordering enforces caps)
  * `reqwest`/`hyper` (HTTPS client for live fills; `rustls` backend)
  * `blake3` (content hashing for strong ETag + CAS keys)
  * `serde` (`deny_unknown_fields` for any DTOs)
  * `zstd`/`flate2` (decompression with ratio/absolute caps applied at middleware)
* **Controls:**

  * Pin versions at workspace root; **`cargo-deny`** for advisories/licenses/bans.
  * Reproducible builds in CI; release SBOM to `docs/sbom/`.
  * Disallow `native-tls`; prefer `rustls` exclusively.

---

## 7) Formal & Destructive Validation

* **Property tests:** header parser bounds (`Range`, `If-None-Match`, `Accept-Encoding`), size/ratio/absolute caps.
* **Fuzzing:** HTTP header parsing (esp. `Range`), gzip/zstd headers; reject malformed/chunked edge cases deterministically.
* **Loom:** producer → bounded `mpsc(2)` → consumer + shutdown; assert no deadlocks, proper cancellation.
* **Chaos:** kill N/2 workers under load; verify `/readyz` transitions and metrics (shed + retries).
* **Perf gates:** warm-hit p95 < 40 ms local; fail build if exceeded.
* **(Optional) Audit mode:** verify audit record emitted for each accepted live fill (hash/host/size) and for each reject with reason.

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Backup:** Core RustyOnions Security Group (on-call rotation)
* **Disclosure policy:** See repository `SECURITY.md` (coordinated disclosure; PGP preferred).

---

## 9) Migration & Upgrades

* **Auth/keys changes** (formats, validation rules, header contracts) require a **major version bump** and a migration guide in `CHANGELOG.md`.
* **Deprecated configs** (e.g., enabling multi-range) must provide a safe default and sunset date; warn on use for ≥1 minor release.
* **Switching hashing** (e.g., BLAKE3 → SHA-256) requires dual-publish window and compatibility note.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS (gw) + optional macaroon| B(svc-edge)
  B -->|pack/CAS hit| A
  B -->|allow-list check| U[Upstream HTTPS]
  U -->|200 + ETag| B
  B -->|BLAKE3 verify + CAS write| A
  B -.->|Reject unauth/oversized/not_allowed| D[Error + Metrics + Audit?]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text:** Client requests go through TLS (at gateway or locally). For misses in offline/live mode, svc-edge enforces allow-list and hard caps before contacting upstream. Successful fills are hashed (BLAKE3) and written to CAS with strong ETag. Violations (unauthorized, oversize, not-allowed, cap exceeded) are rejected and recorded in metrics (and optional audit stream).

---

## 11) Anti-Scope (to avoid security drift)

* No token minting, no user/session auth flows (handled upstream).
* No cache invalidation API here; no DHT/discovery; no writeable shared storage in amnesia mode.
* No unbounded queues or background tasks without join handles and shutdown paths.

---

## 12) Operational Security Notes

* Bind `/metrics` to localhost or secure behind gateway auth; never expose publicly.
* Run as **non-root**; drop ambient capabilities; read-only root FS where possible; mount packs read-only.
* Configure **resource limits** (ulimits for FDs; cgroup CPU/memory) to match caps.
* Prefer **image digests** (pinned) in deployments; enable admission controls (e.g., disallow :latest).
* Keep **allow-list** minimal; treat additions as change-controlled events.

---
