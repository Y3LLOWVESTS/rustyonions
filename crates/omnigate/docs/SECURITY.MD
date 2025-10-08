---

title: Security Notes — omnigate
crate: omnigate (impl: svc-gateway)
owner: Stevan White
last-reviewed: 2025-10-07
status: draft
-------------

# Security Documentation — omnigate

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `omnigate` (implemented by `svc-gateway`).
It complements the repo-wide Hardening, Interop, and IDB blueprints.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats (edge-specific)                                               | Relevant here? | Primary Mitigations (must)                                                                                                                                                                              |
| -------------------------- | --------------------------------------------------------------------- | :------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Client identity spoofing; header forgery; service-to-service spoofing |      **Y**     | TLS 1.3 (tokio-rustls); SNI and cert chain validation; **capability tokens (macaroons)** verified via `svc-passport`; no ambient trust; optional UDS with **SO_PEERCRED** allowlist for local callers   |
| **T**ampering              | Request/body mutation; replay; response injection                     |      **Y**     | TLS integrity; content addressing (BLAKE3) when brokering CAS; idempotent routes; `etag`/`if-match` pass-through; canonical decoding (decompress caps)                                                  |
| **R**epudiation            | Unverifiable actions; missing audit trails                            |      **Y**     | Structured JSON logs with `corr_id`/`req_id`/`peer_addr`; signed capability claims (issuer traceable); clock sync; immutable metrics                                                                    |
| **I**nformation Disclosure | Token/key/PII leakage; side channels                                  |      **Y**     | Redaction (never log secrets); **amnesia mode** (RAM-only); strict CORS (if enabled by host), TLS only; constant-time comparisons for auth                                                              |
| **D**enial of Service**    | Floods; Slowloris; decompression bombs; connection exhaustion         |      **Y**     | **RPS quotas** (global/per-IP/per-cap); **DRR fair-queue**; `max_body_bytes=1MiB`; **decompress_ratio_cap=10×**; `read/write/idle` timeouts; `max_conns` cap; early admission-control; shed via 429/503 |
| **E**levation of Privilege | Using anon routes to reach privileged ops; confused deputy            |      **Y**     | Strict route classification (anon/authed/internal); macaroon caveats (scoped capabilities, TTLs); deny by default; separate class weights in DRR; no proxying of auth-bypassed hops                     |

> Scope note: omnigate is **public-facing edge**. Treat all inbound as hostile unless presented with a valid capability token issued by `svc-passport`. No trust is inherited from network location.

---

## 2) Security Boundaries

* **Inbound (public HTTP/TLS):**

  * **Public app APIs** (role: *broker/hydrator*, not owner of business logic)
  * `GET /healthz` (safe), `GET /readyz` (safe), `GET /metrics` (**private**; bind to localhost or restricted interface)
* **Outbound (to trusted internal services):**

  * `svc-passport` (verify caps / JWKS)
  * `svc-index`, `svc-storage`, `svc-mailbox`, `svc-overlay`, `svc-dht` (hydration/lookup)
* **Trust zone:** Runs in **edge zone**; talks to **internal zone** over mTLS or private network (recommended). `/metrics` **must not** be Internet-exposed.
* **Assumptions:**

  * Kernel primitives (Bus, Metrics, Health, Config watcher) are correct and **fail-closed**.
  * Downstreams enforce their own quotas and `/readyz`.
  * No direct DB mounts; any disk IO is for certs only (and avoided when PEMs are env-injected).
  * Time is sane (NTP) for token TTL validation.

---

## 3) Key & Credential Handling

* **Key/secret types**

  * **TLS server keypair** (PEM).
  * **Capability verification material** (Passport JWKS/issuer certs).
  * Optional: PQ-hybrid KEM policy flags (not long-term keys here; negotiated per session).
* **Acquisition & storage**

  * Prefer **inline PEM via env** (`OMNIGATE_TLS_CERT_PEM` / `OMNIGATE_TLS_KEY_PEM`) delivered by a secret manager; fallback to file paths with restrictive perms (**0600** key).
  * **Amnesia mode**: no on-disk persistence of secrets; in-memory only.
* **Rotation**

  * TLS: ≤ **30 days** recommended; hot-reload via config watcher; overlap old/new (two-cert rustls config acceptable).
  * Passport JWKS: cache with short TTL (e.g., 5–15 min); refresh in background; fail-closed on invalid signatures.
* **Zeroization**

  * Wrap inline PEMs in `zeroize::Zeroizing` or wipe buffers after rustls config is built.
  * Never log secrets; redact `Authorization` or custom macaroon headers entirely.
* **Validation**

  * On boot/reload: load + parse PEM, verify chain (if used), reject weak key sizes/ciphers; JWKS must be reachable (or serve from last good cache; mark degraded).

---

## 4) Hardening Checklist (must pass before ship)

* [ ] **Ingress caps**: `max_conns >= 1`, **RPS** caps set (global/per-IP/per-cap), **DRR** weights configured (`anon`, `authed`, `internal`).
* [ ] **Timeouts**: `read=5s`, `write=5s`, `idle=60s` (or stricter).
* [ ] **Body**: `max_body_bytes = 1MiB` (never higher unless justified); **decompress_ratio_cap = 10×** with absolute byte ceiling.
* [ ] **TLS**: rustls only; no raw `openssl`. Keys via env PEM preferred; files have **0600** key perms.
* [ ] **UDS (if used)**: dir `0700`, socket `0600`, enforce **SO_PEERCRED** allowlist.
* [ ] **Auth**: `auth.mode=passport` in prod; tokens must be **scoped** and **short-TTL**.
* [ ] **Logs**: JSON; secrets redacted; include `corr_id`, `peer_addr`, `class` (anon/authed/internal).
* [ ] **/metrics** restricted to localhost or private VLAN; **/readyz** flips to degraded on downstream failure.
* [ ] **Chaos**: restart under load; verify graceful drain and backpressure metrics.
* [ ] **No unsafe code** in this crate; enable lints that would block accidental inclusion.

---

## 5) Observability for Security

* **Counters (examples):**

  * `rejected_total{reason="unauth"|"quota"|"body_cap"|"decompress_cap"|"ratio_cap"|"degraded"}`
  * `auth_failures_total{issuer="passport"}`
  * `tls_handshake_failures_total{reason}`
  * `bus_lagged_total` (slow consumers imply degraded telemetry)
* **Gauges/Histograms:**

  * `fq_inflight{class}`
  * `downstream_latency_seconds{service}` (histogram)
* **Log fields (required):**

  * `service`, `corr_id`, `req_id`, `peer_addr`, `route`, `class`, `decision` (allow/deny), `reason`, `duration_ms`
  * **Never** log tokens or full URLs with secrets.
* **Health/Readiness:**

  * `/healthz` returns OK if process alive.
  * `/readyz` **fail-closed** when: TLS invalid; config reload failed; downstream strict-boot fail; or `fq_inflight` saturates.

---

## 6) Dependencies & Supply Chain

* **Security-sensitive crates:**

  * `tokio-rustls` (TLS 1.3) — pinned at workspace root; prefer modern cipher suites.
  * `axum`/`hyper` — HTTP stack; keep features minimal; disable HTTP/2 cleartext.
  * `serde` (+`deny_unknown_fields` on config DTOs).
  * `base64`, `sha2`, `hex` (interop hygiene), `blake3` (CAS where relevant).
* **Controls:**

  * `cargo-deny` (licenses/bans/advisories) green in CI.
  * **SBOM** emitted on release; store under `docs/sbom/`; sign artifacts.
  * Reproducible builds target; lockfile committed; forbid `git` deps for runtime path.
  * `#![forbid(unsafe_code)]` at crate level (unless unavoidable and justified).

---

## 7) Formal & Destructive Validation

* **Property tests**

  * Admission: `work_full ⇒ Busy` (no hidden buffering).
  * Decompression: never exceed **ratio cap** nor absolute byte ceiling; reject ambiguous encodings.
  * Token checks: reject expired/not-yet-valid; enforce caveats (scope/audience).
* **Fuzzing**

  * HTTP header parsers, content-encoding decoders, small OAP/bridge components. Seed with boundary cases (1, 1024, 1MiB).
* **Loom (dev-only)**

  * Producer/consumer with shutdown; assert no deadlocks, no missed shutdown, bounded queues.
* **Chaos / Soak**

  * Kill workers under load; verify `/readyz` flips, then recovers; verify **no token leakage** in logs on errors.
* **TLA+ (optional)**

  * DRR fairness and starvation freedom: for positive weights, every class eventually progresses.

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Backup:** (add reviewer/rotation)
* **Disclosure policy:** See root `SECURITY.md` (coordinated disclosure; PGP key if applicable).

---

## 9) Migration & Upgrades

* Changes to **auth mode**, token format, or TLS policy are **security-relevant** and require:

  * CHANGELOG entry with **migration steps**,
  * SemVer **major** if breaking,
  * Feature flag or staged rollout (dual-stack acceptor where possible).
* Deprecate old env keys with aliases for ≥1 minor; emit warnings; document removal window.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3| T[Terminator (rustls)]
  T -->|Admission: quotas + DRR| ADM{Admit?}
  ADM -- No --> REJ[429/400 Reject + Metrics]
  ADM -- Yes --> AUTH[Verify Capability (passport)]
  AUTH -- Fail --> REJ
  AUTH -- OK --> ROUTE[Route/Hydrate]
  ROUTE --> DS[(Downstream services)]
  REJ -.-> MET[Security Metrics]
  DS -.-> MET
  style T fill:#0b3a2e,stroke:#064e3b,color:#fff
  style REJ fill:#7f1d1d,stroke:#b91c1c,color:#fff
```

**Text description:** TLS terminates at rustls; requests undergo **admission** (quotas/DRR). If admitted, a capability check is performed. On success, the request is routed/hydrated to internal services. Rejections and downstream outcomes feed security metrics.
