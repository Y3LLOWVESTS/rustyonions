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