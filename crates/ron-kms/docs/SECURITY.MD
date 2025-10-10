
---

title: Security Notes — ron-kms
crate: ron-kms
owner: Stevan White
last-reviewed: 2025-10-09
status: draft
-------------

# Security Documentation — ron-kms

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-kms`.
It complements the repo-wide Hardening Blueprint and Interop Blueprint, plus `docs/CONFIG.md` and `IDB.md`.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats                                                                    | Relevant in `ron-kms`? | Mitigation                                                                                                                                                                                                                                                                   |
| -------------------------- | -------------------------------------------------------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Fake clients, forged capabilities, MITM on ingress                         | **Y**                  | TLS 1.3 (tokio-rustls); macaroon **capability tokens** with explicit KMS scopes (`kms:sign`,`kms:verify`,`kms:rotate`,`kms:wrap`); deny ambient authority; mTLS optional intra-mesh; stable `client_id` extraction and `corr_id` propagation                                 |
| **T**ampering              | Mutated requests; key store rollback; audit log edits                      | **Y**                  | Content addressing (`payload_b3 = BLAKE3(payload)`); **append-only audit chain** with per-chunk Merkle roots and **signed checkpoints**; **anti-rollback** counters; fsync on mutate; `deny_unknown_fields` on DTOs                                                          |
| **R**epudiation            | Disputed operations; missing logs                                          | **Y**                  | **Deterministic audit records** `(op_id, tenant, kid@v, alg, purpose, input_hash, ts, result, actor, attestation?)`; stable error envelopes; NTP-disciplined UTC + monotonic clocks for ordering                                                                             |
| **I**nformation Disclosure | Leakage of private keys, plaintext inputs, or policy hints                 | **Y**                  | **Non-exportability** (HSM/TPM/PKCS#11; sealed blobs for file backend); **amnesia mode** (RAM-only) for Micronode; zeroization + page-lock where available; **redaction**: logs/metrics only lengths or BLAKE3 hashes; constant-time compares; no debug format of secrets    |
| **D**enial of Service      | Flooding sign path; slow-loris; compression bombs; per-tenant interference | **Y**                  | Hard **timeouts**, **bounded channels**, **rate limits** (RPS), body caps (1 MiB), decompression ratio cap (≤ 10×), **reject-before-latency** (429), per-tenant quotas (via policy), degraded-mode clarity (sign → 503 with `Retry-After`, verify advertises `cache_age_ms`) |
| **E**levation of Privilege | Using sign to bypass policy; rotating/destroying without authority         | **Y**                  | Strict **scope verification** in KMS; **separation of duties** (policy decisions in `ron-policy`, authentication in `ron-auth`); idempotent mutations bound to client `op_id`; audit trail with actor identity; optional **attestation required** for sign path              |

---

## 2) Security Boundaries

* **Inbound (trust boundary termination):**

  * HTTP/JSON (canonical):

    * `POST /v1/kms/sign`
    * `POST /v1/kms/verify`
    * `POST /v1/kms/wrap` / `POST /v1/kms/unwrap`
    * `POST /v1/kms/keys` (create), `POST /v1/kms/keys/{kid}/rotate`
    * `GET /v1/kms/keys/{kid}` (public material)
  * **Auth**: Bearer macaroon with KMS scopes; KMS validates **capabilities** (not business policy).

* **Outbound (dependencies invoked by `ron-kms`):**

  * **Custody backends:** `pkcs11` (HSM/Yubi), `tpm`, **file** (sealed), `cloud` (wrap/unwrap).
  * **Policy** (`ron-policy`): verify hybrid modes/quotas; returns allow/deny + mode (`OR/AND/PQ-only`).
  * **Time/entropy:** OS `getrandom`, monotonic clock, NTP-disciplined UTC.
  * **Metrics/health:** Prometheus `/metrics`; `/healthz`; `/readyz`. (No external DBs.)

* **Trust Zone:** Privileged service inside the cluster **security tier**; may be exposed to internal meshes only. Public exposure requires explicit review (extra mTLS and WAF/ingress caps).

* **Assumptions:**

  * Kernel services (metrics, config hot-reload, health) are honest and available.
  * Hardware roots (HSM/TPM) enforce non-exportability and attestation semantics.
  * Storage may be compromised **at rest** → tamper-evidence and anti-rollback are mandatory.

---

## 3) Key & Credential Handling

* **Key classes managed/used by `ron-kms`:**

  * **Operational signing keys**: Ed25519; PQ **ML-DSA** (Dilithium); **hybrid** bundles (both).
  * **KEM keys**: X25519; PQ **ML-KEM** (Kyber) for wrap/unwrap (hybrid by default).
  * **AEAD keys**: ChaCha20-Poly1305 or AES-GCM (nonce strategy **explicit**, no reuse per `kid#vN`).
  * **Audit/attestation keys**: bind audit checkpoints to device/slot/firmware when supported.
  * **TLS** (service endpoint): optional; managed externally; hot-reload supported.
  * **Macaroon secrets**: **not stored** by KMS; verification is capability-scope checking (no long-term secret custody in `ron-kms`).

* **Storage & handling:**

  * **Non-exportable** by design. PKCS#11/TPM: object handles only. File backend: sealed blobs (KDF + AEAD), fsync on mutate. **Amnesia mode**: RAM-only, no durable keys.
  * Memory: **zeroize on drop**, avoid `Debug`/`Display` of secrets, **page-lock** when available.
  * **Attestation**: if enabled, sign operations include evidence (or session-scoped evidence) proving key residency.

* **Rotation & revocation:**

  * KID scheme: `tenant/purpose/alg/uuid#vN`. New versions **do not** invalidate old ones for verify.
  * Rotation runbook: policy-gated; **idempotent** via client `op_id`.
  * Audit/attestation key rotation: scheduled; checkpoints prove continuity; record key transition in audit.
  * TLS: rotate ≤ 90 days; macaroon root rotation handled by `ron-auth`.

---

## 4) Hardening Checklist (from Blueprint)

* [x] **Timeouts**: read=5s, write=5s, idle=60s; backend sign deadline ≤ 2s; attestation ≤ 3s; checkpoint ≤ 1s.
* [x] **Concurrency cap**: listener & pool sized; **bounded mpsc** queues (sign: 512, audit: 2048).
* [x] **RPS cap**: `sign_rate_limit_rps` (default 500); reject with 429 before latency stretch.
* [x] **Request body cap**: 1 MiB; **decompression ratio** ≤ 10×; deny unknown fields.
* [x] **AEAD nonce** policy: **explicit**, per-`kid#vN` uniqueness enforced; reject reuse.
* [x] **No blocking in async**: PKCS#11/TPM calls via `spawn_blocking` if they may block.
* [x] **UDS** (if enabled): dir `0700`, socket `0600`, `SO_PEERCRED` allowlist.
* [x] **Chaos**: restart under load; verify stays available via cache TTL; `/readyz` flips on audit checkpoint failure.
* [x] **Logs**: structured JSON; secrets redacted (length/hash only).
* [x] **Clocks**: monotonic for sequencing + UTC for stamps; skew tolerance enforced.

---

## 5) Observability for Security

* **Security-relevant metrics:**

  * `kms_ops_total{op,alg,result}`
  * `kms_reject_total{reason="backpressure"|"unauth"|"pq_policy"|"decompress_cap"|"ratio_cap"}`
  * `kms_audit_integrity_failed_total`
  * `tls_handshake_failures_total` (if TLS enabled at the service)
  * `verify_cache_age_ms` gauge (during degraded verify)
  * `auth_failures_total{service="ron-kms"}` (capability verification failures)

* **Logs (structured JSON):** `service`, `op_id`, `corr_id`, `tenant`, `kid@v`, `result`, `reason`, **never payloads** (only `payload_b3` and lengths), peer address, and **attestation class** when relevant.

* **Health gates:**

  * `/readyz=false` when: audit checkpoint cannot be written/verified; required **attestation** is missing; backend session cannot be re-established; config invalid after reload.
  * `/healthz` shows degraded flags; verify path advertises `cache_age_ms` when reading from stale cache.

---

## 6) Dependencies & Supply Chain

* **Security-sensitive crates (pinned at workspace root):**

  * Crypto: `blake3`, `hkdf`, `subtle`, `zeroize`, `rand_core`/`getrandom`, Ed25519 (e.g., `ed25519-dalek`), PQ suites (e.g., `pqcrypto`, `ml-dsa`/`ml-kem` bindings pinned to vetted forks).
  * Transport: `tokio`, `hyper`/`axum`, `tokio-rustls` (TLS 1.3 only).
  * Serde stack: `serde` + `serde_json` with `deny_unknown_fields` on DTOs.
  * Storage/adapters: `pkcs11` bindings, `tss-esapi` (TPM), filesystem I/O.

* **Controls:**

  * **`cargo-deny`** (licenses/advisories/bans) is mandatory in CI.
  * **SBOM** generated at release; stored under `docs/sbom/`.
  * **Reproducible builds** + version pinning; release gates include KATs and perf deltas.

---

## 7) Formal & Destructive Validation

* **Property/KATs:**

  * Signature correctness per alg; **hybrid** vectors (classical + PQ); HKDF context binding.
  * AEAD nonce uniqueness property test per `kid#vN`.
  * Idempotent mutations (`op_id`) return identical results.

* **Fuzzing:**

  * JSON DTO fuzzers (size caps, unknown fields).
  * Any non-HTTP framing (OAP/1) parsers.

* **Loom (concurrency):**

  * **Rotate vs Sign** races preserve version monotonicity; no lock across `.await`.
  * Audit appender/checkpointer ordering invariants.

* **Chaos drills:**

  * Backend outage: **sign** → 503 + `Retry-After`; **verify** succeeds via cached pubkeys ≤ TTL, and responses include `cache_age_ms`.
  * Restore/rollback simulation: on mismatch, write path refuses; integrity alarm fires.

* **(Optional) TLA+:**

  * Model of `Sign/Rotate/Audit` demonstrating safety (no rollback, monotonic audit index) and liveness (eventual drain under bounded failures).

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Backup:** Core RustyOnions Security ([security@rustyonions.local](mailto:security@rustyonions.local))
* **Disclosure policy:** See repo root `SECURITY.md` (coordinated disclosure; 90-day max SLA; crypto issues prioritized).

---

## 9) Migration & Upgrades

* **PQ migration:** staged via config (`produce_hybrid`, `verify_mode=or|and|pq_only`, `rollout_percent`). Document per-tenant cutovers and rollbacks; measure `kms_reject_total{reason="pq_policy"}` and alg latency histograms.
* **Breaking changes** to auth, attestation, or KID layout require a **major version** and a clear migration path in `CHANGELOG.md`.
* **Deprecations:** keep env aliases ≥ 1 minor; announce EoS window; supply `config` transformer when renaming keys.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + macaroon (scoped)| B(ron-kms)
  B -->|capability check + policy consult| C[ron-policy]
  B -->|custody op| D[HSM/TPM/Sealed Store]
  B -.->|reject unauth/oversized/backpressure| E[Typed Error + Metrics + Audit]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text description:** Client calls `ron-kms` over TLS with a scoped macaroon. KMS validates capability scopes (and consults `ron-policy` for mode/limits), performs custody via HSM/TPM/sealed store, and emits tamper-evident audit records. Unauth/oversized/backpressured requests receive typed errors and are counted in metrics.

---

### Appendix A — Security Defaults (quick ref)

* **Allowed algs:** `ed25519`, `ml-dsa`, `x25519`, `ml-kem` (centrally allow-listed).
* **Modes:** Micronode (amnesia=true, Hybrid-OR), Macronode (HSM, attestation **required**, Hybrid-AND).
* **Observability redaction:** `hashes` by default; `log_inputs=false`.
* **Attestation:** if policy requires and backend supports, **block sign** when missing.
* **Verify-during-outage:** soft TTL with `cache_age_ms` surfaced to callers.

---
