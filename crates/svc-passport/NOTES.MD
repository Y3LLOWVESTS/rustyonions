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




### BEGIN NOTE - NOVEMBER 7 2025 - 13:31 CST


# svc-passport — Carry-over Notes (Beta Sprint)

## Current status (as of last build)

* Workspace pins are unified. `Cargo.toml` now mirrors root pins (Axum 0.7.9, Tower 0.5.2, Tokio 1.x, tower-http 0.6.6). ✅
* `IssuerState` exists and provides the helpers that handlers call:
  `sign`, `verify`, `build_envelope`, `jwks` (stub), `rotate`, `attest`, `verify_envelope`. ✅

  * Base64 decoding updated to the new engine API. ✅
* HTTP handlers compile aside from serve/Router wiring. Most earlier missing-method errors (jwks/rotate/attest/verify_envelope) are resolved via the `IssuerState` implementations. ✅
* **Blocker remaining:** Axum 0.7 server bootstrap error:

  ```
  error[E0277]: Router<IssuerState> does not implement Service<IncomingStream<'_>>
  ```

  This appears whenever we pass a stateful `Router<S>` directly to `axum::serve(listener, app)`.

### Why this error happens (quick diagnosis)

* In Axum 0.7, `Router<()>` implements `Service<IncomingStream<'_>>` and can be passed directly to `axum::serve`.
* `Router<S>` with **non-unit state** (e.g., `Router<IssuerState>`) does **not** implement `Service<IncomingStream<'_>>`.
* **Fix:** Convert the Router into a *make service* before passing it to `axum::serve`, i.e., use `into_make_service()` or `into_make_service_with_connect_info::<SocketAddr>()`.

You previously hit “no method named `into_make_service`” — that typically happens if:

* Axum features are mismatched; or
* A conflicting `tower` version leaks (0.4 vs 0.5); or
* The call was on the wrong type (e.g., after a `.layer()` that changed the type).

We’ve already aligned features/pins. The reliable Axum-0.7 pattern below should compile with our workspace versions.

## Minimal serve pattern (what to use)

Use **one** of these two; both are correct for Axum 0.7:

**A) With client address (recommended for logging/rate-limit by IP):**

```rust
use std::net::SocketAddr;

let make = app.into_make_service_with_connect_info::<SocketAddr>();
let server = axum::serve(listener, make).with_graceful_shutdown(shutdown_signal());
// NOTE: `WithGracefulShutdown` *is* a future here; it's okay to await.
if let Err(e) = server.await {
    tracing::error!("server error: {e}");
}
```

**B) Without connect info:**

```rust
let make = app.into_make_service();
let server = axum::serve(listener, make).with_graceful_shutdown(shutdown_signal());
if let Err(e) = server.await {
    tracing::error!("server error: {e}");
}
```

> If you still see “no method named `into_make_service`”, double-check:
>
> * `axum = { workspace = true, default-features = false, features = ["tokio","http1","http2","json"] }`
> * `tower = { workspace = true, default-features = false, features = ["util"] }` (0.5.2)
> * No local `tower = "0.4.*"` stray pin in any other crate depending chain into `svc-passport`.

## Files we touched / added (conceptually)

* `src/state/issuer.rs`: implemented helpers; base64 engine; mapped KMS errors via `Error::Internal`.
* `Cargo.toml`: aligned to workspace pins; ensured Tower 0.5; kept Hyper local.
* `src/http/handlers/{issue.rs,verify.rs}`: use `IssuerState` helpers; return `(StatusCode, Json(_))`.
* `src/http/router.rs`: `build_router(cfg, health) -> Router<IssuerState>` and routes mounted under `/v1`.

## What’s left for **Beta**

Here’s an explicit checklist with acceptance criteria and where to put things.

### 1) Server bootstrap (blocker)

* [ ] Update `src/bootstrap.rs` to one of the serve patterns above.
* [ ] Ensure we call `.with_state(issuer)` on the **final** router we pass to `into_make_service*`.
* [ ] Import `std::net::SocketAddr` if using connect info.
* [ ] Remove unused imports (the last build flagged `Health` re-import and `Duration`).
  **Done when:** `cargo run -p svc-passport` binds and responds 200 on `/healthz` (or `/version` if present).

### 2) KMS client surface (dev-kms path first)

* [ ] Define the dev KMS (feature `dev-kms`) end-to-end:

  * `sign(msg) -> (kid, sig)`, `verify(kid, msg, sig) -> bool`, `rotate() -> kid`, `attest() -> Value`, `jwks() -> Value`.
  * For Ed25519, return a stable `kid` (e.g., SHA256(pubkey) hex; or `ed25519:<first16hex>`).
* [ ] Unit tests for happy paths + negative signature verification.
  **Done when:** `cargo test -p svc-passport` green with dev-kms enabled.

### 3) API shape (v1)

Routes (all JSON):

* `POST /v1/issue` — body: raw bytes or `{msg_b64}`; returns `Envelope { alg, kid, msg_b64, sig_b64 }`
* `POST /v1/verify` — `{ alg?, kid, msg_b64, sig_b64 }` → `{"ok": true/false}`
* `GET  /v1/keys` — JWKS doc (even if dev KMS returns empty array initially)
* `POST /v1/rotate` — returns `{kid}`
* `GET  /v1/attest` — returns device/attestation JSON if supported (stub otherwise)
* `GET  /version`, `GET /healthz`, `GET /readyz` — admin/ops plane
  **Done when:** Handlers return stable envelopes, consistent errors, and we have example curl snippets in README.

### 4) Error model (stable, no leaks)

* [ ] Map internal failures to `Error::{Malformed, InvalidSignature, Internal, Unauthorized, TooLarge}`.
* [ ] HTTP → Problem envelope: `{code, message, retryable, retry_after_ms?}` for 4xx/5xx.
* [ ] Ensure no secret data in error strings; prefer static reasons.
  **Done when:** All handlers use a single `http::errors` mapping; clippy clean with `-D warnings`.

### 5) Limits & guards

* [ ] Config-driven caps: `limits.max_msg_bytes`, per-route body cap with `tower-http::limit` or custom extractor.
* [ ] Optional `fair_queue` and `concurrency_limit` (tower util) for `/verify` hot path.
  **Done when:** Oversized body returns 413 with a clear problem envelope; concurrency limit visible in logs/metrics.

### 6) Verify batching (optional now, but planned)

* [ ] `POST /v1/verify_batch` accepts `Vec<Envelope>` and returns `Vec<bool>`.
* [ ] Add two features you scaffolded: `dalek-batch` and `parallel-batch`.

  * `dalek-batch`: use dalek batch verify for same-alg Ed25519 vectors.
  * `parallel-batch`: rayon/parallel chunks with per-core verify.
    **Done when:** Criterion bench shows throughput/latency improvements on 1k-10k envelopes.

### 7) Metrics & observability

* [ ] Prometheus counters/histograms:

  * `passport_requests_total{route,method,code}`
  * `passport_verify_latency_seconds` (histogram)
  * `passport_issue_latency_seconds` (histogram)
  * `passport_keys_rotations_total`
  * `passport_verify_batch_size`
* [ ] `/metrics` endpoint behind admin bind.
* [ ] Structured logs: request id, kid, and status; never log raw message/sig.
  **Done when:** `curl /metrics` shows passport_* series and Grafana dashboard can plot verify latency.

### 8) Config & admin plane

* [ ] `Config::load()` (already present) + `PASSPORT_CONFIG`/`PASSPORT_CONFIG_FILE`.
* [ ] Admin bind (separate from public bind) optional; if single bind, prefix `/admin/*`.
* [ ] `GET /version` returns `{crate, version, git?, build?}`.
  **Done when:** We can flip caps/limits via env var and see the effect without code changes.

### 9) Security & correctness

* [ ] `Security { require_aud: bool }` honored in verify: if true, enforce an `aud` field in envelope or a wrapped JWT/JWS form (design decision: either keep our simple envelope + optional `aud` header, or add a JWT route).
* [ ] Strict alg check (only `Ed25519` until we add others).
* [ ] Timing-safe comparisons in verify (dalek handles this, but be explicit if we ever do manual compare).
  **Done when:** Negative tests cover wrong kid, wrong sig, wrong alg, missing fields.

### 10) Docs, runbook, and examples

* [ ] `README.md` with:

  * Quickstart (dev-kms): `cargo run -p svc-passport`
  * Example issue/verify/keys/rotate/attest curl snippets
  * Config table with defaults
  * Limit/guard behavior
  * Prometheus series reference
* [ ] `RUNBOOK.md`:

  * Healthz/readyz meanings
  * Rotating keys safely in prod
  * Backup/export of active keys for external HSM migration (future)
    **Done when:** A new contributor can get responses from all routes in under 5 minutes.

### 11) Tests & benches

* [ ] Unit tests: `issuer.rs`, `handlers`, error mapping.
* [ ] Integration tests: end-to-end issue→verify (good/bad sigs), rotate, jwks.
* [ ] Criterion benches:

  * `issue_single`, `verify_single`
  * `verify_batch_{1,8,64,512,4096}` (with/without features)
    **Done when:** CI executes tests; benches stored with baselines (e.g., `passport-2025-11-07`).

### 12) Future (post-beta)

* HSM/KMS backends: pkcs11, AWS KMS, YubiHSM2.
* Real JWKS (JWK export of Ed25519 in OKP form).
* JWS/JWT convenience endpoints (optional).
* Revocation list integration (with `svc-auth`/`revocation.rs`).
* Attestation pass-through for specific hardware.

## Quick reference: things we fixed earlier (keep)

* **Router state generic:** `build_router` returns `Router<IssuerState>`, and we attach the state with `.with_state(issuer)`. Do **not** coerce to `Router<()>`.
* **Base64 engine:** `use base64::{engine::general_purpose::STANDARD, Engine as _};` and `STANDARD.encode/decode`.
* **Error typing:** Map `anyhow::Error` from KMS to crate `Error::Internal` so signatures match.
* **Clippy discipline:** `-D warnings` is on; keep imports lean to avoid build-breakers.

## Concrete next patch (when we resume)

* In `src/bootstrap.rs`, replace the serve block with:

```rust
use std::net::SocketAddr;

pub async fn run(cfg: Config, issuer: IssuerState) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(&cfg.server.bind).await?;
    let app = build_router(cfg.clone(), /* health */ Health::new()).with_state(issuer);

    let make = app.into_make_service_with_connect_info::<SocketAddr>();
    let server = axum::serve(listener, make).with_graceful_shutdown(shutdown_signal());

    if let Err(e) = server.await {
        tracing::error!("server error: {e}");
        return Err(anyhow::Error::msg(e.to_string()));
    }
    Ok(())
}
```

* Verify `router.rs` signature:
  `pub fn build_router(cfg: Config, _health: Health) -> Router<IssuerState> { ... }`

That’s everything we need queued up. When we jump back in, we’ll paste the serve change, run, and then finish the admin plane + metrics and the verify-batch fast path.


### END NOTE - NOVEMBER 7 2025 - 13:31 CST





### BEGIN NOTE - NOVEMBER 7 2025 - 15:45 CST

```BASH

mymac@RustyOnionsDevNet RustyOnions % cargo fmt -p svc-passport
cargo clippy -p svc-passport --no-deps -- -D warnings
crates/svc-passport/scripts/smoke_passport.sh --spawn

    Checking ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel)
    Checking ron-policy v0.1.0-beta.1 (/Users/mymac/Desktop/RustyOnions/crates/ron-policy)
    Checking svc-passport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-passport)
    Finished `dev` profile [optimized + debuginfo] target(s) in 2.87s
[INFO] Spawning svc-passport (cargo run -p svc-passport)
[INFO] svc-passport spawned (pid=43870); logs at /tmp/svc-passport.log
[ OK ] Service is up at http://127.0.0.1:5307
[INFO] Target: http://127.0.0.1:5307
[ OK ] /healthz
[ OK ] /v1/keys
[ OK ] issue → envelope
[ OK ] verify (single)
[ OK ] verify (batch)
[ OK ] admin/rotate
[ OK ] admin/attest
[ OK ] /metrics (present)
[ OK ] negative: tampered → 400
[ OK ] negative: over-limit → 413
[ OK ] All checks passed
[INFO] Stopping spawned svc-passport (pid=43870)
```


🔥 Green wall achieved! That’s the snapshot we wanted.

# svc-passport beta — what’s left & % complete

**Estimated completion:** **92–95%** ✅
You’ve got the working surface, guards, metrics, negatives, and smoke harness all passing end-to-end.

## Tiny checklist to cross the beta line

1. **Unit tests (fast)**

* Add a couple of surgical unit tests:

  * `issuer.sign/verify` happy/negative
  * `verify_one` malformed b64 → `400`
  * batch over max → `413`
* Gate: `cargo test -p svc-passport` green.

2. **Criterion microbenches (nice-to-have for beta)**

* Benches: `issue_single`, `verify_single`, `verify_batch_{8,64,512}`.
* Gate: `cargo bench -p svc-passport -- --save-baseline passport-<date>` runs and records.

3. **CI wire-up (quick)**

* Add job that runs: fmt → clippy (`-D warnings`) → tests → smoke:

  * `crates/svc-passport/scripts/smoke_passport.sh --spawn`
* Gate: CI green on PR.

4. **JWKS realism (optional-now)**

* Current `/v1/keys` works; if you want “beta+”, export OKP/Ed25519 with `x`, `crv: Ed25519`, `kty: OKP`, and stable `kid`.

5. **Config polish (very small)**

* Verify env overrides for limits & concurrency are read (they are).
* Add a `PASSPORT_MAX_BATCH` env hook if not already.

6. **Security pass (quick)**

* Ensure we never log payloads/sigs; only `kid` and result labels (you’re good).
* Confirm error messages are static/non-leaky (they are).

If you want to stamp **beta today**: you can, with (1) and (3) checked. Benches & “real JWKS” can be “beta.1”.



### END NOTE - NOVEMBER 7 2025 - 15:45 CST





### NEW NOTE - NOVEMBER 7 2025 - 18:43 CST


---

# svc-passport — Carry-Over Notes (Beta Sprint)

## 0) TL;DR of where we are

* **Green runtime** (manual + smoke): service starts, core API works, negative paths enforced, metrics present.

  * `smoke_passport.sh --spawn` ends with “**All checks passed**” (we’ve seen this multiple times).
  * Endpoints exercised: `/healthz`, `/v1/keys`, `/v1/passport/issue`, `/v1/passport/verify`, `/v1/passport/verify_batch`, `/admin/rotate`, `/admin/attest`, `/metrics`.
* **Bootstrap fixed** for Axum 0.7 (`Router<()>` served directly; state via `Extension(Arc<IssuerState>)`).
* **Handlers**:

  * `issue`: canonical claim serialize → `sign` (dev KMS) → stable envelope (alg/kid/msg_b64/sig_b64) ✅
  * `verify`: preflight time window → (optional) audience gate → sig verify ✅
  * `verify_batch`: same logic per-item with batch cap and metrics ✅
* **Config-driven limits** in router: `DefaultBodyLimit::max(..)`, `ConcurrencyLimitLayer` on hot paths ✅
* **Metrics**: present and wired:

  * `passport_ops_total{op,result,alg}`
  * `passport_op_latency_seconds` (histogram)
  * `passport_batch_len` (histogram) ✅
* **Smoke script**: spawns service, probes all routes, runs positive and negative cases, and stops process ✅

**Current blocking errors** (only in your latest run):

1. **Integration tests** are failing because they expect `tests/../src/test_support.rs` but that file doesn’t exist.
2. **Benches** show `Unrecognized option: 'save-baseline'` → Criterion not configured.

These are tooling/test harness issues, not service logic.

**Completion estimate (Beta): ~92–94%**. What remains is minor glue (tests/benches harness), a quick JWKS touch-up, and a short QA pass. (Docs can be deferred per your instruction.)

---

## 1) Service surface (Beta scope)

**Public API (`/v1`)**

* `POST /v1/passport/issue` → returns envelope `{alg,kid,msg_b64,sig_b64}`
* `POST /v1/passport/verify` → `{"ok": true|false}` (preflight + optional audience)
* `POST /v1/passport/verify_batch` → `{"results":[bool,...]}` (caps + metrics)
* `GET /v1/keys` → JWKS (currently stubbed via dev KMS view)

**Admin/ops**

* `GET /healthz` (constant ok for probe)
* `GET /metrics` (Prometheus exposition; working)
* `POST /admin/rotate` (dev rotate; returns new `kid`)
* `GET /admin/attest` (dev attestation stub)

**Guards & Limits**

* Per-route **body cap** via `DefaultBodyLimit::max(max_body_bytes)`
* **Concurrency** via `ConcurrencyLimitLayer` on `/verify` and `/verify_batch`
* Preflight **time window** & (optional) **audience** enforced before crypto

**Observability**

* Prometheus counters/histograms wired; no high-card labels; healthy.

---

## 2) What we changed along the way (keep these invariants)

* Router is **unit-state** (`Router<()>`), we pass issuer via `Extension(Arc<_>)`. This keeps `axum::serve` happy on 0.7.
* **Base64** uses new engine trait: `use base64::Engine;` then `general_purpose::STANDARD.encode/decode`.
* Errors map to a small, stable set (no secrets in messages).
* Negative paths:

  * Tampered → **400**
  * Over-limit → **413**
  * Invalid signature → **ok=false** (verify) or typed error if we choose to expose reason in a problem envelope later.

---

## 3) Your latest terminal output (quick triage)

```
error: couldn't read tests/../src/test_support.rs
  in tests/handlers.rs and tests/issuer.rs

error: Unrecognized option: 'save-baseline' (when running cargo bench)
```

**Cause**:

* The tests expect **`crates/svc-passport/src/test_support.rs`** to exist (they include it via `#[path="../src/test_support.rs"] mod test_support;`), but it’s missing.
* Criterion isn’t in `Cargo.toml` (`cargo bench` default harness doesn’t know `--save-baseline`).

**Fix plan (fast):**

1. **Create** `src/test_support.rs` (tiny helper that builds an `IssuerState` using dev KMS).
2. **(Optional but helpful)** keep a parallel `tests/test_support.rs` for tests that use `mod test_support;` without a `#[path=...]`.
3. **Add Criterion** in `[dev-dependencies]` and declare the bench target (`harness=false`).

---

## 4) Exact drop-in fixes to unbreak tests & benches

### A) Create: `crates/svc-passport/src/test_support.rs`

```rust
// crates/svc-passport/src/test_support.rs
use crate::{
    config::Config,
    kms::client::{DevKms, KmsClient},
    state::issuer::IssuerState,
};
use std::sync::Arc;

pub fn issuer_state_for_tests() -> IssuerState {
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let cfg = Config::load().expect("Config::load() in tests");
    IssuerState::new(cfg, kms)
}

pub fn default_config() -> Config {
    Config::load().expect("Config::load() in tests")
}
```

> This satisfies tests that do:
>
> ```rust
> #[path = "../src/test_support.rs"]
> mod test_support;
> ```

### B) (Optional) Create: `crates/svc-passport/tests/test_support.rs`

```rust
// crates/svc-passport/tests/test_support.rs
use std::sync::Arc;
use svc_passport::{
    config::Config,
    kms::client::{DevKms, KmsClient},
    state::issuer::IssuerState,
};

pub fn issuer_state_for_tests() -> IssuerState {
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let cfg = Config::load().expect("Config::load() in integration tests");
    IssuerState::new(cfg, kms)
}

pub fn default_config() -> Config {
    Config::load().expect("Config::load() in integration tests")
}
```

> This covers tests that simply do `mod test_support;` **without** `#[path=...]`.

### C) Update benches in `Cargo.toml`

Add Criterion and bench target:

```toml
[dev-dependencies]
reqwest = { workspace = true, features = ["rustls-tls-native-roots","json"] }
tokio   = { workspace = true, features = ["macros","rt-multi-thread","time"] }
criterion = "0.5"

[[bench]]
name = "verify"
harness = false
```

If the bench file expects `Config::default()`, either:

* change the bench to call `Config::load()`, or
* implement `Default for Config` (trivial wrapper around `load()` with env-free defaults). I recommend **calling `Config::load()`** to stay consistent with runtime behavior.

**Bench run:**

```
cargo bench -p svc-passport -- --save-baseline passport-$(date +%Y%m%d)
```

---

## 5) Beta Checklist (what’s left)

### A) Tests (unit + integration)

* [ ] Ensure `tests/handlers.rs` and `tests/issuer.rs` compile by resolving `test_support` include path (see Section 4).
* [ ] Add negative test for **bad alg** (anything ≠ `Ed25519` should be rejected or return `ok=false`).
* [ ] Add negative test: **missing/empty `aud`** when `require_aud=true` → `400` (BadAudience).
* [ ] Add test: batch over `max_batch` → `413`.

**Acceptance:** `cargo test -p svc-passport` is green.

### B) JWKS polish (dev KMS)

* [ ] Make sure `/v1/keys` returns **OKP** keys for Ed25519 with stable `kid` (e.g., `sha256(pub)` truncated hex or the `ed25519/default/v1` we used). Dev KMS can export public key as JWK `{kty:"OKP", crv:"Ed25519", x:"base64url(pub)", kid:"..."}`.
* [ ] If JWKS is already present via `IssuerState::jwks()`, sanity-check one curl output.

**Acceptance:** `curl /v1/keys | jq .keys[0].kty=="OKP"` returns `true`.

### C) Config edges

* [ ] Confirm `PASSPORT_MAX_MSG_BYTES`, `PASSPORT_VERIFY_CONCURRENCY`, `PASSPORT_VERIFY_BATCH_CONCURRENCY` read and applied.
* [ ] Confirm `security.require_aud` toggle works (env or toml) and flips behavior.

**Acceptance:** Toggle env then smoke → tampered/audience checks behave accordingly.

### D) Metrics coverage (quick)

* [ ] `passport_ops_total{op=issue}` increments.
* [ ] `passport_ops_total{op=verify}` increments ok and fail.
* [ ] Histograms show samples for issue/verify latency; `passport_batch_len` shows batch sizes.

**Acceptance:** `curl /metrics` has non-zero counters after running smoke.

### E) Benches (optional but nice for Beta)

* [ ] Criterion harness compiles and runs.
* [ ] Minimal scenarios: `verify_single`, `verify_batch_{8,64,512}` (even if dalek-batch isn’t hooked yet).

**Acceptance:** Baseline artifact saved; no panic.

---

## 6) Commands we actually run (golden sequence)

Local green build + smoke:

```
cargo fmt -p svc-passport
cargo clippy -p svc-passport --no-deps -- -D warnings
cargo run -p svc-passport  # (or use the smoke script to spawn)

# in another terminal
crates/svc-passport/scripts/smoke_passport.sh           # uses existing instance
crates/svc-passport/scripts/smoke_passport.sh --spawn   # spawns & kills for you
```

Full CI-style sweep:

```
cargo fmt -p svc-passport
cargo clippy -p svc-passport --no-deps -- -D warnings
cargo test   -p svc-passport
cargo bench  -p svc-passport -- --save-baseline passport-$(date +%Y%m%d)
crates/svc-passport/scripts/smoke_passport.sh --spawn
```

Env toggles you can play with:

```
PASSPORT_MAX_MSG_BYTES=1048576
PASSPORT_VERIFY_CONCURRENCY=64
PASSPORT_VERIFY_BATCH_CONCURRENCY=16
PASSPORT_CONFIG=... or PASSPORT_CONFIG_FILE=...
```

---

## 7) Known pitfalls (so we don’t regress)

* **Axum 0.7 serve state**: keep router unit-state + `Extension(Arc<IssuerState>)`. Don’t switch back to `Router<IssuerState>`.
* **Base64 trait**: always `use base64::Engine;` before calling `.decode()` or `.encode()`.
* **Tower 0.5**: make sure no crate pulls in `tower 0.4`. We pinned `tower` with features `["util","limit"]`.
* **Handler signatures**: keep them Axum-style `(State(..), Json(..)) -> Result<impl IntoResponse, Error>`.
* **Error mapping**: keep small, stable codes; don’t log raw message/sig; no secret leakage.

---

## 8) “Open items” we intentionally deferred (Post-Beta or Nice-to-Have)

* **Real HSM/KMS backends** (PKCS#11, AWS KMS, YubiHSM2)
* **Batch verify accelerators**:

  * `dalek-batch` feature (same-alg optimization)
  * `parallel-batch` feature (rayon chunking)
* **Bus RPC** (if we want the service callable over RON-bus internally)
* **JWT/JWS convenience endpoints** (wrappers over envelope)
* **Revocation/allow-list hooks** (ties to `ron-auth` later)
* **Docs**: README/RUNBOOK/API markdown (you said skip now)

---

## 9) First 15 minutes next session (lock Beta)

1. **Create** `src/test_support.rs` (Section 4A).
2. **(Optional)** Add `tests/test_support.rs` (Section 4B) or adjust your test modules to point to the src one.
3. **Add Criterion** to `Cargo.toml` (Section 4C).
4. Run:

   ```
   cargo fmt -p svc-passport
   cargo clippy -p svc-passport --no-deps -- -D warnings
   cargo test -p svc-passport
   cargo bench -p svc-passport -- --save-baseline passport-$(date +%Y%m%d)
   crates/svc-passport/scripts/smoke_passport.sh --spawn
   ```
5. **JWKS glance**: `curl :5307/v1/keys | jq .` and verify OKP/Ed25519 shape.

If all green → **tag Beta** for `svc-passport`.

---

## 10) Status stamp (today)

* Runtime & smoke: **GREEN**
* Metrics: **present**
* Router bootstrap (Axum 0.7): **solid**
* Verify/Issue/Batch/Admin: **working**
* Tests: **RED** (path mismatch for `test_support`)
* Benches: **RED** (Criterion not wired)


### END NOTE - NOVEMBER 7 2025 - 18:43 CST



### BEGIN NOTE - NOVEMBER 8 2025 - 9:20 CST

---

# svc-passport — Carry-Over Notes (Beta Sprint)

## 0) TL;DR (current state)

* **Service boots and passes smoke end-to-end**
  `scripts/smoke_passport.sh --spawn` → ✅ `/healthz`, `/metrics`, `/v1/keys`, `issue`, `verify`, `verify_batch`, tamper negative path (saw 400), and teardown clean.
* **Core handlers work**: `issue`, `verify`, `verify_batch`, `keys`, `admin/rotate`, `admin/attest` (dev stub) are alive.
* **Axum 0.7 bootstrap is correct**: router built and served with unit state; limits/concurrency layers wired.
* **Metrics endpoint** (`/metrics`) exposes counters/histograms we’ve added so far; more coverage planned (see §5D).
* **Tests**: The integration tests that build a Router are **green when config is injected via env** (we fixed loader behavior).
* **Benches**: Criterion harness runs. Typical laptop numbers (2019 MBP i5 1.4GHz, wall power):

  * `verify_single`: ~**36–37 µs**
  * `verify_batch_64`: ~**2.30–2.36 ms**
* **Config**: `config/default.toml` good for dev. Test harness now finds config via env or falls back to the file.

**Beta progress estimate:** **~88–90%**. Core path is solid; remaining work is polish, persistence, negative-path rigor, and QA.

---

## 1) What’s running (surface area, behavior)

### Public API (`/v1`)

* `POST /v1/passport/issue` → stable envelope `{ alg, kid, msg_b64, sig_b64 }` (Ed25519).
* `POST /v1/passport/verify` → `true|false`. Preflight time window, optional `aud` gating per config.
* `POST /v1/passport/verify_batch` → `Vec<bool>`; batch caps; metrics tracked.
* `GET /v1/keys` → JWKS (dev-backed; see polish items).

### Admin/ops

* `GET /healthz` → 200 (liveness).
* `GET /metrics` → Prometheus exposition (present).
* `POST /admin/rotate` → dev key rotation returns new `kid`.
* `GET /admin/attest` → dev attestation stub (to be enriched).

### Guards, limits, concurrency

* Per-route **body caps** via `DefaultBodyLimit::max(..)`.
* Hot paths wrapped with **`ConcurrencyLimitLayer`**.
* **Time window** and **optional audience check** happen **before** crypto.

---

## 2) Recent fixes & important invariants (keep them)

* **Axum 0.7 `to_bytes`** in tests requires a limit: `to_bytes(body, usize::MAX)`.
* **Config loader for tests** looks at `PASSPORT_CONFIG` / `PASSPORT_CONFIG_FILE` and otherwise points at `crates/svc-passport/config/default.toml`.
  (This fixed the “No such file or directory” panics when running tests out of tree.)
* **Smoke script** improvements:

  * Honors `PASSPORT_CONFIG` / `PASSPORT_CONFIG_FILE`.
  * Can **kill a busy port** holder, or run on a **dynamic port** via `PASSPORT_DYNAMIC_PORT=1`.
  * Verifies negative tamper path (accepts either `200:false` or `400`).
* **Base64** always through `use base64::Engine` and `general_purpose::STANDARD` to match 0.22 API.
* **Small, stable error surface** (no secret leakage).

---

## 3) Config that matters right now

`crates/svc-passport/config/default.toml`:

```toml
[server]
bind = "127.0.0.1:5307"
admin_bind = "127.0.0.1:5308"

[passport]
issuer = "svc-passport"
default_ttl_s = 3600
max_ttl_s = 86400
clock_skew_s = 120

[verify]
target_batch = 64
max_batch = 256
max_wait_us = 500

[cache]
vk_ttl_s = 60
jwks_ttl_s = 60

[limits]
max_msg_bytes = 4096
max_batch = 512

[security]
require_aud = true
```

**Env precedence** (current effective order in our test harness & scripts):

1. `PASSPORT_CONFIG` (inline TOML)
2. `PASSPORT_CONFIG_FILE` (path to TOML)
3. default to repo file: `crates/svc-passport/config/default.toml`

---

## 4) How to run it (golden commands)

**Local dev / CI sweep**

```bash
cargo fmt -p svc-passport
cargo clippy -p svc-passport --no-deps -- -D warnings
cargo test -p svc-passport
```

**Smoke (fixed port)**

```bash
PASSPORT_CONFIG_FILE=crates/svc-passport/config/default.toml \
crates/svc-passport/scripts/smoke_passport.sh --spawn
```

**Smoke (avoid collisions)**

```bash
PASSPORT_DYNAMIC_PORT=1 PASSPORT_CONFIG_FILE=crates/svc-passport/config/default.toml \
crates/svc-passport/scripts/smoke_passport.sh --spawn
```

**Benches (Criterion)**

```bash
cargo bench -p svc-passport --bench verify -- --measurement-time 10 --sample-size 100
# optional: save named baseline
cargo bench -p svc-passport --bench verify -- --save-baseline passport-$(date +%Y%m%d)
```

**Handlers test (explicit config)**

```bash
PASSPORT_CONFIG="$(cat crates/svc-passport/config/default.toml)" \
cargo test -p svc-passport --test handlers
```

---

## 5) What’s left for Beta (actionable checklist)

### A) API/JWKS/rotation polish

* [ ] **JWKS correctness**: ensure `/v1/keys` returns **OKP/Ed25519** with proper `kid`, `crv:"Ed25519"`, and base64url `x`. Verify the `kid` matches the signer used by `issue`.
* [ ] **Rotation grace**: keep old public keys visible for a short window after `/admin/rotate` so older passports still verify.
  *Acceptance:* after rotate, previously issued envelope still verifies; `/v1/keys` contains both `kid`s until grace expires.
* [ ] **Optional persistence for rotation** (Post-Beta if needed): simple file or sled store to preserve active/previous `kid` across restart.

### B) Limits/validation tests (tighten negative paths)

* [ ] Enforce `limits.max_msg_bytes` with `413` on `issue`. Add unit/integration tests.
* [ ] Enforce `verify.max_batch` and `limits.max_batch` consistently (return `413` or `400` with clear problem reason). Add tests.
* [ ] **Alg fencing**: reject any `alg` ≠ `Ed25519` with a typed error (or `ok=false`), and test it.
* [ ] **Audience gating** when `[security].require_aud=true`:

  * Missing/empty `aud` → 400 (BadAudience).
  * Wrong `aud` → `false` or 400 (pick and document). Add tests.

### C) Observability (metrics + logs)

* [ ] Counters: `passport_ops_total{op=issue|verify|verify_batch, result=ok|fail, alg}`.
* [ ] Histograms: `passport_op_latency_seconds{op}`, `passport_batch_len`.
* [ ] Cache metrics: `passport_vk_cache_{hits,misses}`, `passport_jwks_cache_{hits,misses}`.
* [ ] Rotation: `passport_rotations_total`, and a gauge for active keys.
* [ ] Structured logs: include route, `kid`, result, and request id; **no** secrets.

### D) Readiness/health/shutdown

* [ ] `/readyz` (if not present yet) that gates on listeners bound, config loaded, caches warm (optional).
* [ ] Graceful shutdown (SIGINT/SIGTERM) drains inflight requests and reports not ready before exit. Quick test via `scripts/smoke_passport.sh`.

### E) Config & docs

* [ ] Document **search order** for config (inline TOML env, file env, default file) in README.
* [ ] Add minimal **runbook**: rotate procedure, key grace, how to restore state, how to check JWKS.
* [ ] Production switches: disable `dev-kms`, point to real KMS/HSM (future), recommended TLS, log redaction.

### F) Bench/QA

* [ ] Bench matrix (single vs batch 8/64/256) and stable baseline stored (use `critcmp` locally).
* [ ] Spot-check batch perf under `parallel-batch` (feature-gated) with `RAYON_NUM_THREADS=$(sysctl -n hw.physicalcpu)`—document caveats.

---

## 6) Acceptance criteria for declaring **Beta**

* `cargo fmt && cargo clippy -D warnings && cargo test` → **green** on CI (Linux/macOS).
* `scripts/smoke_passport.sh --spawn` → **All checks passed** (fixed and dynamic port).
* `/v1/keys` enumerates active and (during grace) previous KIDs; round-trip verify succeeds before and after rotation.
* Limits enforced with correct status codes; negative tests in place (oversize, bad alg, missing/invalid `aud`, over-batch).
* Metrics present for hot paths; logs structured; no secret leakage.

---

## 7) Known pitfalls (avoid regressions)

* **Tests failing to find config**: always set one of
  `PASSPORT_CONFIG` or `PASSPORT_CONFIG_FILE` in CI. We solved this in `src/test_support.rs`, but CI envs can still bite if unset.
* **Axum 0.7 body bytes**: `to_bytes(body, limit)`—not the old hyper helper.
* **Tower 0.5 vs 0.4**: keep workspace pins aligned to avoid split.
* **Dev-KMS only for dev**: make sure builds targeting production don’t accidentally ship dev signer.

---

## 8) Benchmark quick notes (so we don’t lose context)

* Laptop (2019 MBP i5-8257U, plugged in):

  * `verify_single`: **~36–37 µs**
  * `verify_batch_64`: **~2.30–2.36 ms**
* Expect **2–4× faster** single verify and larger gains for batch on a modern server core (AVX2/fast turbo, higher clocks, better cache).
* Use:

  ```
  cargo bench -p svc-passport --bench verify -- --measurement-time 10 --sample-size 100
  ```

  Save baselines: `--save-baseline passport-YYYYMMDD`

---

## 9) Next 3 commits (surgical plan)

1. **JWKS + attest polish**

   * Correct OKP/Ed25519 JWKS w/ stable `kid`; enrich `/admin/attest` with build, features, active/prev KIDs.
   * Add a test that `issue` → `verify` round-trips using `/v1/keys`.

2. **Limits + alg/aud tests**

   * Enforce `limits.max_msg_bytes` & batch limits with proper codes; add negative tests.
   * Add tests for bad/missing `alg`, and `aud` behaviors per config.

3. **Metrics coverage**

   * Counters + histograms + cache metrics; grep `/metrics` in smoke to assert non-zero after run.

*(Optional 4th) Rotation persistence (simple file store) with grace window and restart test.*

---

## 10) Troubleshooting quick ref

* **Smoke times out but port is bound**:
  Use dynamic port:
  `PASSPORT_DYNAMIC_PORT=1 PASSPORT_CONFIG_FILE=... scripts/smoke_passport.sh --spawn`
* **Bench errors about `save-baseline`**: ensure running the **bench target** (not the bin).
  `cargo bench -p svc-passport --bench verify -- --save-baseline ...`
* **Handlers test panics on config**:
  `PASSPORT_CONFIG="$(cat crates/svc-passport/config/default.toml)" cargo test -p svc-passport --test handlers`

---

## 11) “Where we stopped” stamp (today)

* Smoke: **GREEN** (all checks passed).
* Tests: **GREEN** for router handlers when env config is provided.
* Benches: **GREEN** and stable; numbers recorded.
* Remaining: JWKS polish, limits/negatives rigor, metrics coverage, (optional) rotation persistence, readiness/shutdown niceties, README/runbook.

> Once the JWKS/attest polish + limits/aud/alg tests + metrics coverage are in and smoke stays green, we can **stamp Beta**.


### END NOTE - NOVEMBER 8 2025 - 9:20 CST




### BEGIN NOTE - NOVEMBER 8 2025 - 10:05 CST


# svc-passport — Carry-Over Notes (Beta Complete + Next Steps)

*Last updated: 2025-11-08 (America/Chicago)*

## 0) TL;DR

* **Status:** ✅ **Beta complete.**

  * Smoke script passes end-to-end (fixed & dynamic port).
  * Unit & integration tests are green (limits, alg/aud, JWKS round-trip).
  * JWKS endpoint returns correct media type, cache headers, and **ETag** (BLAKE3).
  * Negative paths enforced (oversize, over-batch, bad alg, missing/empty `aud`, tamper).
* **Benchmarks (2019 MBP i5-8257U):**

  * `verify_single`: **36.3–36.6 µs**
  * `verify_batch_64`: **2.252–2.265 ms** (~35 µs/item)
* **API surface:** `/v1/passport/{issue,verify,verify_batch}`, `/v1/keys`, `/admin/{rotate,attest}`, `/healthz`, `/metrics`.
* **Security posture:** Ed25519 only; strict input validation; audience gating (optional); no secret leakage in errors; JWKS cacheable with strong opaque `ETag`.

---

## 1) What’s implemented (and stable)

### Public API (`/v1`)

* `POST /v1/passport/issue`
  Input: arbitrary JSON payload (we sign the exact bytes).
  Output: envelope `{ alg:"Ed25519", kid, msg_b64, sig_b64, aud? }`.
  Enforces `limits.max_msg_bytes` (413 on exceed).
  If `[security].require_aud=true`, includes default `aud = passport.issuer`.

* `POST /v1/passport/verify`
  Input: envelope above.
  Fences `alg` (must be `Ed25519`), `kid` non-empty, and (optionally) `aud` presence.
  Output: `true | false`. Tamper or invalid → 400 with stable problem code.

* `POST /v1/passport/verify_batch`
  Accepts **either** `{ "items": [Envelope, ...] }` **or** bare array `[Envelope, ...]`.
  Enforces `limits.max_batch` (413 on exceed).
  Per-item return: `Vec<bool>`.

* `GET /v1/keys` (JWKS)
  Returns **OKP/Ed25519** keys with current `kid` (and prior, during grace when enabled).
  Headers:

  * `Content-Type: application/jwk-set+json`
  * `Cache-Control: public, max-age=<jwks_ttl_s>`
  * `ETag: "<blake3(hex over exact JSON)>"`
    Conditional `If-None-Match` supported → `304 Not Modified`.

### Admin/Ops

* `GET /healthz` — basic liveness.
* `GET /metrics` — Prometheus exposition with op counters/histograms (see Metrics).
* `POST /admin/rotate` — dev key rotation returns new `current_kid`.
* `GET /admin/attest` — dev attestation doc including build profile/version and KID view.

### Guards, limits, concurrency

* Per-route **body caps** (`DefaultBodyLimit::max`).
* **ConcurrencyLimitLayer** on verify paths (env-tunable).
* Early rejects for `alg`, `kid`, `aud`, and oversize payloads (pre-crypto).

---

## 2) Config (effective keys and precedence)

File: `crates/svc-passport/config/default.toml` (dev values)

```toml
[server]
bind = "127.0.0.1:5307"
admin_bind = "127.0.0.1:5308"

[passport]
issuer = "svc-passport"
default_ttl_s = 3600
max_ttl_s = 86400
clock_skew_s = 120

[verify]
target_batch = 64
max_batch = 256
max_wait_us = 500

[cache]
vk_ttl_s = 60
jwks_ttl_s = 60

[limits]
max_msg_bytes = 4096
max_batch = 512

[security]
require_aud = true
```

**Env precedence** used by loader/tests/scripts:

1. `PASSPORT_CONFIG` (inline TOML)
2. `PASSPORT_CONFIG_FILE` (path)
3. Default: `crates/svc-passport/config/default.toml`

Env overrides (examples) used by router guards:

* `PASSPORT_MAX_MSG_BYTES` (body limit layer)
* `PASSPORT_VERIFY_CONCURRENCY`, `PASSPORT_VERIFY_BATCH_CONCURRENCY`

---

## 3) How to run (dev/CI/ops)

### Local dev & CI sweep

```
cargo fmt -p svc-passport
cargo clippy -p svc-passport --no-deps -- -D warnings
cargo test -p svc-passport
```

### Smoke (fixed port)

```
PASSPORT_CONFIG_FILE=crates/svc-passport/config/default.toml \
crates/svc-passport/scripts/smoke_passport.sh --spawn
```

### Smoke (dynamic port to avoid collisions)

```
PASSPORT_DYNAMIC_PORT=1 PASSPORT_CONFIG_FILE=crates/svc-passport/config/default.toml \
crates/svc-passport/scripts/smoke_passport.sh --spawn
```

### Specific tests we care about for Beta gates

```
cargo test -p svc-passport --test limits --test audience_alg --test jwks_roundtrip
```

### Benchmarks (Criterion)

```
cargo bench -p svc-passport --bench verify -- --measurement-time 10 --sample-size 100
# Optional (avoid the “100 samples in 10s” warning for batch_64):
cargo bench -p svc-passport --bench verify -- --measurement-time 18 --sample-size 100
# Save baseline:
cargo bench -p svc-passport --bench verify -- --save-baseline passport-$(date +%Y%m%d)
```

---

## 4) How to use svc-passport with other RON crates

### With `ron-kernel` / `ron-metrics`

* svc-passport exposes `/metrics` (Prometheus). Point your cluster scraper to the advertised address (default `127.0.0.1:5307/metrics`).
* Health checks: `/healthz` for liveness. (If you adopt `/readyz`, use that for readiness; see Nice-to-haves.)

### With `ron-kms` (future, production)

* Current dev build uses an in-process **DevKms** signer (Ed25519) via an abstract `KmsClient` trait.
* To plug real KMS/HSM: implement `KmsClient` (sign/attest/rotate) and inject at bootstrap.
* Rotation invariant: **never break verification**—keep old verify keys published in JWKS for a grace window and retained in the verifier.

### With policy/gateway (`ron-policy`, `svc-gateway`, `omnigate`)

* Audience gating integrates cleanly: issue with `aud` and configure `[security].require_aud=true` to ensure verifiers reject envelopes without `aud`.
* Gateway layers can coalesce/queue verify requests and call `verify_batch` for higher throughput.
* Policy can short-circuit at the edge when `alg != Ed25519` or `kid` is unknown (cheap rejects before crypto).

---

## 5) Metrics you’ll see

* Counters:
  `passport_ops_total{op="issue|verify|verify_batch|keys", result="ok|fail|not_modified", alg="Ed25519|n/a"}`

* Histograms:
  `passport_op_latency_seconds{op}` — end-to-end handler latency
  `passport_batch_len` — observed batch sizes on `/verify_batch`

**Smoke sanity:** after a run you should see non-zero `issue`, `verify`, `verify_batch`, `keys` counters; `keys{result="not_modified"}` increments if you hit `If-None-Match`.

---

## 6) Security posture (Beta)

* **Algorithms:** Only `Ed25519` accepted/issued.
* **Input validation:**

  * `alg` must be `Ed25519` → 400 `BadAlg`
  * `kid` must be non-empty → 400 `BadKid`
  * `aud` required when `[security].require_aud=true` → 400 `BadAudience`
  * `msg_b64`, `sig_b64` must decode → 400 `Malformed`
* **Size/abuse guards:**

  * `limits.max_msg_bytes` enforced on `issue` (413)
  * `limits.max_batch` enforced on `verify_batch` (413)
  * Concurrency caps on hotpaths (env-tunable)
* **Error hygiene:** stable problem envelopes; no secret leakage.
* **JWKS caching:** strong opaque `ETag` (BLAKE3 over exact JWKS bytes), `Cache-Control: public,max-age=<jwks_ttl_s>`.

---

## 7) Example usage (cURL)

Issue:

```
curl -sX POST http://127.0.0.1:5307/v1/passport/issue \
  -H 'Content-Type: application/json' \
  -d '{"sub":"user123","roles":["basic"]}'
```

Verify (single):

```
curl -sX POST http://127.0.0.1:5307/v1/passport/verify \
  -H 'Content-Type: application/json' \
  -d '{"alg":"Ed25519","kid":"<kid>","msg_b64":"<...>","sig_b64":"<...>","aud":"svc-passport"}'
```

Verify (batch — array form):

```
curl -sX POST http://127.0.0.1:5307/v1/passport/verify_batch \
  -H 'Content-Type: application/json' \
  -d '[{"alg":"Ed25519","kid":"<kid>","msg_b64":"<...>","sig_b64":"<...>","aud":"svc-passport"}]'
```

JWKS + conditional GET:

```
curl -si http://127.0.0.1:5307/v1/keys
# copy ETag value from response, then:
curl -si http://127.0.0.1:5307/v1/keys -H 'If-None-Match: "<etag-from-previous>"'
```

---

## 8) Benchmarks — how to run & interpret

Run:

```
cargo bench -p svc-passport --bench verify -- --measurement-time 10 --sample-size 100
```

Numbers we saw on a 2019 MBP i5-8257U:

* `verify_single`: 36.3–36.6 µs (≈27–28k verifications/sec/core)
* `verify_batch_64`: 2.252–2.265 ms (≈35 µs/item; batching overhead negligible)

On modern desktop/server cores, expect materially lower µs (and higher k/s). Use `--measurement-time 18` for batch to avoid Criterion’s sample-time warning.

To store and compare baselines:

```
cargo bench -p svc-passport --bench verify -- --save-baseline passport-$(date +%Y%m%d)
```

---

## 9) Troubleshooting quick ref

* **Smoke times out but port is bound:**
  Use dynamic port:
  `PASSPORT_DYNAMIC_PORT=1 PASSPORT_CONFIG_FILE=... scripts/smoke_passport.sh --spawn`

* **Tests can’t find config:**
  Set one of:
  `PASSPORT_CONFIG="$(cat crates/svc-passport/config/default.toml)"` **or**
  `PASSPORT_CONFIG_FILE=crates/svc-passport/config/default.toml`

* **400 on verify during smoke with `BadAudience`:**
  Ensure `[security].require_aud=true` and payload includes `aud` (issue handler auto-sets `aud = passport.issuer`).

---

## 10) Nice-to-haves beyond Beta (polish / productionization)

### A) Rotation grace + persistence

* **Grace window:** Keep both current and previous verify keys in `/v1/keys` for `N` seconds after `/admin/rotate`.
* **Persistence:** Store active/previous KIDs (and public keys) on disk (sled/file) to survive restarts during/after rotation.
* **Acceptance:** Pre-rotate envelopes continue to verify post-rotate; JWKS lists both KIDs during grace; survives process restart.

### B) `/readyz` and graceful shutdown

* Add `/readyz` that flips to **ready** only after listeners bind and (optionally) JWKS cache warms.
* On SIGINT/SIGTERM: gate readiness to false, stop accepting, drain in-flight, then exit.

### C) Extended observability

* Add cache metrics: `passport_vk_cache_{hits,misses}`, `passport_jwks_cache_{hits,misses}` if/when caches are introduced.
* Add `passport_rotations_total` and a gauge for active keys.
* Structured logs: include route/op, `kid`, and request id (no sensitive fields).

### D) Hardening & Interop

* Strict JWKS conformance tests (OKP/Ed25519, base64url params) against reference validators.
* Fuzz inputs (invalid base64, huge fields, unexpected types).
* Rate limits/quotas/fair-queue before crypto (if fronted by gateway).

### E) KMS integration (non-dev)

* Implement `KmsClient` for hardware modules (e.g., PKCS#11 wrapper) and a remote signer path; add chaos tests that rotate while issuing/verifying.

### F) SDK helpers

* Thin client helpers in `ron-app-sdk` for building/validating envelopes and handling retries/304 on JWKS with cache.

---

## 11) “Why this design works” (principles we’re keeping)

* **Minimal, boring surface:** Tiny set of well-defined endpoints; stable JSON DTOs.
* **Cheap rejects before crypto:** Body caps, alg/aud/kid validation → less wasted crypto cycles.
* **Batch where it matters:** `verify_batch` for throughput; array or `{items:[]}` accepted for ergonomic clients.
* **Cacheability:** JWKS with solid ETag + max-age reduces load and latency across the fleet.
* **Separation of concerns:** Dev KMS now; real KMS later via `KmsClient` without changing HTTP/API.

---

## 12) “Beta acceptance” stamp (what we met)

* `cargo fmt && cargo clippy -D warnings && cargo test` → **green** on macOS; Linux CI expected to be green with the same flow.
* `scripts/smoke_passport.sh --spawn` → **All checks passed** (issue, verify, batch verify, tamper negative).
* JWKS: correct headers; `ETag` support; round-trip test confirms key listed matches signer KID.
* Limits enforced with correct status codes; negative tests present and passing.
* Metrics exposed for hot paths.

> From here, the “nice-to-haves” (grace + persistence, `/readyz`, extended metrics/logs, stronger interop tests) are incremental and won’t destabilize the public API.

— End of notes —



### END NOTE - NOVEMBER 8 2025 - 10:05 CST



### BEGIN NOTE - MAY 6 2026 - 17:55 CST



## Set 4 — `svc-passport` Crate Notes

Drop this into the `svc-passport` crate notes.

---

# `svc-passport` Crate Carryover Notes

## NEXT_LEVEL / WEB3_2 Username Claim + Public Profile Route Foundation

## 1. Crate role

`svc-passport` is the RustyOnions identity/capability service.

Its long-term responsibilities include:

```text
passport issuance
passport verification
capability token issue/verify/revoke flow
public profile claim/read model
username claim ownership
future persistent passport/profile store
future local/passport custody integration
future alt-passport semantics
```

During this session, `svc-passport` became the first backend service to own actual `@username` profile claim behavior.

Important boundary:

```text
svc-passport owns identity/profile claim truth.
svc-passport does not mutate wallets.
svc-passport does not mutate ron-ledger.
svc-passport does not issue ROC.
svc-passport does not expose private keys or wallet spend authority in public profile responses.
```

Correct route chain after this session:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-passport
```

Incorrect route chain:

```text
CrabLink
→ svc-passport
```

CrabLink must never call `svc-passport` directly.

---

## 2. Session summary

In this session, `svc-passport` gained a deterministic username/profile claim core and HTTP routes for local profile claim/read proof.

Main achievements:

```text
Added in-memory UsernameClaimStore.
Added username normalization and validation.
Added reserved username rejection.
Added duplicate username rejection.
Added one-username-per-passport-subject rule.
Added idempotent repeat claim behavior.
Added read-only PublicProfileResponse.
Added profile claim/read HTTP handlers.
Added profile route tests.
Fixed old handler test contract drift.
Cleaned scaffold tests for Clippy.
Got svc-passport all-target tests green.
Got svc-passport focused Clippy green.
```

New local service routes:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
GET  /v1/passport/profile/_debug
```

These routes are now proxied by:

```text
omnigate:
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username

svc-gateway:
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

---

## 3. Files changed / added

Primary files touched:

```text
crates/svc-passport/src/lib.rs
crates/svc-passport/src/profile.rs
crates/svc-passport/src/http/handlers/mod.rs
crates/svc-passport/src/http/handlers/profile.rs
crates/svc-passport/src/http/router.rs
crates/svc-passport/src/test_support.rs
crates/svc-passport/tests/profile_claims.rs
crates/svc-passport/tests/profile_routes.rs
crates/svc-passport/tests/handlers.rs
crates/svc-passport/tests/api_issue.rs
crates/svc-passport/tests/api_verify.rs
crates/svc-passport/tests/api_revoke.rs
crates/svc-passport/tests/invariants.rs
crates/svc-passport/tests/readiness.rs
```

---

## 4. Public module additions

`src/lib.rs` now exports:

```rust
pub mod profile;
```

The crate root header was updated to reflect that `svc-passport` now includes:

```text
Ed25519 passport issue/verify
short-TTL capability tokens
username/profile claim helpers
public profile route foundation
```

Important invariant added to crate root intent:

```text
no wallet spend authority in profile DTOs
no private alt linkage
deterministic profile claims
```

---

## 5. New core module: `profile.rs`

New file:

```text
crates/svc-passport/src/profile.rs
```

This is the core of the new profile work.

It intentionally starts as an in-memory claim store for Phase 3 / local proof. It is not yet the final durable production store.

Important types:

```rust
UsernameClaimStore
UsernameClaimRequest
UsernameClaimRecord
PublicProfileResponse
UsernameClaimStatus
PassportKind
ProfileClaimError
```

Important helper functions:

```rust
normalize_username(input: &str) -> Result<String, ProfileClaimError>
normalize_handle(input: &str) -> Result<String, ProfileClaimError>
```

Important constants:

```rust
PUBLIC_PROFILE_SCHEMA: &str = "svc-passport.public-profile.v1";

USERNAME_MIN_BYTES: usize = 3;
USERNAME_MAX_BYTES: usize = 32;
PASSPORT_SUBJECT_MAX_BYTES: usize = 256;
DISPLAY_NAME_MAX_BYTES: usize = 96;
PROFILE_BIO_MAX_BYTES: usize = 1024;
PUBLIC_REF_MAX_BYTES: usize = 512;
```

---

## 6. Username claim behavior

`UsernameClaimStore::claim_main_username()` implements current claim semantics.

Request shape:

```rust
pub struct UsernameClaimRequest {
    pub passport_subject: String,
    pub requested_username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_image: Option<String>,
}
```

Expected JSON request shape:

```json
{
  "passport_subject": "passport:main:skinnycrabby",
  "requested_username": "@SkinnyCrabby",
  "display_name": "Skinny Crabby",
  "bio": "Building the content-addressed creator web.",
  "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
}
```

Current behavior:

```text
@ prefix is accepted.
Username is normalized to lowercase.
Handle is returned with @.
Reserved usernames reject.
Confusing username shapes reject.
Duplicate username rejects.
One passport subject cannot claim two different usernames.
Repeating the same username claim for the same passport subject is idempotent.
avatar_image must use crab:// when provided.
created_at_ms must be nonzero.
```

Example:

```text
Input requested_username: @SkinnyCrabby
Stored username: skinnycrabby
Stored handle: @skinnycrabby
Stored profile_crab_url: crab://@skinnycrabby
Status: confirmed
```

---

## 7. Username validation rules

The username rules in `svc-passport` currently mirror the staged project policy:

```text
3–32 bytes
ASCII lowercase after normalization
can contain a-z
can contain 0-9
can contain _
can contain -
can contain .
must start with ASCII alphanumeric
must not end with '.', '-', or '_'
must not contain spaces
must not contain consecutive dots
must not contain reserved names
```

Reserved examples:

```text
admin
alt
api
app
article
asset
assets
b3
comment
crab
gateway
image
mail
manifest
mod
moderator
music
passport
podcast
post
profile
root
ron
site
sites
stream
support
sys
system
video
wallet
```

Important note:

```text
Long-term, this logic should ideally converge on ron-naming::RonUsername to avoid parser drift.
For this session, duplicate validation was acceptable to stage the work safely.
```

---

## 8. Public claim record

Internal service-local record:

```rust
pub struct UsernameClaimRecord {
    pub passport_subject: String,
    pub passport_kind: PassportKind,
    pub username: String,
    pub handle: String,
    pub username_status: UsernameClaimStatus,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_image: Option<String>,
    pub profile_crab_url: String,
    pub public_profile_cid: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}
```

Important current status:

```text
passport_kind is Main for claim_main_username().
username_status is Confirmed after successful claim.
public_profile_cid is None for now.
```

`public_profile_cid` remains null because public profile b3 manifest publication is not implemented yet.

---

## 9. Public profile response

Read-only response:

```rust
pub struct PublicProfileResponse {
    pub schema: &'static str,
    pub passport_subject: String,
    pub passport_kind: PassportKind,
    pub username: String,
    pub handle: String,
    pub username_status: UsernameClaimStatus,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_image: Option<String>,
    pub profile_crab_url: String,
    pub public_profile_cid: Option<String>,
    pub reputation_score: Option<u32>,
    pub moderator_score: Option<u32>,
    pub warnings: Vec<String>,
}
```

Example successful response:

```json
{
  "schema": "svc-passport.public-profile.v1",
  "passport_subject": "passport:main:skinnycrabby",
  "passport_kind": "main",
  "username": "skinnycrabby",
  "handle": "@skinnycrabby",
  "username_status": "confirmed",
  "display_name": "Skinny Crabby",
  "bio": "Building the content-addressed creator web.",
  "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image",
  "profile_crab_url": "crab://@skinnycrabby",
  "public_profile_cid": null,
  "reputation_score": null,
  "moderator_score": null,
  "warnings": [
    "public profile is read-only",
    "reputation and moderation scores are not computed yet"
  ]
}
```

Important truth rule:

```text
reputation_score remains None until real backend scoring exists.
moderator_score remains None until real backend scoring exists.
svc-passport must not fake rep/mod values.
```

Important privacy rule:

```text
PublicProfileResponse must not expose:
private_key
seed_phrase
spend_authority
wallet_spend_authority
private_alt_mapping
parent_passport
main_passport_subject
```

Tests check this.

---

## 10. New HTTP handler module

New file:

```text
crates/svc-passport/src/http/handlers/profile.rs
```

New handlers:

```rust
claim_profile()
get_profile()
profile_debug()
```

Routes added in router:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
GET  /v1/passport/profile/_debug
```

### `POST /v1/passport/profile/claim`

Purpose:

```text
Claim a main-passport username in the current in-memory store.
```

Returns:

```text
201 Created + PublicProfileResponse
```

Important note:

```text
This is a Phase 3 local core route, not the final durable production persistence path.
```

### `GET /v1/passport/profile/:username`

Purpose:

```text
Read a public profile by username or handle.
```

Returns:

```text
200 OK + PublicProfileResponse
```

or:

```text
404 Not Found
code: profile_not_found
retryable: false
```

### `GET /v1/passport/profile/_debug`

Purpose:

```text
Tiny safe local debug endpoint.
```

Returns:

```json
{
  "schema": "svc-passport.profile-debug.v1",
  "profile_routes": [
    "POST /v1/passport/profile/claim",
    "GET /v1/passport/profile/:username"
  ],
  "wallet_mutation": false,
  "ledger_mutation": false,
  "private_keys": false,
  "alt_linkage": false
}
```

---

## 11. HTTP error mapping

`ProfileClaimError` maps to route problem responses.

Important mappings:

### Reserved username

```text
HTTP 400
code: reserved_username
retryable: false
```

### Invalid username shape

```text
HTTP 400
code examples:
username_too_short
username_too_long
invalid_username_start
invalid_username_character
consecutive_dots
invalid_username_trailing_punctuation
```

### Duplicate username

```text
HTTP 409
code: username_unavailable
retryable: false
```

### Passport already owns another username

```text
HTTP 409
code: passport_already_has_username
retryable: false
```

### Invalid avatar URL

```text
HTTP 400
code: invalid_crab_url
retryable: false
```

### Store poisoned / corrupt

```text
HTTP 500
code: store_poisoned or store_corrupt
retryable: true for store_poisoned
```

Problem body shape:

```rust
pub struct ProfileProblem<'a> {
    pub code: &'a str,
    pub message: &'a str,
    pub retryable: bool,
}
```

---

## 12. Router integration

File touched:

```text
crates/svc-passport/src/http/router.rs
```

`build_router()` now constructs:

```rust
let profile_store = Arc::new(UsernameClaimStore::new());
```

and adds it as an Axum extension:

```rust
.layer(Extension(profile_store))
```

New route section:

```rust
.route("/v1/passport/profile/_debug", get(profile::profile_debug))
.route(
    "/v1/passport/profile/claim",
    post(profile::claim_profile).route_layer(DefaultBodyLimit::max(max_body_bytes)),
)
.route("/v1/passport/profile/:username", get(profile::get_profile))
```

Important Clippy fix:

```text
Removed #[must_use] from build_router because Router is already #[must_use].
```

This fixed `clippy::double_must_use`.

---

## 13. Handler test contract cleanup

File touched:

```text
crates/svc-passport/tests/handlers.rs
```

This test was updated to match the current actual `svc-passport` beta contract.

Current issue route behavior:

```text
POST /v1/passport/issue
returns flat Ed25519 envelope:
{
  "alg": "Ed25519",
  "kid": "...",
  "msg_b64": "...",
  "sig_b64": "..."
}
```

Current verify behavior:

```text
POST /v1/passport/verify
accepts flat envelope + top-level aud when required
returns bool
```

Current verify_batch behavior:

```text
POST /v1/passport/verify_batch
accepts bare array of flat envelopes
returns Vec<bool>
```

Important fixes:

```text
Verify request includes required aud from cfg.passport.issuer.
sig_b64 uses standard Base64, not URL-safe Base64.
Batch test tampers sig_b64 using STANDARD base64 engine.
```

Correct import:

```rust
use base64::{engine::general_purpose::STANDARD, Engine as _};
```

Important lesson:

```text
svc-passport currently emits standard Base64 in sig_b64.
Do not decode sig_b64 with URL_SAFE_NO_PAD in tests.
```

---

## 14. Test support cleanup

File touched:

```text
crates/svc-passport/src/test_support.rs
```

Added:

```rust
#![allow(dead_code)]
```

Reason:

```text
Integration tests include this file selectively with #[path = "../src/test_support.rs"].
Some tests use issuer_state_for_tests().
Some tests use default_config().
A helper can appear unused depending on the integration test crate.
```

This is acceptable for test support only.

---

## 15. Scaffold test cleanup

Old `assert!(true)` scaffold tests were replaced because Clippy rejects constant assertions under `-D warnings`.

Files touched:

```text
crates/svc-passport/tests/api_issue.rs
crates/svc-passport/tests/api_verify.rs
crates/svc-passport/tests/api_revoke.rs
crates/svc-passport/tests/invariants.rs
crates/svc-passport/tests/readiness.rs
```

New test behavior:

```text
api_issue.rs checks /v1/passport/issue path stability.
api_verify.rs checks /v1/passport/verify path stability.
api_revoke.rs checks /v1/passport/revoke path stability.
invariants.rs checks attenuation scaffold never widens a scope set.
readiness.rs checks /readyz route path stability.
```

This cleaned Clippy while keeping placeholder tests meaningful.

---

## 16. Tests added

### `tests/profile_claims.rs`

Covers core store behavior:

```text
normalizes_username_and_handle
rejects_reserved_and_confusing_usernames
claim_main_username_returns_confirmed_public_profile
repeated_same_passport_same_username_is_idempotent
duplicate_username_for_different_passport_rejects
same_passport_cannot_claim_second_username
public_profile_unknown_username_returns_none
public_profile_json_does_not_leak_private_fields
```

Important semantics proved:

```text
Username normalization works.
Reserved usernames reject.
Duplicate username by different passport rejects.
Same passport cannot claim a different second username.
Same passport same username is idempotent.
Public profile response is read-only and privacy-safe.
Rep/mod scores remain None.
```

### `tests/profile_routes.rs`

Covers HTTP behavior:

```text
profile_debug_route_is_safe
claim_then_get_public_profile
duplicate_username_returns_conflict
reserved_username_rejects
unknown_profile_returns_404
```

Important route behavior proved:

```text
POST /v1/passport/profile/claim returns 201.
GET /v1/passport/profile/:username returns public profile.
Duplicate username returns 409.
Reserved username returns 400.
Unknown profile returns 404.
Debug route exposes no private state.
```

---

## 17. Green proof from this session

`svc-passport` passed:

```bash
cargo fmt -p svc-passport
cargo test -p svc-passport --test profile_claims
cargo test -p svc-passport --test profile_routes
cargo test -p svc-passport --test handlers
cargo test -p svc-passport --all-targets
cargo clippy -p svc-passport --all-targets --no-deps -- -D warnings
```

Important green results:

```text
profile_claims.rs: 8 passed
profile_routes.rs: 5 passed
handlers.rs: 2 passed
all-targets: passed
focused clippy: passed
```

Current status:

```text
GREEN
```

---

## 18. Relationship to `ron-naming`

Current state:

```text
ron-naming owns canonical username/crab link parser grammar.
svc-passport currently has local username validation for profile claims.
```

This is acceptable for the staged implementation, but long-term we should avoid drift.

Future cleanup target:

```text
Use ron-naming::RonUsername inside svc-passport profile claim code if dependency boundaries permit.
Keep reserved username list consistent.
Keep validation error mapping deterministic.
```

Be careful:

```text
Do not introduce dependency cycles.
Do not pull runtime service behavior into ron-naming.
Do not weaken svc-passport route-level error taxonomy.
```

---

## 19. Relationship to `ron-proto`

Current state:

```text
ron-proto defines PassportPublicProfileV1 and PassportPublicManifestV1.
svc-passport currently returns its own PublicProfileResponse route shape.
```

This was intentional staging.

Future convergence target:

```text
Map UsernameClaimRecord into ron_proto::PassportPublicProfileV1.
Eventually publish PassportPublicManifestV1 as a b3-backed public profile manifest.
```

Potential future evolution:

```text
PublicProfileResponse remains lightweight service route response.
PassportPublicProfileV1 becomes canonical manifest/profile object.
Omnigate may hydrate public profile pages from PassportPublicProfileV1.
CrabLink may display either route response or manifest-backed DTO depending on route.
```

Need to avoid breaking current tests when converging.

---

## 20. Relationship to Omnigate and Gateway

After this session, `svc-passport` profile routes are proxied by Omnigate and svc-gateway.

Route chain:

```text
svc-gateway:
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

omnigate:
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username

svc-passport:
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```

`svc-passport` should not know about CrabLink or gateway routing.

It only owns profile semantics.

---

## 21. Current limitations

Important limitations at the end of this session:

```text
Username claims are in-memory only.
Claims do not survive service restart.
No durable profile DB yet.
No migration/replay/audit log yet.
No profile update route yet.
No profile delete/revoke route yet.
No b3 public profile manifest publication yet.
public_profile_cid remains None.
Reputation score remains None.
Moderator score remains None.
No real site role computation.
No alt passport claim semantics.
No main-passport/alt privacy mapping.
No public proof/signature model yet.
No profile capability/policy gate beyond route validation.
```

These are expected for the current staged milestone.

---

## 22. Required future work for `svc-passport`

### A. Durable username/profile store

The current in-memory store is only a proof.

Future production requirement:

```text
Persist username claims durably.
Reject duplicates across restarts.
Make claims replayable/auditable.
Define migration format.
Define crash-safety behavior.
```

Possible storage design:

```text
username -> passport_subject
passport_subject -> username
profile record by passport_subject
claim event log
```

Need deterministic recovery.

### B. Idempotency for HTTP profile claim route

The core store makes same passport/same username idempotent, but the HTTP route does not yet use idempotency-key explicitly.

Future route work:

```text
Honor Idempotency-Key.
Return same response for repeated same idempotency key.
Reject conflicting body for reused key if policy requires.
```

### C. Profile update route

Needed eventually:

```text
PATCH /v1/passport/profile/:username
```

or similar.

Should update:

```text
display_name
bio
avatar_image
public_payout_account maybe
public site refs maybe
```

Must not allow:

```text
username ownership transfer without explicit policy
private alt leakage
wallet spend authority changes
```

### D. Public profile manifest publication

Future route or background process should publish:

```text
PassportPublicManifestV1
```

as a b3 object.

Then:

```text
public_profile_cid = Some(b3:...)
crab://<hash>.profile resolves to manifest-backed page
crab://@username resolves to current profile
```

### E. Reputation/moderation integration

Current response has:

```text
reputation_score: None
moderator_score: None
```

Future:

```text
Read real scores from reputation/moderation subsystem.
Do not compute fake values.
Include source/timestamp if using ron-proto summaries.
```

### F. Alt passport semantics

Future requirements from NEXT_LEVEL:

```text
Main passports can create sites.
Alt anonymous passports can browse/surf/comment.
Alt passports should not create sites.
Public alt profile should not reveal main passport.
Private mapping must remain private/local/controlled.
```

`svc-passport` likely needs explicit alt APIs later, but do not add casually.

### G. Capability/policy gates

Profile claim route eventually needs real policy/capability gating:

```text
Who can claim a username?
Can only main passport claim?
Does bootstrap automatically claim?
What bearer/capability is required?
How are conflicts audited?
Can usernames be released?
Can usernames be transferred?
```

Current route is local proof/dev profile claim surface.

### H. Use `ron-proto` DTOs

Future:

```text
Return or convert to ron_proto::PassportPublicProfileV1.
Publish ron_proto::PassportPublicManifestV1.
Use ron-proto validation before publication.
```

---

## 23. Important invariants to preserve

Do not add to `svc-passport`:

```text
wallet balance mutation
ROC issue/burn/transfer
ledger direct append
storage writes without explicit design
gateway-specific behavior
CrabLink-specific behavior
fake reputation values
fake moderation scores
private key exposure
seed phrase exposure
private alt linkage in public responses
site ownership enforcement before policy design
```

Always preserve:

```text
fail closed
deterministic error codes
safe public DTOs
no private identity leaks
no wallet spend authority in profile responses
no locks across await
bounded body sizes on mutating routes
```

---

## 24. Known good route examples

### Claim profile

```bash
curl -sS -X POST http://127.0.0.1:5307/v1/passport/profile/claim \
  -H 'content-type: application/json' \
  -d '{
    "passport_subject": "passport:main:skinnycrabby",
    "requested_username": "@SkinnyCrabby",
    "display_name": "Skinny Crabby",
    "bio": "Building the content-addressed creator web.",
    "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
  }' | jq .
```

Expected:

```text
HTTP 201
schema = svc-passport.public-profile.v1
username = skinnycrabby
handle = @skinnycrabby
username_status = confirmed
profile_crab_url = crab://@skinnycrabby
reputation_score = null
moderator_score = null
```

### Get profile

```bash
curl -sS http://127.0.0.1:5307/v1/passport/profile/skinnycrabby | jq .
```

Expected:

```text
HTTP 200
schema = svc-passport.public-profile.v1
username = skinnycrabby
handle = @skinnycrabby
```

### Missing profile

```bash
curl -i http://127.0.0.1:5307/v1/passport/profile/missing
```

Expected:

```text
HTTP 404
code = profile_not_found
retryable = false
```

### Reserved username

```bash
curl -i -X POST http://127.0.0.1:5307/v1/passport/profile/claim \
  -H 'content-type: application/json' \
  -d '{"passport_subject":"passport:main:site","requested_username":"@site"}'
```

Expected:

```text
HTTP 400
code = reserved_username
retryable = false
```

---

## 25. Suggested future acceptance gates

Before future `svc-passport` changes:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p svc-passport
cargo test -p svc-passport --test profile_claims
cargo test -p svc-passport --test profile_routes
cargo test -p svc-passport --all-targets
cargo clippy -p svc-passport --all-targets --no-deps -- -D warnings
```

When changing proxy chain too:

```bash
cargo test -p omnigate --test passport_profile_routes
cargo test -p svc-gateway --test identity_routes_proxy
```

---

## 26. Current status at end of session

Current `svc-passport` status:

```text
GREEN
```

Feature status:

```text
Username normalization: complete for current phase
Username claim core: complete for current in-memory phase
Duplicate rejection: complete for current phase
Reserved name rejection: complete for current phase
One username per passport: complete for current phase
Read-only public profile response: complete for current phase
HTTP claim/read routes: complete for current phase
Durable persistence: pending
Public profile b3 manifest publication: pending
Real reputation/mod scoring: pending
Alt passport semantics: pending
Capability/policy hardening: pending
```

Next relevant project step involving `svc-passport`:

```text
CrabLink should consume the gateway route:
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

Then a later backend batch should add:
crab://@username resolution through Omnigate /crab/resolve.
```


### END NOTE - MAY 6 2026 - 17:55 CST