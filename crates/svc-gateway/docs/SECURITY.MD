

````markdown
---
title: Security Notes — svc-gateway
crate: svc-gateway
owner: Stevan White
last-reviewed: 2025-10-02
status: draft
---

# Security Documentation — svc-gateway

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-gateway`.  
It complements the repo-wide Hardening Blueprint and Interop Blueprint.

---

## 1) Threat Model (STRIDE)

| Category | Threats | Relevant in `svc-gateway`? | Mitigation |
|---|---|---:|---|
| **S**poofing | Fake identities, unauthenticated peers, forged `X-Forwarded-*` | **Y** | TLS 1.3 (tokio-rustls); capability tokens (macaroons); strict `Host` validation; canonicalize/whitelist forwarded headers; optional mTLS in private zones |
| **T**ampering | Mutated requests, header smuggling, TE/CL desync, path normalization attacks | **Y** | HTTP/1.1+2 only, no h2c; strip hop-by-hop headers; reject TE/CL conflicts; normalized path; fixed error taxonomy; integrity headers preserved (`ETag="b3:<hex>"`) |
| **R**epudiation | Missing/weak audit trails, unverifiable actions | **Y** | Structured JSON logs (req/resp), `corr_id`/trace IDs, tenant ID (if present), hashed token IDs, time-sync; immutable log stream |
| **I**nformation Disclosure | PII leakage in logs, verbose errors, token/key exposure | **Y** | Amnesia mode; redaction layer; no secret logging; uniform errors; size-bounded traces; zeroize secrets in memory |
| **D**enial of Service | Floods, slow-loris, connection churn, decompression bombs | **Y** | Hard timeouts (read/write 5s), concurrency cap (≤512), RPS cap (≤500/inst), body cap (1 MiB), decompress ratio ≤10:1 + absolute 8 MiB; DRR + quotas; fail-fast 429/503 |
| **E**levation of Privilege | Capability bypass, confused deputy, path overreach | **Y** | Capability tokens (macaroons) with caveats (route/verb/tenant/TTL); default-deny routes; no ambient auth; strict method allow-list |

Additional web-specific risks & stances:

- **Request smuggling:** deny TE/CL conflicts; single framing source of truth; disable `Transfer-Encoding` other than `chunked`; no proxy-in-proxy ambiguity.
- **CORS:** default **deny**; allow only explicit origins/methods/headers per route (if gateway must serve browsers).
- **Clickjacking:** not applicable (API), but may set `X-Frame-Options: DENY` on any accidental HTML paths.
- **HSTS:** optional if terminating TLS for browser clients.

---

## 2) Security Boundaries

- **Inbound (public)**  
  - `TLS :bind_addr` → `GET /readyz`, `GET /healthz`, `GET /metrics` (metrics usually bound to localhost)  
  - `POST/PUT/PATCH/DELETE /api/*` (capability-gated)  
  - `GET /api/*` (read-mostly; sheds writes first under pressure)  
  - `GET /version` (build info)  

- **Outbound (trusted internal)**  
  - `omnigate` (hydration/BFF), or directly to internal services (e.g., storage/index/mailbox) over mTLS or mesh.  
  - Metrics sink (Prometheus scrape).  
  - Optional: control bus (read-only) for config updates.

- **Trust Zone**  
  - `svc-gateway` is **public-facing** ingress. Treat all inbound as untrusted. Outbound into a **trusted service mesh** (same tenant or cluster).

- **Assumptions**  
  - Kernel invariants hold (Bus, Metrics, Health, Config).  
  - Downstream services enforce their own `/readyz` and quotas.  
  - Filesystem is namespaced per service; no shared world-writable dirs.  
  - Time is roughly in sync (for capability TTLs).

---

## 3) Key & Credential Handling

- **Key types**
  - **TLS**: server certificate/private key (PEM).  
  - **Macaroon root/verification keys**: for minting/validating capability tokens (or verification-only if minted elsewhere).  
  - **Optional mTLS CA**: to validate internal clients in private zones.

- **Storage & loading**
  - From disk paths in config (0600 perms); **never** from CLI args.  
  - In **amnesia mode**, secrets are **memory-only** and not persisted.  
  - In-memory secrets wrapped with `zeroize::Zeroizing` and dropped on shutdown.

- **Rotation**
  - **TLS**: rotate ≤ 30 days or per corporate policy; support SIGHUP reload.  
  - **Macaroons**: rotate root key ≤ 30 days; support **key ID (kid)** versioning and **grace window** for dual-key validation.  
  - **Revocation**: deny-list by token ID (hash) for high-risk incidents; prefer short TTL + narrow caveats to minimize blast radius.

- **Usage discipline**
  - Do not log tokens or keys; log only a stable **hash prefix** for token ID correlation.  
  - Avoid sharing the same root key across environments/tenants.

---

## 4) Hardening Checklist (from Blueprint + svc-gateway specifics)

- [ ] **Ingress caps**: read=5s, write=5s, idle=60s; **max inflight=512**; **RPS=500/inst** (tunable with safety guard).  
- [ ] **Body cap**: 1 MiB; streaming reads in ~64 KiB chunks.  
- [ ] **Decompression guard**: ratio ≤10:1 and **absolute decoded cap ≤8 MiB**; reject 413 on breach.  
- [ ] **Framing safety**: disable TE/CL conflicts; reject unknown `Transfer-Encoding`; strip hop-by-hop headers (`Connection`, `Upgrade`, etc.).  
- [ ] **Header limits**: max header count & bytes; reject oversize with 431.  
- [ ] **Method/route allow-list**: default-deny; OPTIONS only if explicitly allowed.  
- [ ] **Host/authority validation**: enforce expected `Host`/SNI; drop absolute-form anomalies.  
- [ ] **Forwarded headers**: canonicalize `X-Forwarded-For` / `Forwarded`; never trust client-supplied chain without proxy trust config.  
- [ ] **UDS**: dir `0700`, socket `0600`, enforce `SO_PEERCRED` allow-list.  
- [ ] **TLS 1.3 only**: modern cipher suites; disable renegotiation; ALPN `h2,http/1.1` only; no h2c.  
- [ ] **Capability checks**: per-route/verb/tenant; enforce TTL and caveats before any downstream I/O.  
- [ ] **Amnesia mode**: RAM-only buffers/logs; redact PII; zeroize secrets.  
- [ ] **Chaos**: restart under load; `/readyz` flips correctly; writes shed first.

---

## 5) Observability for Security

- **Metrics**
  - `rejected_total{reason="unauth"|"quota"|"body_cap"|"decoded-cap"|"decoded-ratio"|"method"|"host"}`  
  - `auth_failures_total{service="svc-gateway"}`  
  - `tls_handshake_failures_total`  
  - `busy_rejections_total{endpoint}`  
  - `gateway_degraded{bool}` (1 when shedding writes)

- **Logs (JSON, structured)**
  - Fields: `ts`, `level`, `service`, `route`, `method`, `status`, `reason`, `peer_addr`, `tenant`, `corr_id`, `span_id`, `req_bytes`, `resp_bytes`, `latency_ms`.  
  - **Never** log tokens/keys; log token **hash prefix** (e.g., `tok=ab12…`).  
  - Error taxonomy is deterministic: 400/401/403/413/415/429/503 only for edge failures.

- **Health gates**
  - `/readyz` fail-closed when degraded; `/healthz` remains basic liveness; `/metrics` always best-effort.

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates**
  - `tokio-rustls` (TLS 1.3), `axum`/`hyper`/`tower-http` (HTTP stack), `serde` (strict DTOs), `zeroize` (secret memory), `prometheus` (metrics).  
- **Pinning & policy**
  - Versions are **pinned at workspace root**; deny wildcard deps.  
  - `cargo-deny` in CI (licenses, bans, advisories, sources).  
  - Reproducible builds where possible; vendor lockfiles.  
- **SBOM**
  - Generate at release; store under `docs/sbom/`; include build metadata and hashes.

---

## 7) Formal & Destructive Validation

- **Property tests**:  
  - Capability validation (TTL/caveats), error taxonomy mapping, header/path normalization, TE/CL conflict handling.  
- **Fuzzing**:  
  - HTTP request parser paths and body/decompression streams (chunked + gzip/deflate/br), boundary values (1 MiB cap ±1).  
- **Loom (dev)**:  
  - Producer→bounded queue→consumer under shutdown; show no deadlock or lost shutdown.  
- **Chaos**:  
  - Kill worker tasks at random under load; confirm `/readyz` degraded state; verify drain deadline and abort accounting.  
- **Perf/Safety gates**:  
  - p95 latency baseline lock; decompression bomb (100 KiB → 12 MiB) must 413 without OOM.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- *  
- **Disclosure policy:** see repo root `SECURITY.md` (responsible disclosure; 90-day public window unless otherwise agreed).

---

## 9) Migration & Upgrades

- **Breaking changes** (auth format, capability model, TLS/mTLS expectations) require a **major version bump** and explicit migration steps in `CHANGELOG.md`.  
- **Deprecations** must include: overlap window, config aliases, automated conversion notes.  
- **Key rotation**: document cutover schedule; support dual-key validation period.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + macaroon| B(svc-gateway)
  B -->|cap checks + caps/limits| B
  B -->|validated request| C[Downstream Service]
  B -.->|reject unauth/oversized/degraded| D[Error + Metrics + JSON log]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text description:** Client connects over TLS 1.3 and presents a capability token. `svc-gateway` enforces caps/limits, validates capabilities, and either forwards the request to a downstream service or rejects deterministically while emitting security metrics and structured logs.

```
```
