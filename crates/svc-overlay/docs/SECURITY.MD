

---

## 🔐 Updated file (drop-in)

````markdown
---
title: Security Notes — svc-overlay
crate: svc-overlay
owner: Stevan White
last-reviewed: 2025-10-03
status: draft
---

# Security Documentation — svc-overlay

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-overlay`.  
It complements the repo-wide Hardening, Interop, IDB, and Config blueprints.

> **Security creed**
> - **Zero ambient trust** — all effects are capability-gated; IP ≠ identity.
> - **Fail closed** — uncertainty → reject loudly with typed reasons + metrics.
> - **Amnesia-ready** — with `amnesia=true`, no secrets touch disk.
> - **Small surface** — overlay ≠ DHT; no routing/discovery here.

---

## 1) Threat Model (STRIDE)

| Category | Example threats | Relevant? | Primary mitigations |
|---|---|---:|---|
| **S**poofing | Fake peers, source-IP trust, gateway confusion | **Yes** | TLS 1.3 (rustls) + **ALPN=ron/oap1**; optional **mTLS** (cluster); **capability tokens (macaroons)**; deny IP trust |
| **T**ampering | Mutated frames, downgrade, on-path edits | **Yes** | OAP/1 **length bounds**; **BLAKE3 content addressing** before serve; TLS1.3 only; PQ **hybrid KEX** when enabled |
| **R**epudiation | Unattributed actions | **Yes** | Structured JSON logs (`corr_id`, `peer_id`, `cap_id`), immutable counters |
| **I**nfo Disclosure | Token/key leaks; over-logging; cert theft | **Yes** | **0600** key files; **zeroize/secrecy**; amnesia; log scrubbers; metrics/health on localhost or gated |
| **D**oS | Slowloris, handshake floods, oversize frames, decompression bombs, queue exhaustion, QUIC amplification | **Yes** | **Accept/handshake rate-limits**, **1 MiB** cap, **≤10×** decompression, per-peer **RPS** + inflight, **timeouts** (handshake/read/write/idle), **bounded queues** + fair-shed, QUIC anti-amplification |
| **E**levation of Privilege | Bypassing authz, wildcard CAs, weak SAN checks | **Yes** | Macaroons (caveats), **mTLS SAN pinning** (cluster), CA pinsets, forbid wildcard CAs |

**Out-of-scope here (belongs elsewhere):**
- DHT/routing security (lives in `svc-dht`).
- Persistent database security (overlay should not persist state with amnesia).

---

## 2) Security Boundaries

**Inbound (attack surface)**
- **Overlay listener(s)**: TCP/TLS(/QUIC) speaking OAP/1; handshake RPS limiting at accept-loop.
- **Admin/metrics**: `/metrics`, `/healthz`, `/readyz`, `/version` (localhost or gateway only).
- **UDS (optional)**: local control; `SO_PEERCRED` + UID allowlist.
- **Config inputs**: file/env/CLI (validated; no secrets in logs).

**Outbound**
- **ron-transport** to peers (TLS/QUIC/Tor).  
- **Kernel/Bus** events (shutdown, config updated).  
- **ron-kms** for PQ hybrid (when available).

**Trust zones**
- Public internet (**untrusted**).
- In-cluster peers with mTLS (**minimally trusted**, capabilities still required).
- Local admin plane (**least privilege**).

**Assumptions**
- Kernel provides good time/entropy.
- Downstream services expose `/readyz` & enforce their own quotas.
- Isolated FS namespaces; no shared writable dirs with other tenants.

---

## 3) Key & Credential Handling

**Types**
- TLS server key/cert (PEM); optional client certs for mTLS.
- Macaroon capability tokens.
- Optional node identity (Ed25519) if used for peer identity.
- PQ material via `ron-kms` (Kyber + X25519) when `pq_mode=hybrid`.

**Storage & lifecycle**
- At rest: keys/tokens **0600**, parent **0700**, read-only mounts.
- In memory: `zeroize::Zeroizing` or `secrecy::Secret`.
- Amnesia: prohibits writing secrets to disk; ephemeral creds only.

**Rotation & revocation**
- TLS rotation ≤ **30d** (SIGHUP reload; rebind if needed).
- Macaroon rotation via secret roll; tiny cache TTLs (≤5m); caveat expiry; blacklist support.
- PQ rollout: canary first; fail-closed with incompatible peers.

**TLS/mTLS policy**
- TLS **1.3 only**; **cipher allowlist**: `TLS_AES_128_GCM_SHA256`, `TLS_CHACHA20_POLY1305_SHA256`.
- **ALPN**: `ron/oap1` required.
- Session tickets: **rotated** frequently or **disabled** per policy.
- mTLS: pin CA bundle; verify SAN against allowed patterns or exact IDs.

---

## 4) Hardening Checklist (overlay-specific)

- [ ] Handshake **rate-limit** at accept-loop (per-ip/peer-id bucket; burst & sustained).  
- [ ] Timeouts: handshake=3s; read=5s; write=5s; idle=15–30s.  
- [ ] Concurrency cap: ≤ **512** inflight; per-peer inflight ≤ 32/128 (micro/macro).  
- [ ] RPS cap per-peer: 200/500 (micro/macro).  
- [ ] Body/frame caps: **1 MiB**; decompression ratio ≤ **10×** + absolute post-size cap.  
- [ ] Bounded channels; `try_send` + typed `Busy`; drop metrics.  
- [ ] TLS: key/cert **0600**; TLS1.3 only; ALPN required; cipher allowlist.  
- [ ] mTLS (cluster): SAN pinning; no wildcard CAs; CA pinset versioned.  
- [ ] UDS: dir **0700**, socket **0600**, enforce `SO_PEERCRED` + UID allowlist.  
- [ ] Amnesia: no persistent writes; assert at boot (fail fast).  
- [ ] QUIC: anti-amplification limits; validate path MTU; disable 0-RTT unless explicitly allowed.  
- [ ] Chaos: restart under load; `/readyz` flips; writes fail first.  
- [ ] Metrics/admin bound to localhost or gated behind an ingress proxy.

---

## 5) Observability for Security

**Metrics**
- `auth_failures_total{reason="unauth|expired|invalid_caveat"}`
- `rejected_total{reason="oversize|ratio_cap|timeout|busy|unauth|handshake_ratelimit"}`
- `tls_handshake_failures_total{reason}`
- `handshake_attempts_total`, `handshake_ratelimited_total`
- `backpressure_events_total{peer}`, `queue_dropped_total{queue}`
- `readyz_state` (gauge: 0=down,1=ready,2=degraded)

**Logs (JSON)**
- Fields: `service="svc-overlay"`, `ts`, `level`, `corr_id`, `peer_addr`, `peer_id?`, `cap_id?`, `reason`, `error_code`.
- Scrubbers: redact tokens/keys; size caps; rate-limit repetitive errors to avoid amplification.
- Never log full certificates or private keys; fingerprint only.

**Health gates**
- `/readyz` returns **503** on degraded security posture (e.g., KMS needed for PQ but unavailable, key perms invalid, overload); **200** only when safe to accept **writes**.

---

## 6) Dependencies & Supply Chain

**Sensitive crates**: `tokio-rustls/rustls`, `zeroize`, `secrecy`, `blake3`, `serde`/`serde_json`, `bytes`, `humantime-serde`, `byte_unit`, optional `arti`, `quinn`.

**Controls**
- `cargo-deny` (advisories/bans/licenses) **required** in CI.
- `cargo-audit` **required**; block known vulns.
- `cargo-vet` **required**; policy checked-in; third-party audits recorded.
- Lockfile committed; pin critical deps at workspace root.
- **SBOM** (CycloneDX) generated on release and stored under `/docs/sbom/`.
- **Provenance** (SLSA-style) attestation emitted by CI for release artifacts.

---

## 7) Formal & Destructive Validation

**Property tests**
- Frames >1 MiB → **FrameTooLarge**; decompression > caps → `400` + metric.
- Handshake burst beyond limiter → `rejected_total{reason="handshake_ratelimit"}` increments.
- Log scrubber never emits `macaroon`, `private_key`, or raw token bytes.

**Fuzz**
- OAP/1 parser (length truncation, split boundaries, mutation corpus).
- Token/cap parser with corrupted caveats → unauth path.

**Loom**
- Producers → bounded egress → Writer + shutdown; assert **no deadlock**, **no lost shutdown**, **no send-after-close**.
- (PQ path) simulate hybrid material hand-off signal to ensure no use-before-ready interleavings.

**Chaos**
- Slowloris peers; handshake floods; Tor egress churn; verify `/readyz` flips, **no memory growth**, fair-shed engages.

**Interoperability**
- Mixed cluster PQ (off/on); fail-closed when unsupported; negotiate cleanly when available.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Security contact:** security@rustyonions.dev  
- **Disclosure:** see repo `SECURITY.md` (coordinated disclosure; 90-day max).

---

## 9) Migration & Upgrades

- Crypto/auth changes (e.g., making **mTLS mandatory**, enabling **PQ hybrid** by default) require a **major version** and a **canary rollout** plan.  
- Key rotation plans include rollback material for one cycle.  
- Deprecations ship with env/flag aliases (≥1 minor), deprecation logs, and **CHANGELOG.md** steps.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Remote Peer] -->|TLS 1.3 (+ mTLS optional) + ALPN ron/oap1| B(svc-overlay)
  A -.->|Macaroon (capability)| B
  B --> C{Checks}
  C -->|OK| D[Gossip/Session Handling]
  C -->|unauth| R1[[Reject: unauth]]:::rej
  C -->|oversize| R2[[Reject: oversize]]:::rej
  C -->|ratio_cap| R3[[Reject: ratio_cap]]:::rej
  C -->|timeout| R4[[Reject: timeout]]:::rej
  C -->|handshake_ratelimit| R5[[Reject: handshake_ratelimit]]:::rej
  D --> E[Downstream (bus/transport)]
  classDef rej fill:#7f1d1d,stroke:#450a0a,color:#fff
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text:** Peer connects over TLS1.3 (optional mTLS). Overlay validates capability, size/ratio/time, and handshake rate limits. Allowed traffic flows to session/gossip; rejections emit metrics/logs with granular `reason`.

---

## 11) Implementation Notes (ties to code/config)

* **Config keys:** `tls.*`, `auth.macaroon_path`, `overlay.pq_mode`, `limits.*`, `timeouts.*`, `overlay.peer.*`, `overlay.gossip.*`, `uds.*`; (handshake limiter shares `overlay.peer.max_rps` bucket or a dedicated `handshake_rps` knob if added later).
* **Boot guards:** fail start if TLS/macaroons missing or perms > **0600**; if `amnesia=true`, assert no writable secret paths.
* **Runtime guards:** if PQ is configured but KMS unavailable/incompatible → `/readyz` **degraded** and **writes shed first**.
* **Tor note:** never tie quotas to IP; key by **peer identity**/capability when onion-routed.
* **QUIC note:** disable 0-RTT unless explicitly allowed and audited; enforce anti-amplification budget.

---

## 12) Forbidden / Anti-Scope (security edition)

* ❌ Implementing DHT/routing or accepting unauthenticated control here.
* ❌ Logging secrets or raw tokens; stack traces on hot paths.
* ❌ Custom crypto; only rustls/ron-kms/blake3.
* ❌ Unbounded queues; IP-based trust; wildcard CAs; enabling TLS<1.3.

```
```

