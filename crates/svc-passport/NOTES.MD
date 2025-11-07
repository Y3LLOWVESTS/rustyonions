### BEGIN NOTE - NOVEMBER 7 2025 - 8:50 CST

---

# Carry-Over Notes from `ron-kms` → `svc-passport`

## 0) TL;DR

* `svc-passport` should **delegate all signing/verification** to `ron-kms` (Ed25519 + strict verify), **batch verifications**, and **treat key rotation/versioning as a first-class invariant**.
* Use **versioned KID** (`KeyId`) on every passport; keep **old verifying keys** for historical validation (no-break rotations).
* Ship **soft-seal** (feature-gated) for at-rest secrets and **with-metrics** for ops visibility.
* Expose minimal, boring HTTP with **stable JSON DTOs**; apply **quotas/fair queue** before crypto work; **coalesce/queue** verifies for batching.
* Provide **perf gates**, **property tests**, **chaos tests** (rotate while issuing/verifying), and **interop KATs**.

---

## 1) What from `ron-kms` you should directly leverage

### 1.1 APIs/Traits you can rely on

* **Generate/Sign/Verify** + **Versioned Verify** (strict Ed25519).
* **Batch verify**: single-shot `verify_batch(pks, msgs, sigs)` with:

  * `dalek-batch` (true multiscalar) and
  * `parallel-batch` (Rayon chunking).
* **In-memory keystore** (head signs; history retained). Perfect for tests and dev mode.
* **Attest**: returns alg + key versions; use to publish a JWKS-like response (Ed25519 VK bytes + version).

### 1.2 Features to keep mirrored

* `with-metrics`: counters & histograms (ops, failures, latency, batch_len).
* `soft-seal`: ChaCha20-Poly1305 soft seals for small secrets (no disk spill).
* `dalek-batch` / `parallel-batch`: enable in `svc-passport` to get batch verify.
* `fast` (optional): only if you explicitly want ring signing lanes in the service; default remains dalek.

### 1.3 Performance policy to adopt

* **Batch policy for verify**:

  * `target_batch = 64`, `max_batch = 256`, `max_wait ≈ 500µs`.
* **Threading**: `RAYON_NUM_THREADS ≈ physical cores` (or use global Rayon).
* Expect ~**0.32–0.33 ms** @32 and **~0.73–0.74 ms** @64 per batch on a 2019 MBP i5; **multi-million/s** verifies on big servers (see scaling notes below).

---

## 2) `svc-passport` scope & invariants (recommended)

### 2.1 What `svc-passport` is

* An **authn/authz token issuer & verifier** exposing HTTP routes:

  * **Issue** short-lived “passports” (signed payloads).
  * **Introspect/verify** passports (single or batch).
  * **Publish public keys** (JWKS-like) with **versioned** Ed25519 verifying keys.
  * **Rotate** signing key (admin plane), with **no-break** verification of historical tokens.

### 2.2 Invariants

* **I-1 Strict Verify**: Ed25519 `verify_strict` everywhere—no legacy accepts.
* **I-2 Versioned KID**: Every passport carries a `kid` pointing to the exact version that signed it.
* **I-3 Batching**: Verification requests are **coalesced** up to policy caps before crypto work.
* **I-4 No Ambient Authority**: Every admin/issuer route requires explicit capability (no blanket trust).
* **I-5 Deterministic Envelopes**: Canonical JSON serialization and field ordering for signatures.
* **I-6 Time Safety**: All passports include `{iat, exp}`; clock skew windows are bounded and observable.

---

## 3) Data model & DTOs (stable, boring, versionable)

### 3.1 Canonicalizable payload (the thing we sign)

```json
{
  "iss": "svc-passport",      // issuer name/ID
  "sub": "principal-id",      // subject (user/app/service)
  "aud": ["service-A"],       // audience(s)
  "iat": 1731025200,          // issued-at (unix seconds)
  "exp": 1731028800,          // expiry   (unix seconds)
  "nbf": 1731025200,          // not-before (optional)
  "scopes": ["read:x", "write:y"],   // capabilities/scopes
  "nonce": "random-96bit",    // anti-replay (optional per flow)
  "ctx": { "tenant": "t1" },  // contextual claims (small, signed)
  "kid": "ed25519/t1/auth/v3" // versioned KeyId (MUST match signer)
}
```

**Signature envelope** (Ed25519; message = canonical JSON bytes):

```json
{
  "alg": "Ed25519",
  "kid": "ed25519/t1/auth/v3",
  "sig": "<base64-URL-no-pad>",        // 64-byte signature
  "msg": "<base64-URL-no-pad>"         // canonical JSON bytes of payload
}
```

### 3.2 Endpoints (proposed)

* `POST /v1/passport/issue`
  In: `{sub,aud,scopes,ctx?,ttl_s?,nbf?,nonce?}` → Out: **signature envelope**
* `POST /v1/passport/verify`
  In: **one** envelope → Out: `{ok, reason?}`
* `POST /v1/passport/verify_batch`
  In: `[{...envelope}, ...]` → Out: `[{ok, reason?}, ...]` (same order)
* `GET /v1/keys`
  Out: `{ alg:"Ed25519", current:".../vX", keys:[{kid,vk_b64,created_ms}, ...] }`
* Admin plane (guarded):

  * `POST /admin/rotate` → bumps version; returns new `kid` + attest.
  * `GET /admin/attest` → passthrough of `ron-kms` attest.

**Canonicalization**: use a fixed serializer (sorted keys, UTF-8, no trailing spaces), so `msg` bytes are stable across languages.

---

## 4) Batch verify path (how to wire it)

* Gather incoming `verify(_batch)` requests into an **in-process queue** keyed by **(kid, vk_bytes)**:

  * Same KID → same VK → can be verified together.
* Apply **coalescing** loop:

  * Every `max_wait` or when `target_batch` reached (per key), pop a batch and call `ron-kms::backends::ed25519::verify_batch`.
  * If `parallel-batch` is on, split larger sets into chunks (e.g., 64–256 per chunk).
* **Small-N** fallback: if batch < 8 and near latency SLO, do serial multiscalar verify; otherwise wait until `max_wait` elapses.
* Record `kms_batch_len` histogram for each completion.

---

## 5) Rotation & key discovery

* On **issue**: call `ron-kms` head signer; stamped `kid` is **exact** current version (e.g., `.../v7`).
* On **verify**: if `(kid → vk)` not in the local cache:

  1. Try `ron-kms` keystore lookup;
  2. Else fetch from **`/v1/keys`** (self) which proxies `ron-kms.attest`.
* Cache policy: TTL (~60s) + **versioned** entries are immutable; invalidation only when rotation bumps the **current**.

---

## 6) Threat model & countermeasures

* **Replay**: Include `nonce` for flows that need it; servers may track `jti`/`nonce` in short rolling window (in-memory set or CAS store).
* **Clock skew**: Allow ±`clock_skew_s` on `{iat, nbf, exp}`; expose violations via metrics.
* **Downgrade**: Envelope includes `alg="Ed25519"`; reject anything else (until PQ features are explicitly enabled).
* **Key confusion**: `kid` must match the verifying key bytes used; mismatches are a **hard fail** with stable problem code.
* **Envelope tamper**: Signature over the exact canonical `msg` bytes; never verify recomposed structs.

---

## 7) Metrics & observability (copy the good patterns)

* Counters:

  * `passport_ops_total{op ∈ [issue,verify], result ∈ [ok,fail], alg="ed25519"}`
  * `passport_failures_total{reason ∈ [expired,nbf,scope,nonce,verify,unknown]}`
* Histograms:

  * `passport_op_latency_seconds{op}`
  * `passport_batch_len` (same buckets as KMS)
* Gauges (optional):

  * `passport_cache_items{kind ∈ [vk, jwks]}`

**Logs/Spans**:

* Include `kid`, `batch_len`, `verify_path ∈ [serial,parallel]`, `result`, and `reason`.

---

## 8) Config knobs (v1 that matter)

```toml
[passport]
issuer = "svc-passport"
default_ttl_s = 3600
max_ttl_s     = 86400
clock_skew_s  = 120

[verify]
target_batch = 64
max_batch    = 256
max_wait_us  = 500

[cache]
vk_ttl_s = 60
jwks_ttl_s = 60

[limits]
max_msg_bytes = 4096
max_batch = 512

[security]
require_aud = true
```

---

## 9) Testing plan (carry-over patterns)

### 9.1 Property tests

* **Batch == Single**: For random envelopes, `verify_batch` results must equal single verifies.
* **Rotation safety**: Tokens issued at `vN` must verify during and after rotation to `vN+1`.
* **Canonicalization**: Serialize → sign → re-serialize → verify must hold (no encoder drift).

### 9.2 Chaos tests

* Spawn a rotator (ticks every 50–200 ms) while a verifier goroutine hammers `verify_batch` with mixed-version tokens; assert no spurious failures (beyond intended expiry/NBF).

### 9.3 Interop KATs

* Seed a set of deterministic test vectors (keys, payloads, signatures) and verify:

  * `svc-passport` verify accepts KATs generated by **pure ed25519-dalek** and **ring** (optional).
  * Downstream libs in other languages can validate the `msg` byte canonical form and signature.

### 9.4 Benchmarks (smoke)

* **Latency**: `verify_batch` at {8,32,64,256} (serial & parallel).
* **Throughput**: elements/s at {64,128,256,512}.
* Save **AC-powered baselines** and add absolute **perf gates** (re-use `perf_gate.sh` pattern).

---

## 10) REDACTED

## 11) Dependencies (keep aligned with workspace)

* **Runtime/HTTP**: `axum = 0.7` (features: `tokio,http1,http2,json`), `tokio` (io-util), `tower-http` (metrics/trace if needed).
* **Crypto**: none directly (call `ron-kms`), but for canonicalization you’ll want `serde_json` + deterministic serializer.
* **Metrics**: `prometheus = 0.14` (feature-gated `with-metrics`).
* **Concurrency**: `rayon` (optional; enable with `parallel-batch`), `parking_lot`.
* **Serde/UUID/Time**: `serde`, `serde_json`, `uuid`, `time`.
* **Error**: `thiserror`, `anyhow` (internal only).
* **Security**: `zeroize` for any ephemeral secrets; `soft-seal` feature pulls `chacha20poly1305`.

---

## 12) Error model (stable, machine-parseable)

* `expired` / `nbf` / `bad_aud` / `scope_denied` / `verify_failed` / `malformed` / `unknown_kid` / `internal`
* Always return:

```json
{ "code": "verify_failed", "message": "signature did not verify", "retryable": false }
```

* For batch responses, **result per index**—do not short-circuit entire batch.

---

## 13) Security posture

* **No unsafe** (`#![forbid(unsafe_code)]`).
* **Strict Ed25519** only (until PQ features are explicitly added).
* **Input caps**: `max_msg_bytes`, `max_batch`, decompression guards if you accept compressed bodies.
* **Amnesia option**: optionally add an **amnesia mode** (no persistent caches/logs), mirroring the design elsewhere in RON-CORE.

---

## 14) Performance targets (Beta)

* AC laptop (2019 i5): parallel `verify_batch/32` ≤ **0.80 ms**, `verify_batch/64` ≤ **1.00 ms** (matches `ron-kms`).
* Server (64C): `verify_batch/64` ≈ **30 µs** ballpark; **2M+ verifies/s** @64–256 batches (see scaling section in KMS notes).
* Add `perf_gate.sh` with absolute µs thresholds; integrate into `beta_check.sh`.

---

## 15) Roll-out & migration

* **Rotation without breakage**: publish `/v1/keys` ahead of rotation; after rotation, issue new tokens with new `kid` while keeping historical VKs.
* **Cache warming**: prefetch keys on startup via `attest` or `/v1/keys`.
* **Canary gating**: expose `passport_batch_len` and tail latency percentiles before opening floodgates.

---

## 16) Nice-to-have (post-Beta)

* **Macaroons-like caveats** (time, ip, geo) as deterministic claims.
* **Audience key pinning**: include audience VK fingerprint in claim set for service-to-service trust.
* **PKCS#11** feature to offload sign to HSM (keeps verify on CPU with batch).
* **PQ placeholders** (`mlkem`, `mldsa` enums) for compile-time shape, no runtime yet.
* **CLI**: `svc-passport-cli` to mint/verify/rotate for demos & KAT generation.

---

## 17) Acceptance criteria for `svc-passport` Beta

* ✅ Issue/Verify/Verify-batch routes implemented with strict Ed25519 via `ron-kms`.
* ✅ `/v1/keys` publishes current + historical VKs; `kid` always stamped in tokens.
* ✅ Coalescing batch verifier with policy knobs and `passport_batch_len` metrics.
* ✅ Property, chaos, and interop tests green.
* ✅ No unsafe; clippy clean.
* ✅ Metrics gated behind `with-metrics`; perf gates in scripts.
* ✅ README/RUNBOOK documents canonicalization, batch policy, rotation, and limits.

---

## 18) Commands you’ll actually run

Format, lint, test:

```
cargo fmt -p svc-passport
cargo clippy -p svc-passport --no-deps -- -D warnings
cargo test  -p svc-passport
```

Smoke benches (parallel path):

```
RUSTFLAGS="-C target-cpu=native" \
cargo bench -p svc-passport --features "dalek-batch,parallel-batch" \
  --bench verify_batch -- --measurement-time 4 --sample-size 25
```

Perf gate:

```
chmod +x crates/svc-passport/scripts/perf_gate.sh
crates/svc-passport/scripts/perf_gate.sh
```

---

### END NOTE - NOVEMBER 7 2025 - 8:50 CST