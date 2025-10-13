````markdown
---
title: Security Notes — svc-mailbox
crate: svc-mailbox
owner: Stevan White
last-reviewed: 2025-10-12
status: draft
---

# Security Documentation — svc-mailbox

This document defines the **threat model**, **security boundaries**, and **hardening requirements** for `svc-mailbox`.  
It complements the repo-wide Hardening, Interop, and IDB blueprints.

---

## 1) Threat Model (STRIDE)

| Category | Threats (examples) | Relevant in `svc-mailbox`? | Primary Mitigations |
|---|---|---:|---|
| **S**poofing | Client or peer impersonation; stolen tokens | **Yes** | TLS 1.3 (tokio-rustls); **macaroon capabilities** with scoped caveats (topic/op/quotas/expiry); `cap_cache_ttl=30s` with fast revocation; optional UDS + `SO_PEERCRED` allowlist |
| **T**ampering | Payload/message alteration, replay, or corrupt persistence | **Yes** | **BLAKE3** `payload_hash` + `hash_chain` (I-12); idempotency key + `T_replay=300s`; deny-unknown-fields; OAP/1 frame caps; DLQ quarantine with reason/counters |
| **R**epudiation | Untraceable operations or lack of audit | **Yes** | Structured JSON logs; **`corr_id` (UUIDv7)** propagation; immutable `msg_id`; stable error taxonomy; time-synced (tolerant) telemetry |
| **I**nformation Disclosure | PII in logs; secret/key exposure | **Yes** | **Amnesia mode** (RAM-first, zeroize); never log macaroon contents; TLS key perms 0600; minimal telemetry; per-topic retention caps |
| **D**enial of Service | Flood, slow-loris, decompression bombs, queue exhaustion | **Yes** | 5s I/O timeouts; request body cap **1 MiB**; **decompress ratio ≤10×**; bounded shard queues + global inflight cap; `429/503` with `Retry-After`; jittered backoff; `/readyz` degrades writes first |
| **E**levation of Privilege | Use of broader-than-intended rights | **Yes** | Capabilities **must** scope topic/ops/quotas/expiry; rotation & revocation drills; no ambient trust; least-privilege defaults; feature-gated admin endpoints |

---

## 2) Security Boundaries

- **Inbound (public/service API):**  
  - `POST /v1/send` (enqueue; capability required)  
  - `POST /v1/recv` and `POST /v1/recv/stream` (lease/visibility)  
  - `POST /v1/ack/{msg_id}`, `POST /v1/nack/{msg_id}`  
  - `GET /metrics` (usually **localhost** or via gateway), `GET /healthz`, `GET /readyz`
- **Inbound (admin/operator):**  
  - `POST /v1/dlq/reprocess` (guarded; role-scoped capability)  
- **Outbound (downstream calls):**  
  - Capability authority/validator (macaroon root or verifier)  
  - Metrics collector (Prometheus scrape)  
  - Optional: audit/event bus (append-only), object store if enabled for durable DLQ (macronode)
- **Trust Zone:**  
  - Runs as a **networked service**; can be Internet-facing behind a gateway. Metrics ideally loopback-only.  
  - **Micronode**: single-tenant dev/test; ephemeral DLQ by design.  
  - **Macronode**: multi-tenant cluster profile; durable DLQ; stricter network policy.
- **Assumptions:**  
  - Upstream gateway terminates TLS if mailbox is behind it; otherwise mailbox terminates TLS.  
  - Capability service clocks are within ±2 minutes (IDB I-17).  
  - Host/container FS permissions isolate service user; data path ownership is correct.  

---

## 3) Key & Credential Handling

- **Key/Secret Types:**  
  - **TLS**: certificate + private key (PEM)  
  - **Macaroon**: root/verification keys (or shared verifier secret)  
  - Optional **signing key** for envelope authenticity (`sig` field; future use)
- **Storage:**  
  - **Macronode**: secrets mounted read-only; mode **0600**; owned by service user.  
  - **Micronode / Amnesia=ON**: secrets only in memory where feasible; **no disk spill**.  
- **Rotation & Revocation:**  
  - TLS certs via standard secret rotation.  
  - **Macaroons:** rotate ≤ 30 days; **revocation** honored within `cap_cache_ttl` (default **30s**). Cache purge on auth-failure spikes.  
- **Zeroization:**  
  - Wrap sensitive material with `zeroize::Zeroizing`; clear buffers after use.  
  - Avoid cloning secret bytes; pass `Arc<[u8]>`/views only when necessary.
- **Access Controls:**  
  - Configurable **role separation**: producer-only, consumer-only, admin.  
  - Admin endpoints require distinct capabilities (no wildcard caps).

---

## 4) Hardening Checklist (Mailbox-Specific)

- [ ] **I/O timeouts**: read=5s, write=5s, idle=60s (configurable).  
- [ ] **Body cap**: **1 MiB** (OAP/1 frame); **decompress ratio cap**: 10×; reject on exceed.  
- [ ] **Bounded queues**: per-shard capacity (`shard_capacity`), global inflight cap (`global_inflight`).  
- [ ] **Backpressure**: `429/503` with `Retry-After`; never buffer unbounded.  
- [ ] **Idempotency window**: `T_replay=300s` (≥ 2× visibility); dedup table bounded.  
- [ ] **Visibility**: monotonic deadlines; reappearance after expiry; races pass property tests.  
- [ ] **Integrity**: verify `payload_hash` and `hash_chain`; `E_INTEGRITY` on mismatch → DLQ after policy.  
- [ ] **Auth**: all ops require scoped macaroon; **no ambient** auth; `/metrics` not public.  
- [ ] **Logs**: JSON only; redact secrets; include `corr_id`, `peer_addr`, decision reason.  
- [ ] **UDS**: dir `0700`, sock `0600`, enforce `allow_uids`, verify `SO_PEERCRED`.  
- [ ] **/readyz**: fail-closed under saturation (degrade **writes** first).  
- [ ] **Chaos**: restart under load; messages are at-least-once or DLQ’d; no silent loss.

---

## 5) Observability for Security

- **Metrics (minimum set):**  
  - `rejected_total{reason="unauth"|"oversize"|"ratio_cap"|"saturated"}`  
  - `auth_failures_total{scope}` (cap validation failures)  
  - `tls_handshake_failures_total`  
  - `dlq_total{topic,reason}` and `dlq_reprocess_total{topic}`  
  - `integrity_fail_total{reason}`  
  - `busy_rejections_total{endpoint}`  
  - `queue_depth{queue,shard}`, `saturation{queue,shard}`
- **Logs:**  
  - Always structured JSON; include `service`, `corr_id`, `peer_addr`, `cap_scope`, `decision`, and `reason`.  
  - Emit **security decision logs** for authz failures, integrity failures, and DLQ moves.  
- **Health gates:**  
  - `/healthz`: liveness only.  
  - `/readyz`: returns **503** if saturation thresholds or auth backends are unhealthy; prefer **degrading writes first**.

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates:**  
  - `tokio-rustls` (TLS 1.3), `rustls-pemfile`  
  - `blake3` (hashing), `zeroize` (secret memory hygiene)  
  - `serde` + `serde_json` (strict DTOs with `deny_unknown_fields`)  
  - `bytes`, `tokio`, `hyper`/`axum` or `tower` stack (transport)  
  - Macaroon/capability library (internal or vetted external)  
- **Controls:**  
  - **Pinned** via workspace Cargo.toml; **`cargo-deny`** in CI (advisories/licenses/bans).  
  - **SBOM** generated at release and stored under `docs/sbom/`.  
  - Reproducible builds and supply chain attestation (if configured org-wide).

---

## 7) Formal & Destructive Validation

- **Property tests:** malformed frames → 400/413; oversize/compression → 413; duplicate SEND → 409 with same `msg_id`.  
- **Fuzzing:** OAP/1 frame decoders, JSON envelopes (`payload_hash`, `hash_chain`, attrs); corpus includes boundary sizes and random bit flips.  
- **Loom (dev-only):** RECV/ACK races under shutdown and visibility expiry; assert no deadlocks, no double-ACK.  
- **Chaos drills:** kill shard workers under load; verify **at-least-once** (reappearance or DLQ); revoke caps mid-traffic; verify fail-closed ≤ `cap_cache_ttl`.  
- **(Optional) TLA+ sketch:** model at-least-once + visibility timers; verify safety (no loss without DLQ) and liveness (eventual delivery/ quarantine).

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Backup:** RustyOnions Core Team (security@project.local)  
- **Disclosure policy:** see repo root `SECURITY.md` (coordinated disclosure; 90-day SLA).

---

## 9) Migration & Upgrades

- **Auth/capability format changes** → **major version** bump with dual-accept window and clear rollback.  
- **Key rotation changes** (e.g., macaroon root rollover) → publish schedule + overlap period; CI tests for mixed-mode.  
- **Deprecations** → announce in `CHANGELOG.md` with EoS date and config migration table.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + macaroon| B(svc-mailbox)
  B -->|verify cap + size + integrity| B
  B -->|Validated request| C[Downstream (cap authority / svc-mod)]
  B -.->|Reject unauth / oversize / saturated / integrity| D[Error + Security Metrics]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Alt text:** Client connects to `svc-mailbox` using TLS and macaroon capability. Mailbox validates capability, size, and integrity before processing. Unauthorized/oversized/integrity failures are rejected with metrics; valid requests proceed to internal processing or downstream services.

---

```
```
