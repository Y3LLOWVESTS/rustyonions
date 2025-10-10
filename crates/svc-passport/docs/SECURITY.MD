---

title: Security Notes — svc-passport
crate: svc-passport
owner: Stevan White
last-reviewed: 2025-10-09
status: draft
-------------

# Security Documentation — svc-passport

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-passport`.
It complements the repo-wide Hardening, Interop, and IDB blueprints.

`svc-passport` mints **capability-scoped tokens** (macaroon-style) with **short TTLs**, backed by keys managed in **ron-kms**. It never stores durable PII, and it does **not** verify tokens authoritatively (that’s `ron-auth`). PQ-hybrid readiness (e.g., `ed25519+ml-dsa`) is supported per policy.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats (examples)                                                   | Relevant in `svc-passport`? | Mitigation (tech & process)                                                                                                                                                                                           |
| -------------------------- | -------------------------------------------------------------------- | --------------------------: | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Client impersonation; forged issuer; rogue internal caller           |                     **Yes** | TLS termination at edge; optional service TLS via `tokio-rustls`; capability tokens require signed envelope (`alg`, `kid`, `epoch`); issuer policy anchored in `svc-registry` signatures; PEERCRED allowlist for UDS. |
| **T**ampering              | Modified requests, caveat inflation, token mutation                  |                     **Yes** | Detached signature(s) over token; `serde(deny_unknown_fields)`; canonical encoding; request body/ratio caps; policy-enforced caveat upper-bounds.                                                                     |
| **R**epudiation            | “We didn’t mint this token”; missing audit trail                     |                     **Yes** | Structured JSON logs with `corr_id`/`kid`/`epoch`; append-only issuance events (to `ron-audit` when enabled); clock-synced timestamps; config reload audit events.                                                    |
| **I**nformation Disclosure | Token/secret leakage; PII persistence                                |                     **Yes** | No durable PII (opaque `subject_ref` only); secrets never logged; amnesia mode for RAM-only caches; keys isolated in `ron-kms`; redacted config snapshots.                                                            |
| **D**enial of Service      | Floods, slow-loris, decompression bombs, queue exhaustion, KMS stall |                     **Yes** | Timeouts (`read/write=5s`, `kms=3s`); body cap (1 MiB); decompression ratio cap (≤10×); bounded mpsc queues; RPS/inflight caps; early **429 Busy**; readiness sheds issuance under pressure; breaker on KMS path.     |
| **E**levation of Privilege | Overbroad tokens; bypass revocation; policy downgrade                |                     **Yes** | Least-privilege caveats with maxima; issuer-policy versioning and signature (registry); fast revocation via epoch rotation + bus events; CI gates on policy changes; no “admin” omnipotent tokens.                    |

---

## 2) Security Boundaries

* **Inbound (trust edge):**

  * HTTP: `POST /v1/passport/issue`, `/verify`, `/revoke`
  * Metrics/health: `GET /metrics`, `/healthz`, `/readyz` (no secrets)
  * Optional UDS for intra-host callers (PEERCRED-checked)
  * Config reload signal: SIGHUP or `ConfigUpdated` bus event
* **Outbound (downstream dependencies):**

  * `ron-kms` (sign/rotate; network or UDS)
  * `svc-registry` (fetch issuer/alg policy descriptors)
  * Kernel bus (publish `PassportRevoked`, `PassportPolicyUpdated`)
  * `ron-audit` (structured issuance/revocation logs, if configured)
* **Trust Zone / Deployment:**

  * Typically runs in a **private cluster network** behind `svc-gateway`; may be exposed internally over UDS.
  * **Macronode:** normal persistence for metrics/logs; **Micronode:** amnesia-first, RAM-only caches.
* **Assumptions:**

  * Time is reasonably synchronized (token `iat/exp` sanity).
  * `ron-kms`, `svc-registry`, and `ron-auth` adhere to their own security notes and readiness contracts.
  * OS/container hardening (read-only FS where possible; least-privileged UID/GID; seccomp/AppArmor if available).

---

## 3) Key & Credential Handling

* **Key Types:**

  * **Issuer signing keys**: classical (`ed25519`) and optional PQ (`ml-dsa`, per policy). May be hybrid (`ed25519+ml-dsa`).
  * **TLS certificate/key** (optional service TLS if not terminated elsewhere).
  * **Registry client auth** (if MTLS or token is required by `svc-registry`).
* **Custody & Storage:**

  * Private keys **never** leave `ron-kms`. `svc-passport` only receives signatures and KID references.
  * No durable secret material at rest in `svc-passport`. In Micronode/amnesia mode, caches are RAM-only.
* **Rotation & Revocation:**

  * **Key/Epoch rotation ≤ 30 days**; overlapping grace window ensures old tokens verify until `exp`.
  * Revocation via **epoch bump** and **bus event**; downstream (`ron-auth`, services) must drop stale `kid/epoch` within SLO (≤ 5 s p99).
  * TLS cert/key rotation per ops policy (reload triggers rebind if service TLS used).
* **Zeroization & Redaction:**

  * Any transient sensitive buffers are wrapped in `Zeroizing` where applicable.
  * Logs/config diffs **redact** secret-bearing fields; tokens are **never** logged verbatim.

---

## 4) Hardening Checklist (enforced/required)

* [ ] **Request timeouts:** `read=5s`, `write=5s`, `idle=60s` (configurable).
* [ ] **Concurrency cap:** ≥512 inflight ceiling (per instance).
* [ ] **RPS cap:** ≥500 default (tune per instance).
* [ ] **Body cap:** 1 MiB (HTTP 413 on exceed).
* [ ] **Decompression ratio cap:** ≤10×; reject over-ratio payloads.
* [ ] **Bounded channels:** issuance(512), revoke(256); `try_send` + 429 Busy on overflow.
* [ ] **UDS security:** parent dir `0700`, socket `0600`, `SO_PEERCRED` allowlist when UDS enabled.
* [ ] **No ambient trust:** all tokens caveat-scoped; unknown caveats rejected.
* [ ] **Amnesia mode:** RAM-only caches; no persistent artifacts post-start (Micronode).
* [ ] **Chaos drill:** restart under load → `/readyz` flips Not Ready for issuance; verify preflight may remain OK.

---

## 5) Observability for Security

* **Metrics (minimum set):**

  * `passport_rejects_total{reason="unauth"|"ttl_too_long"|"unknown_caveat"|"over_limit"|"ratio_cap"}`
  * `tls_handshake_failures_total` (if service TLS is enabled)
  * `passport_revocations_total{reason}` / `passport_tokens_issued_total{aud,svc}`
  * `busy_rejections_total{endpoint}` (backpressure indicator)
  * `config_reloads_total`, `config_reload_errors_total`
* **Logs (structured JSON):** include `corr_id`, `peer_addr`, `endpoint`, `kid`, `epoch`, `decision`, `reason`, `latency_ms`. No token bodies or secrets.
* **Health Gates:**

  * `/healthz`: liveness only.
  * `/readyz`: **fail-closed issuance** on degraded state (KMS unhealthy, queues saturated, policy desync). Verification preflight can remain available to help drain.

---

## 6) Dependencies & Supply Chain

* **Security-sensitive crates:**

  * `tokio-rustls` (TLS 1.3 only), `ring`/`rustls` transitive
  * `serde`, `serde_json` (strict DTOs with `deny_unknown_fields`)
  * `blake3` (content hashing if used for refs), `zeroize`
  * `axum`/`hyper` for HTTP; `tower` for timeouts/limits
* **Controls:**

  * Pinned versions at workspace root; Renovate/Dependabot monitored.
  * `cargo-deny` CI: advisory audit, duplicate/ban lists (e.g., ban `openssl-sys` unless explicitly allowed).
  * **SBOM** generated on release (CycloneDX) and stored in `docs/sbom/`.
  * **Reproducible builds** target (deterministic flags where feasible).

---

## 7) Formal & Destructive Validation

* **Property tests:**

  * Caveat enforcement never increases privilege; TTL clamp monotonicity; unknown fields rejected.
* **Fuzzing:**

  * DTO parser fuzz (JSON boundary/invalid inputs); optional compression bomb corpus.
* **Loom (dev):**

  * Producer→bounded queue→consumer with shutdown; assert no missed shutdown or double-drop.
* **Chaos:**

  * Kill/restart workers under load; induce KMS latency spikes; verify breaker and readiness transitions.
* **TLA+ (optional):**

  * Model issuance/rotation/revocation: safety (no valid token without current policy), liveness (tokens minted or rejected promptly; revocations propagate).

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Backup:** Platform Security (on-call rota)
* **Disclosure policy:** See repo root `SECURITY.md` (coordinated disclosure; private security mailbox).

---

## 9) Migration & Upgrades

* **Auth/Key Changes:** Any change to token envelope, `alg` defaults, or verification semantics requires a **semver minor** at minimum; **major** if DTOs or verification contracts break.
* **Policy/Config Deprecations:** Provide alias keys ≥1 minor; WARN on use; document in CHANGELOG with **end-of-support** window.
* **PQ Rollout Phases:** `mirror → require → enforce` must be announced via registry-signed policy; specify grace windows; include rollback plan (fall back to `mirror`).

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client/App] -->|TLS (edge) + Capability (Bearer)| B(svc-passport)
  B -->|KMS sign (deadline)| K[ron-kms]
  B -->|Policy fetch/verify| R[svc-registry]
  B -.->|Revocation events| BUS[(Kernel Bus)]
  B -->|Minted Token| A
  B -.->|Reject unauth/oversized/over-quota| D[HTTP Error + Metrics]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text description:** Client calls svc-passport with a capability issuance request over TLS (typically terminated at the edge). svc-passport enforces limits, checks policy, requests a signature from ron-kms within a deadline, and returns a token or a structured error. Revocations/policy updates are distributed via the kernel bus; metrics/logs capture all security-relevant outcomes.

---

### Appendix: Reviewer Checklist (quick copy for PRs)

* [ ] No token bodies or secrets in logs (added tests if logging changed).
* [ ] DTOs use `deny_unknown_fields`; new caveats gated by policy maxima.
* [ ] New outbound call has explicit timeout and error mapping.
* [ ] Boundaries updated (inbound/outbound lists) if endpoints or deps changed.
* [ ] Hardening caps unchanged or justified (body, RPS, inflight, ratio).
* [ ] Rotation/epoch logic unchanged or migration note added.
* [ ] SBOM and `cargo-deny` pass.

---
