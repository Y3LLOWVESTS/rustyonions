---

crate: svc-crypto
path: crates/svc-crypto
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Local IPC “crypto concierge” that signs, verifies, hashes, and key-manages for other RustyOnions services via a simple Unix socket RPC.

## 2) Primary Responsibilities

* Provide a stable **sign/verify** and **hash/derive** surface to peers over local IPC (UDS).
* **Manage node/app keys** (create, load, rotate, zeroize) with tight FS permissions and optional “amnesia” (ephemeral) mode.
* Validate **capability tokens** (e.g., `CapClaims`) used across services (expiry/nonce/sig) without leaking key material.

## 3) Non-Goals

* No network-facing API, TLS endpoints, or remote HSM/KMS integration (local-only).
* No payment/receipt encoding, billing, or rate computation (belongs in `ryker`/gateway).
* No PQ algorithms by default (optional/behind features only).

## 4) Public API Surface

> Scope is the *service’s* RPC surface (over `ron-bus`/UDS) plus any exported helper types.

* **Re-exports:** (expected minimal) `sha2`, `base64`, `hex` are *not* re-exported; callers use RPC instead of linking libs directly.
* **Key RPCs / DTOs (expected)**

  * `CryptoReq::Sign { key_id, alg, msg } -> CryptoResp::Signature { sig }`
  * `CryptoReq::Verify { key_id?, alg, msg, sig, pk? } -> CryptoResp::Verified { ok }`
  * `CryptoReq::Hash { alg, data } -> CryptoResp::Digest { digest }`
  * `CryptoReq::Random { n } -> CryptoResp::Bytes { data }`
  * `CryptoReq::Keygen { alg, label, persist } -> CryptoResp::KeyInfo { key_id, public }`
  * `CryptoReq::ListKeys -> CryptoResp::Keys { entries }`
  * `CryptoReq::TokenVerify { token_bytes } -> CryptoResp::Token { ok, reason? }`
* **Events / HTTP / CLI:** likely none; startup logs only. (If there’s a small `bin`, it probably just binds the UDS and serves.)

## 5) Dependencies & Coupling

* **Internal crates:**

  * `ron-bus` (tight) — IPC envelope and UDS helpers; replaceable with a split `ron-ipc` if we rename. \[replaceable: yes]
  * Possibly `ron-kernel` for health/metrics bus events (loose). \[replaceable: yes]
* **External (top 5, expected):**

  * `ed25519-dalek` (v2) — signatures; maintained; MIT/Apache; low risk.
  * `sha2` (0.10), `blake3` (optional) — hashing; low risk.
  * `rand`/`rand_chacha` + `getrandom` — CSPRNG; critical; low risk.
  * `zeroize` — secret zeroization; low risk, important.
  * `anyhow`/`thiserror` — error ergonomics; low risk.
  * (Optional) `pqcrypto`/`oqs` behind feature flags — high maintenance risk; large footprints.
* **Runtime services:** OS filesystem (key store), OS RNG; no DB required unless we use sled for metadata.

## 6) Config & Feature Flags

* **Env/config (suggested/typical):**

  * `RON_CRYPTO_SOCK` (default: `/tmp/ron/svc-crypto.sock`).
  * `RON_CRYPTO_KEYDIR` (default: `$XDG_DATA_HOME/ron/keys` or `/var/lib/ron/keys`).
  * `RON_CRYPTO_AMNESIA=1` → in-mem keys, no persistence.
  * `RON_CRYPTO_ALLOWED_ALGS=ed25519,sha256,...` → whitelist for policy.
  * `RON_CRYPTO_MAX_REQ_BYTES` → guardrail against DoS.
* **Cargo features:**

  * `pq` → enable PQ signatures/hashes (Dilithium/Kyber via oqs).
  * `blake3` → enable BLAKE3 hashing.
  * `metrics` → Prometheus counters/histograms.
  * `serde-pubkey` → (if serializing public keys in JSON for tooling).

## 7) Observability

* **Metrics (if `metrics`):**

  * `crypto_requests_total{op,alg,ok}`
  * `crypto_bytes_total{dir=ingress|egress}`
  * `crypto_latency_seconds{op,alg}` histogram
  * `crypto_key_load_failures_total{reason}`
* **Health:** report ready only after key store scan completes and UDS is bound.
* **Tracing:** span per request with `corr_id`, `op`, `alg`, `key_id`; never log secrets/material.

## 8) Concurrency Model

* UDS accept loop (blocking or tokio), one task/thread per client.
* **Backpressure:** OS socket buffers + per-request size checks; optional bounded job channel if requests are offloaded to worker pool.
* **Locks:** `parking_lot::RwLock` around keystore map; `zeroize` on drop.
* **Timeouts/retries:** Deadline per request (e.g., 2s) to avoid hung clients; caller retriable on `io::ErrorKind::TimedOut`.
* **Isolation:** Signing operations are cheap (ed25519), so inline is OK; heavy PQ ops (if enabled) should go to a worker pool.

## 9) Persistence & Data Model

* **Key store:**

  * Directory layout: `${KEYDIR}/{key_id}.json` for metadata + `${key_id}.sk` for secret (0600).
  * Metadata fields: `{ key_id, alg, created_at, label?, rotations[], status }`.
  * Secrets encoded with `base64` and optionally **sealed at rest** (e.g., age/ChaCha20-Poly1305) if `RON_CRYPTO_MASTER` set; otherwise rely on FS perms.
* **Retention:** soft-delete keys (status=disabled) before wipe; configurable purge window.

## 10) Errors & Security

* **Error taxonomy:**

  * `BadRequest` (unknown alg/oversized payload) — terminal.
  * `NotFound` (key or label) — terminal.
  * `VerifyFailed` — terminal, non-retryable.
  * `KeyLocked/Unavailable` — potentially retryable.
  * `Internal` (I/O, RNG, codec) — retryable if transient.
* **AuthN/Z:**

  * Local IPC trust model; *optionally* enforce **peer UID/GID allowlist** via `SO_PEERCRED` on Linux / `LOCAL_PEERCRED` on macOS.
  * Optionally require a shared local **cap token** on each RPC (HMAC over envelope) to protect from unprivileged processes.
* **Secrets handling:** zeroize on drop; avoid swapping secrets to disk (mlock if available/feasible).
* **PQ-readiness:** gated feature; default off to keep binary small and maintenance low.
* **Side-channel hygiene:** constant-time compares via `subtle` for sig equality; avoid logging inputs.

## 11) Performance Notes

* **Hot paths:** ed25519 sign/verify, sha256 hashing; both are microsecond-scale on commodity CPUs.
* Keep RPC envelopes small; max request bytes guard (e.g., 1–4 MiB).
* Avoid cross-thread copies of large inputs; use borrowed buffers or `bytes::Bytes`.
* Expected targets (dev laptop):

  * p95 `Sign(ed25519, 1 KiB)` ≤ 100 µs
  * p95 `Verify(ed25519, 1 KiB)` ≤ 120 µs
  * SHA-256 throughput ≥ 1.2 GB/s single-thread (release).

## 12) Tests

* **Unit:** KATs for ed25519 (RFC 8032 vectors), sha256 vectors; keystore load/rotate; zeroize assertions.
* **Integration:** end-to-end via UDS: spawn service, issue `Sign`/`Verify`/`Hash` RPCs, assert results.
* **Fuzz:** envelope decode (MsgPack) and token parsing; ensure graceful errors (no panics, no OOM).
* **Loom:** not strictly needed unless internal shared state grows; can add for keystore mutation invariants.
* **Security tests:** peer credential rejection; permission errors on weak keyfile modes.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **Naming drift:** “svc-crypto” implies service; if we also need a linkable library for in-process callers, introduce `ron-crypto` (lib) and keep this crate as the IPC façade.
* **IPC trust:** today it’s likely “any local process can call me”; implement **peer-cred checks** and an allowlist.
* **Observability:** add metrics hooks and structured errors; right now most crypto crates bubble opaque errors.
* **Key sealing at rest:** rely on FS perms → add optional envelope encryption with a master key (env or OS keychain).
* **PQ optionality:** define stable RPC enums for PQ so enabling the feature doesn’t cause wire drift.

### Overlap & redundancy signals

* **Dup hash/sign code** appearing in `gateway` or `overlay` should be removed; those crates should call this service (or `ron-crypto` lib) to avoid drift.
* `CapClaims` verification logic currently mentioned in `ron-bus` analysis fits **better here** (or in a dedicated `ron-cap` lib used by svc-crypto).

### Streamlining (merge/extract/simplify)

* Extract **common DTOs** to `ron-proto` to decouple service from transport crate changes.
* Provide **one façade client** (`svc_crypto::Client`) for Rust callers so they don’t need to craft envelopes manually.
* Add **policy guard**: deny unknown algorithms by default unless explicitly whitelisted.

## 14) Change Log (recent)

* 2025-09-14 — First architectural review; defined surface (sign/verify/hash/random/keygen), guardrails (size/alg whitelist), and security hardening plan (peer-cred/allowlist, key sealing).

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — The intent is clear; finalize RPC enums and publish a small client.
* **Test coverage:** 2 — Add KATs, UDS integration tests, and fuzzers.
* **Observability:** 2 — Basic tracing likely present; add metrics/health endpoints.
* **Config hygiene:** 3 — Needs documented envs and defaults; add max-size/timeouts.
* **Security posture:** 3 — Solid primitives expected; improve caller auth and key sealing.
* **Performance confidence:** 4 — Ops are cheap; add benches to confirm targets.
* **Coupling (lower is better):** 3 — Tied to `ron-bus` envelopes; reduce with `ron-proto` and a thin client.

