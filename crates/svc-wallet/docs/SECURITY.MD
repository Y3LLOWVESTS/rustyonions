---

title: Security Notes — svc-wallet
crate: svc-wallet
owner: Stevan White
last-reviewed: 2025-10-16
status: draft
-------------

# Security Documentation — svc-wallet

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-wallet`.
It complements the repo-wide Hardening Blueprint and Interop Blueprint, and aligns with `docs/IDB.md`, `docs/CONFIG.md`, and the Concurrency model.

---

## 1) Threat Model (STRIDE)

> Context: `svc-wallet` is a public-facing value-plane service (issue/transfer/burn/balance/receipt lookups) that **defers durable truth to `ron-ledger`**. It verifies **capability tokens** (macaroons via `ron-auth`), enforces **economic invariants** (no doublespends/negatives), and runs with **amnesia mode** option for micronodes (RAM-only).

| Category                   | Threats                                                     | Relevant in `svc-wallet`? | Mitigation                                                                                                                                                                                      |
| -------------------------- | ----------------------------------------------------------- | ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Fake identities, forged capabilities, MITM                  | **Y**                     | TLS 1.3 (tokio-rustls); capability tokens (macaroons) validated via `ron-auth`; strict `aud/iss/exp/scope` checks; optional mTLS at edge; deny ambient auth (cookies, IP lists).                |
| **T**ampering              | Mutated requests, replay, ledger commit alteration          | **Y**                     | DTO schema with `deny_unknown_fields`; idempotency keys + nonce reservation (at-most-once); content hashing (BLAKE3) for receipts; append-only commit via `ron-ledger`; HSTS at edge.           |
| **R**epudiation            | “I didn’t do that transfer”                                 | **Y**                     | Structured JSON logs w/ `corr_id`, caller `sub`/account, deterministic receipts (`txid`, `nonce`, `idem`); audit events; time-synced clocks (NTP).                                              |
| **I**nformation Disclosure | PII/secret leakage in logs, headers; key exposure           | **Y**                     | Redaction layer (never log `Authorization`, macaroons, secrets); minimal data retention; amnesia mode (no persistent secrets); TLS-only; zeroize key material in memory.                        |
| **D**enial of Service      | Flooding, slow-loris, decompression bombs, queue exhaustion | **Y**                     | 1 MiB body cap; 10× decompression ratio cap; timeouts (read/write 5s); RPS & inflight limits; mpsc(512) backpressure (reject 429); `/readyz` sheds writes first; circuit breakers on upstreams. |
| **E**levation of Privilege | Capability abuse, scope-bypass, confused deputy             | **Y**                     | Macaroon scope enforcement (account/asset/action/ceilings/TTL); policy checks via `ron-policy`; per-account nonce linearizability; no direct DB; least-privilege service account to upstreams.  |

**Non-goals / Out of scope**

* Multi-party settlement, cross-asset atomic swaps (lives in higher-level services).
* Custody of private signing keys (handled by `ron-kms`/`ron-auth`, not `svc-wallet`).

---

## 2) Security Boundaries

* **Inbound (attack surface):**

  * HTTP API (Axum)

    * `POST /v1/issue` (requires capability: `issue`)
    * `POST /v1/transfer` (capability: `transfer`)
    * `POST /v1/burn` (capability: `burn`)
    * `GET /v1/balance?account=&asset=`
    * `GET /v1/tx/{txid}`
  * Operational: `GET /healthz`, `GET /readyz`, `GET /metrics` (metrics on loopback or behind gateway).

* **Outbound (trusting dependencies):**

  * `ron-ledger` (append-only commit, batch receipt hash).
  * `ron-auth` (capability verification / macaroon caveats).
  * `ron-policy` (issuance and ceiling policy).
  * Metrics sink (Prometheus scrape).
  * Optional bus/notify topic for ledger invalidation (broadcast).

* **Trust Zone:**

  * **Public-facing** HTTP (internet or tenant edge).
  * Upstreams typically intra-tenant (private network/VPC).
  * Filesystem is considered **untrusted** in amnesia mode; otherwise minimal and read-only (certs).

* **Assumptions:**

  * Upstreams expose `/readyz` and enforce their own quotas/limits.
  * Edge/gateway enforces TLS, HSTS, optional mTLS, IP reputation if applicable.
  * Host kernel is hardened (cgroups, low RLIMITs, no ptrace); container user is non-root.

---

## 3) Key & Credential Handling

* **Key/cred types handled/consumed by `svc-wallet`:**

  * TLS certificate and private key (if not terminated upstream).
  * Capability tokens (macaroons) presented by clients (verification via `ron-auth`).
  * Optional service-to-service bearer tokens for upstreams (retrieved from platform secret store).

* **What `svc-wallet` does *not* handle:**

  * Long-term signing keys for capabilities or ledger—custodied by `ron-kms`/`ron-auth`.

* **Storage rules:**

  * TLS private key on disk (if used): permissions `0600`, directory `0700`.
  * All secrets loaded into memory are wrapped in `zeroize::Zeroizing`.
  * **Amnesia mode:** no secret persistence; config rejects persistent stores; process restarts lose any in-memory secret cache.

* **Rotation / Revocation:**

  * Capability tokens: TTL-limited; revocation via `ron-auth` policy or caveat invalidation.
  * TLS certificates: platform rotation; support SIGHUP reload of config/keys.
  * Service tokens: platform-managed rotation (≤30 days); reload on SIGHUP.

* **Leak prevention:**

  * Never log headers `Authorization`, `Cookie`, or token claims; redact values at log layer.
  * Receipts and audit logs **never** contain raw tokens.

---

## 4) Hardening Checklist (from Blueprint + service specifics)

* [ ] 5s timeout on all HTTP requests (`read`/`write`) and total handler deadline.
* [ ] Concurrency cap: ≤512 inflight; listener drops to 429 on pressure (no buffering).
* [ ] Global RPS cap (default 1000/s) with token-bucket burst (default 2000).
* [ ] Request body cap: **1 MiB**; `Content-Length` validated; decompression ratio ≤ **10×**.
* [ ] DTOs enforce `deny_unknown_fields`; amount parsing is **u128** only (no floats).
* [ ] No lock held across `.await`; nonce reservation via atomic `fetch_update`.
* [ ] No persistent writes in **amnesia mode**; fail-fast if config attempts it.
* [ ] TLS via **tokio-rustls** only; no legacy TLS; disable client renegotiation.
* [ ] UDS (optional): dir `0700`, socket `0600`; enforce `SO_PEERCRED` allowlist.
* [ ] No direct DB access; all settlement via `ron-ledger` API.
* [ ] Chaos check: induce upstream stall—`/readyz` must fail-closed; POSTs return 503 with `Retry-After`.
* [ ] Circuit breaker on upstreams; exponential backoff + jitter; bounded retry budget.

---

## 5) Observability for Security

* **Metrics (counters unless noted):**

  * `wallet_rejects_total{reason="unauth"|"policy_denied"|"insufficient_funds"|"limits_exceeded"|"bad_request"}`
  * `busy_rejections_total{endpoint}`
  * `io_timeouts_total{op}`; `upstream_fail_total{svc}`; `circuit_open_total{svc}`
  * `tls_handshake_failures_total`
  * `wallet_idem_replays_total` (replay detection)
  * Gauges: `queue_depth{queue}`, `worker_inflight`

* **Logs (structured JSON):**

  * Always include `service="svc-wallet"`, `corr_id`, `peer_addr`, `http_target`, `status`, and **never** include secrets.
  * On security failures: `reason`, `scope` (sanitized), `account` hash or stable pseudonym, not raw IDs if policy requires.

* **Health gates:**

  * `/healthz`: liveness only.
  * `/readyz`: fail-closed on upstream breaker open, queue saturation, or config invalid; **never** green if we would violate caps/invariants.

---

## 6) Dependencies & Supply Chain

* **Security-sensitive crates:**

  * `tokio-rustls` (TLS 1.3 only), `rustls-pemfile`
  * `axum`/`hyper`/`tower` (hardened middleware stack)
  * `serde`/`serde_json` with strict DTOs
  * `blake3` (receipt hashing)
  * `zeroize` for in-memory secret hygiene
  * `dashmap` for sharded state (nonce table)
  * Internal: `ron-ledger-client`, `ron-auth`, `ron-policy`, `ron-proto`

* **Controls:**

  * Pinned via workspace `Cargo.toml`; `cargo-audit` + `cargo-deny` in CI; ban yanked/unknown-license crates.
  * Reproducible builds target; SBOM generated at release under `docs/sbom/`.
  * Enable `-D warnings` and selected clippy lints (`await_holding_lock`, etc.).

---

## 7) Formal & Destructive Validation

* **Property tests:**

  * Idempotency/doublespend: replays must return **identical receipts**; no two commits per `(account, nonce)`.
  * Non-negativity and conservation per batch.

* **Fuzzing:**

  * HTTP/OAP parser fuzz (malformed JSON, size extremes, compression bombs).
  * Capability token claim fuzz (bad encodings, expired/not-yet-valid, scope mismatch).

* **Loom tests:**

  * Producer→bounded-queue→consumer with shutdown; assert no deadlocks or missed cancellations; nonce reservation linearizable.

* **Chaos:**

  * Inject latency/timeouts on `ron-ledger` and `ron-auth`; assert `/readyz` behavior, bounded queues, and correct error surfacing (429/503).

* **Optional TLA+:**

  * Safety: at-most-once debit per `(account, nonce)`; conservation across commits.
  * Liveness: under finite arrival rate and healthy upstream, eventual completion.

---

## 8) Data Handling & Privacy

* **Data classes:** account IDs, asset IDs, amounts, receipts (txid, nonce, idem). No sensitive PII by default; if account IDs can map to PII externally, treat IDs as **pseudonymous**.
* **Retention:** `svc-wallet` does not persist authoritative balances. Receipts are returned to caller and may be cached transiently (idempotency TTL default 24h).
* **Redaction:** redact tokens/headers; consider hashing account identifiers in logs if compliance requires.

---

## 9) Security Contacts

* **Maintainer:** Stevan White
* **Escalation:** Core platform security on-call (see org runbook).
* **Disclosure policy:** See repo root `SECURITY.md` (responsible disclosure; PGP key if applicable).

---

## 10) Migration & Upgrades

* **Auth or capability format changes:** require a **major version bump**, deprecation window, and dual-accept period where feasible.
* **TLS/mTLS posture shifts:** document edge vs service termination; provide rotation instructions and SIGHUP reload path.
* **Policy enforcement changes (e.g., ceilings):** publish migration guide and config compatibility notes; add deny-by-default toggles with feature flags if needed.

---

## 11) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + macaroon (Authorization: Bearer)| B(svc-wallet)
  B -->|Verify scope/TTL with ron-auth| BA[Auth Check]
  B -->|Policy check| BP[ron-policy]
  B -->|Append-only commit| BL[ron-ledger]
  B -.->|Reject: unauth/oversized/limits/policy| D[4xx/5xx + Metrics + Audit]
  A -.->|Replay (same Idempotency-Key)| B
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text description:** The client calls `svc-wallet` over TLS with a capability token. The service verifies the token (`ron-auth`), enforces policy (`ron-policy`), and commits to the append-only ledger (`ron-ledger`). Invalid/oversized/limit-violating requests are rejected with proper codes and metrics. Replays return the same receipt via idempotency.

---

## 12) Reviewer Checklist (must be green before release)

* Capability verification rejects expired/not-yet-valid tokens and scope mismatches.
* `Authorization` header and secrets are redacted in logs.
* RPS/inflight/body/ratio caps enforced before any upstream call.
* `/readyz` fails when upstream breakers are open or queues are saturated.
* No persistence when `amnesia.enabled=true` (integration test proves zero disk writes).
* Chaos tests pass: stall upstreams → correct shed/flip behavior.
* `cargo-audit`, `cargo-deny`, clippy `-D warnings` all green; SBOM generated.

---


