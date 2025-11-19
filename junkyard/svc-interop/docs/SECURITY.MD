
````markdown
---
title: Security Notes — svc-interop
crate: svc-interop
owner: Stevan White
last-reviewed: 2025-10-12
status: draft
---

# Security Documentation — svc-interop

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-interop`.  
It complements the repo-wide Hardening Blueprint and Interop (IDB) docs.

---

## 1) Threat Model (STRIDE)

| Category | Threats (examples) | Relevant in `svc-interop`? | Mitigation (binding controls) |
|---|---|---:|---|
| **S**poofing | Forged webhooks, stolen bearer tokens, DNS/host spoof | **Y** | TLS 1.3 (`tokio-rustls`), origin allow-list, signature verify at edge (HMAC-SHA256 per provider), capability translation (macaroons, TTL ≤ 60s), strict host pinning |
| **T**ampering | Body/content mutation, replay, path param tricks | **Y** | BLAKE3-256 content addressing internally; webhook replay window ≤ 5m with idem keys; `#[serde(deny_unknown_fields)]`; canonical path routing; signed provider descriptors |
| **R**epudiation | “I never sent that” / missing audit evidence | **Y** | Structured JSON logs, audit events `{b3_hash, provider, req_id, decision, reason}`; stable `corr_id` propagation; clock sync requirement |
| **I**nformation Disclosure | PII leakage, token/secret exposure, verbose errors | **Y** | Redaction policy; amnesia mode (no disk); never log secrets; error taxonomy with non-verbose messages; TLS everywhere outbound |
| **D**enial of Service | Flooding, slowloris, decompression bombs, large payloads | **Y** | Body cap 1 MiB, stream >1 MiB at 64 KiB with ≤2 chunks buffered; decompression ratio ≤10× + absolute cap; 5s timeouts; ≤512 inflight; RPS cap 500/instance; early backpressure; brownout gating |
| **E**levation of Privilege | Using external tokens to act internally, scope creep | **Y** | Capability translation via `svc-passport` (scoped, TTL ≤ 60s); policy engine denies unknown origins; no ambient trust; role separation by service boundary |

**Hashing & signatures policy:**  
- External ingress **must** verify provider signatures **as specified by the provider** (HMAC-SHA256 for GitHub/Slack/Stripe); this usage is **transient at the edge only**.  
- All **internal** content addressing, dedup, and audit hashes are **BLAKE3-256**. No SHA-256 digests are persisted or used as internal identifiers.

---

## 2) Security Boundaries

- **Inbound (public-facing):**
  - `POST /webhooks/:provider` (GitHub, Stripe, Slack Webhooks) — signed webhook ingress
  - `POST /put` — content ingress (bounded, streaming rules apply)
  - `GET /o/:addr` — addressed object fetch (read path)
  - Health/metrics: `GET /healthz`, `GET /readyz`, `GET /metrics`, `GET /version` (metrics should bind localhost)
- **Outbound (to trusted zone / same cluster):**
  - `svc-passport` (capability mint/verify)
  - `svc-mailbox` (enqueue/ACK)
  - `svc-storage` / `svc-index` (object IO / lookup)
  - Audit sink (memory/stdout or file if not in amnesia)
- **Trust Zone:** public ingress endpoints; all downstreams are **internal** (same tenant/mesh). No direct DB exposure from `svc-interop`.
- **Assumptions:**
  - Mesh mTLS or TLS for all outbound calls.
  - Downstream services implement their own `/readyz` and quotas.
  - Host/container follows baseline hardening (seccomp/apparmor, non-root, RO FS where possible).

---

## 3) Key & Credential Handling

- **Types of keys/creds:**
  - TLS cert/key (service listener when TLS enabled at app layer; often terminated upstream)
  - Provider webhook signing secrets (GitHub/Stripe/Slack)
  - Capability/KMS materials are **not** held here; obtained via `svc-passport`/`ron-kms`
  - Optional Slack REST bot token
- **Storage:**
  - Provided via **environment variables from a secret manager**; never in repo or CLI flags.
  - In **amnesia mode**: secrets exist only in memory; audit sink must **not** write to disk.
- **Rotation policy:**
  - Provider secrets: rotate ≤ 30 days; support dual secrets during rollout.
  - TLS key/cert: per platform policy; hot-reload via SIGHUP where supported.
  - Capabilities: TTL ≤ 60s; refresh via `svc-passport`.
- **Zeroization:** secrets wrapped in `zeroize::Zeroizing` or cleared on drop; avoid long-lived clones.

---

## 4) Hardening Checklist (from Blueprint)

- [ ] 5s timeout on all requests (read/write); idle 60s.
- [ ] Concurrency cap: 512 inflight (per-instance).
- [ ] RPS cap: 500/instance; burst 250 (token bucket).
- [ ] Request body cap: 1 MiB; >1 MiB must stream at 64 KiB; buffer ≤2 chunks.
- [ ] Decompression ratio ≤ 10× **and** absolute size cap enforced.
- [ ] DTO hygiene: `#[serde(deny_unknown_fields)]`; reject overlong headers.
- [ ] Origin pinning: allow-listed hosts; signed provider descriptors; replay window ≤ 5m.
- [ ] Signature verification: HMAC-SHA256 at edge only; **do not persist SHA digests**; compute **BLAKE3-256** for all internal references.
- [ ] UDS sockets (if used): dir `0700`, socket `0600`, `SO_PEERCRED` allow-list.
- [ ] No blocking syscalls in async tasks; use `spawn_blocking` for CPU-bound/FS work.
- [ ] Chaos test: restart under load; `/readyz` browns out bridges first; read paths stay green.

---

## 5) Observability for Security

- **Metrics (minimum):**
  - `rejected_total{reason="UNAUTH"|"BAD_ORIGIN"|"BODY_LIMIT"|"DECOMP_LIMIT"|"RATE_LIMIT"|"BACKPRESSURE"|"POLICY_BLOCKED"}`
  - `auth_failures_total{service="svc-interop"}`
  - `tls_handshake_failures_total`
  - `audit_drop_total` (when audit queue sheds)
  - `brownout_active{component="bridge"}`
- **Logs (structured JSON):**
  - Fields: `ts`, `service`, `corr_id`, `peer_addr`, `method`, `path`, `status`, `reason`, `provider`, `req_id`, `latency_ms`
  - **Never** include raw secrets or full tokens. Redact headers like `Authorization`, `Stripe-Signature`, `X-Slack-Signature`.
- **Health gates:**
  - `/readyz` degrades **bridging** on error-rate/p95 breaches or queue saturation; read-only endpoints can remain ready if safe.
  - `/healthz` reflects fatal conditions (e.g., crash-loop, config invalid).

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates:**
  - `tokio`, `axum`, `tower`, `tower-http` (timeouts/limits/middleware)
  - `tokio-rustls` (TLS 1.3)
  - `serde` / `serde_json` (strict schemas)
  - `hmac`, `sha2` (edge verification only), `blake3` (internal hashing)
  - `base64`, `hex`, `time` (strict parsing)
- **Pinned versions:** via workspace root `Cargo.toml` (no ad-hoc bumps in this crate).
- **Controls:**
  - `cargo-deny` (licenses, advisories, bans) in CI
  - Reproducible builds settings at workspace
  - SBOM emitted on release into `docs/sbom/`

---

## 7) Formal & Destructive Validation

- **Property tests:** DTO `deny_unknown_fields`; error taxonomy mapping; reason codes stable.
- **Fuzzing:** webhook parsers/signature base-string builders; decompression inputs (ratio/zip bombs); HTTP framing.
- **Loom tests:** producer→bounded queue→consumer with shutdown; assert no deadlocks/missed shutdown.
- **Chaos tests:** upstream 5xx bursts, latency injection > threshold, process restarts; ensure `/readyz` brownout and recovery are correct.
- **End-to-end sims:** webhook→verify→mailbox→worker→audit; assert single-apply under at-least-once delivery.
- **TLA+ (optional):** spec the idempotency path if additional assurance is required.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Backup:** Core Platform Security (internal group list)  
- **Disclosure policy:** See repo root `SECURITY.md` for how to report vulnerabilities (coordinated disclosure).

---

## 9) Migration & Upgrades

- Changes to **auth, hashing policy, or signature verification** require:
  - An ADR, a minor/major version bump in `CHANGELOG.md`, and dual-accept windows where feasible.
  - For provider webhooks, support dual secrets/alg schemes during rotation.
- Deprecations must include migration steps and explicit end-of-support timelines.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client / Provider] -->|TLS 1.3| B[svc-interop edge]
  A -. webhook .-> B
  B -->|verify HMAC-SHA256 (edge only)| B2{valid?}
  B2 -- no --> D[[Reject + Metrics + Audit]]
  B2 -- yes --> C[Translate to Capability (TTL ≤ 60s)]
  C --> E[Internal Actions (BLAKE3 addressing)]
  B -.->|/readyz brownout on breach| F[(Health)]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text description:** The edge terminates TLS and verifies provider HMAC-SHA256 signatures **only at ingress**. On success, it mints/validates a scoped capability (TTL ≤ 60s) and proceeds internally using **BLAKE3-based** addressing and auditing. On failure, it rejects with reason-coded metrics and audit. `/readyz` reflects brownout under SLO breaches.

---

```
::contentReference[oaicite:0]{index=0}
```
