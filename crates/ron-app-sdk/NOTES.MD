### BEGIN NOTE - NOVEMBER 14 2025 - 18:29 CST

# CARRY-OVER NOTES — `ron-app-sdk` (Beta Push)

**Beta completion estimate:** ~40%
**Why not higher yet:** we have a **strong internal core** (config, helpers, DTO surface, facade, demo, unit tests), but the SDK does **not yet actually talk to a node** — all plane methods and the transport call path are still `todo!()`, and the integration / fuzz / interop tests are still stubs.

These notes are meant to be dropped into `NOTES.MD` and used to reboot work in a fresh instance.

---

## 0) TL;DR (where we are)

**What works right now**

* `cargo build -p ron-app-sdk` — ✅
* `cargo test -p ron-app-sdk --lib` — ✅

  * 16/16 unit tests passing across `cache`, `idempotency`, `retry`, `tracing`, `ready`, `metrics`, `lru`.
* `cargo run -p ron-app-sdk --example demo` — ✅ with `RON_SDK_*` env vars set:

  * `ReadyReport { config_ok: true, transport_ok: true, tor_ok: None, missing: [] }`
  * `SDK context: profile=Micronode, amnesia=false`.

**Major pieces already implemented**

* **Config stack:** `SdkConfig` + `validate()` + `from_env()` mapping `RON_SDK_*` vars into:

  * `Transport` (TLS/Tor), gateway addr, overall timeout, connect/read/write timeouts,
  * retry policy (base/factor/cap/max_attempts/jitter),
  * idempotency config,
  * cache config,
  * tracing config,
  * PQ mode, Tor SOCKS addr.
* **Readiness:** `check_ready(&SdkConfig) -> ReadyReport` with unit tests for:

  * TLS baseline ready,
  * Tor + empty SOCKS → **not ready** (config_ok = false, missing includes `"config"`, `"tor_socks5_addr"`),
  * Tor + valid SOCKS → ready.
* **Core helpers:**

  * `retry.rs`: `base_delay`, `apply_jitter`, `backoff_schedule` (+ test for monotonic capped schedule).
  * `idempotency.rs`: `IdempotencyKey` + `derive_idempotency_key()` + deterministic fingerprinting / encoding (tests for stability and distinguishing logical keys).
  * `tracing.rs`: `SpanFields`, `build_span_fields()`, URL redaction helpers (`/path` vs query stripping) + tests.
  * `metrics.rs`: `SdkMetrics` trait + `NoopSdkMetrics` impl (Send + Sync + 'static).
  * `cache/`: `Lru<K,V>` + `TtlCache<K,V>` on top; tests for TTL expiration, enabled flag, and LRU capacity behavior.
* **DTO surface:** `types.rs` re-exports and aliases:

  * `Capability` (`CapTokenHdr`), `AddrB3` (`ContentId`), `IndexKey` (`NameRef`), `Mail`/`MailInbox`/`Receipt`/`Ack` from `ron-proto`,
  * local `ByteRange { start, end }`.
* **Error taxonomy:** `SdkError` + `RetryClass`:

  * Variants for deadline/transport/TLS/Tor/OAP violation/cap-expired/cap-denied/schema-violation/not-found/conflict/rate-limited/server/unknown.
  * `retry_class()` + `is_retriable()` classify errors into `Retriable` vs `NoRetry`.
* **Context:** `SdkContext { profile: NodeProfile, amnesia: bool }`:

  * Exposed via `RonAppSdk::context()`.
  * Currently we assume `profile=Micronode`, `amnesia=false` until we add a handshake.
* **Facade:** `RonAppSdk` struct with:

  * `new(SdkConfig) -> Result<Self, SdkError>` (validates config, constructs `TransportHandle` stub, builds `SdkContext`).
  * accessors: `context()`, `metrics()`, `metrics_mut()`, `set_metrics(M: SdkMetrics + 'static)`.
  * plane methods matching IDB/API.md:

    * `mailbox_send/recv/ack`
    * `edge_get`
    * `storage_get`/`storage_put`
    * `index_resolve`
  * Right now these delegate to plane modules where bodies are still `todo!()`.

**Key limitation**

* **No real network I/O yet.**

  * `transport::TransportHandle::call_oap(...)` is still a stub.
  * All plane methods are wired at the type level but their implementations are incomplete.
  * Integration tests (`tests/i_*`) and fuzz targets are stub-only.

---

## 1) What we shipped in this instance (accomplishments)

### 1.1 Config, env, and validation

* `SdkConfig` with fields covering:

  * `transport: Transport` (enum: `Tls` | `Tor`),
  * `gateway_addr: String`,
  * `overall_timeout: Duration`,
  * `timeouts: Timeouts { connect, read, write }`,
  * `retry: RetryCfg { base, factor, cap, max_attempts, jitter }`,
  * `idempotency: IdemCfg { enabled, key_prefix }`,
  * `cache: CacheCfg { enabled, max_entries, ttl }`,
  * `tracing: TracingCfg { spans, metrics, redaction }`,
  * `pq_mode: PqMode` (currently `Off` or `Hybrid`),
  * `tor: TorCfg { socks5_addr }`.
* `validate()` enforces invariants from CONFIG.md / IDB:

  * non-empty `gateway_addr`,
  * `retry.max_attempts >= 1`,
  * `retry.factor >= 1.0`,
  * `retry.base <= retry.cap`,
  * `overall_timeout >= connect/read/write`,
  * when `cache.enabled == true`: `max_entries >= 1`, `ttl >= 1s`,
  * when `transport == Tor`: `tor.socks5_addr` must be non-empty.
  * **Fail-closed**: returns `Err(anyhow!(...))` on violation.
* `from_env()` reads `std::env::vars()` (collected into `HashMap`) mapping `RON_SDK_*` env vars into config:

  * `RON_SDK_TRANSPORT`, `RON_SDK_GATEWAY_ADDR`,
  * `RON_SDK_*_TIMEOUT_MS`, `RON_SDK_RETRY_*`, `RON_SDK_IDEM_*`, `RON_SDK_CACHE_*`,
  * `RON_SDK_PQ_MODE`, `RON_SDK_TOR_SOCKS5_ADDR`.
* Helper `parse_ms(key, value)` deals with ms → `Duration` conversion and errors.

**Note:** README + demo comments currently show `RON_APP_SDK_*` examples; behavior actually uses `RON_SDK_*`. This is a known drift (see §2.4).

---

### 1.2 Readiness surface

* `ReadyReport` struct:

  * `config_ok: bool` (result of `SdkConfig::validate()`),
  * `transport_ok: bool` (coarse check of internal transport invariants),
  * `tor_ok: Option<bool>` (only set in Tor mode),
  * `missing: Vec<&'static str>` (coarse labels like `"config"`, `"tor_socks5_addr"`).
* `ReadyReport::is_ready()` default heuristic:

  * `config_ok && transport_ok && tor_ok.unwrap_or(true)`.
* `check_ready(&SdkConfig)`:

  * calls `.validate()` and sets `config_ok`, adds `"config"` to `missing` if it fails,
  * in Tor mode, checks `tor.socks5_addr` to set `transport_ok` and `tor_ok` flags and push `"tor_socks5_addr"` into `missing` on failure.

**Unit tests** (`ready::tests`):

* `tls_baseline_is_ready` — baseline TLS config passes with no missing items.
* `tor_without_socks_addr_is_not_ready` — Tor + empty SOCKS ⇒ not ready; `missing` includes `"config"` + `"tor_socks5_addr"`.
* `tor_with_socks_addr_becomes_ready` — Tor + valid SOCKS ⇒ ready.

---

### 1.3 Helpers: retry, idempotency, tracing, caching

**Retry (`retry.rs`):**

* `base_delay(&RetryCfg, attempt)` → exponential backoff in ms.
* `apply_jitter(base, Jitter)` → jitter variants (Full/None).
* `backoff_schedule(&RetryCfg) -> impl Iterator<Item = Duration>`:

  * yields monotonic, capped schedule.
* Test:

  * `schedule_is_monotonic_and_capped` — ensures we never exceed cap and attempts are non-decreasing.

**Idempotency (`idempotency.rs`):**

* `IdempotencyKey(String)` newtype with a fixed hex length.
* `derive_idempotency_key(cfg: &IdemCfg, logical_key: &str) -> Option<IdempotencyKey>`:

  * returns `None` when idempotency disabled.
  * when enabled, uses a deterministic fingerprint (`stable_fingerprint()` with `AHash`/BLAKE3-like scheme) + optional prefix, encoded as lowercase hex.
* Tests:

  * same inputs → same key,
  * different logical keys → different keys,
  * disabled config → `None`.

**Tracing (`tracing.rs`):**

* `SpanFields<'a>` struct containing:

  * http_method, endpoint (redacted path), capability label (redacted/bucketed), deadline_ms, retry attempt, etc.
* `build_span_fields(...)` builds `SpanFields` using config + capability + endpoint, applying redaction policy:

  * `Redaction::Safe` will:

    * strip query component,
    * collapse multiple slashes,
    * hide sensitive paths (designed to be log-safe).
* Internal helper `redact_endpoint(raw) -> Cow<'_, str>` with tests ensuring:

  * queries stripped,
  * double slashes collapsed,
  * `Redaction::None` keeps raw endpoint.

**Cache (`cache/mod.rs` + `cache/lru.rs`):**

* `Lru<K,V>`:

  * classic map + doubly-linked list LRU implementation,
  * `new(capacity)`, `get`, `insert`, `remove`, tests for capacity + order and remove semantics.
* `TtlCache<K,V>`:

  * wraps `Lru` with per-entry `Instant` and TTL from `CacheCfg`,
  * obeys `enabled` flag, TTL, max entries.
* Tests:

  * `respects_enabled_flag` — disabled cache is a no-op.
  * `evicts_on_ttl` — entries after TTL are evicted.
  * `respects_capacity_lru_behavior` — ensures LRU policy on eviction.

---

### 1.4 DTO surface & error taxonomy

**types.rs:**

* Re-exports from `ron-proto`:

  * `CapTokenHdr` → `Capability`,
  * `ContentId` → `AddrB3`,
  * `NameRef` → `IndexKey`,
  * `Send`/`Recv`/`Ack` → `Mail`/`MailInbox`/`Receipt`/`Ack`,
  * `ManifestV1` is re-exported (currently unused internally but intended for SDK consumers).
* Defines local `ByteRange { start, end }` with `len()` helper.

**errors.rs:**

* `SdkError` enum (`#[non_exhaustive]`):

  * `DeadlineExceeded`, `Transport(io::ErrorKind)`, `Tls`, `TorUnavailable`,
  * `OapViolation { reason: &'static str }`,
  * `CapabilityExpired`, `CapabilityDenied`,
  * `SchemaViolation { path, detail }`,
  * `NotFound`, `Conflict`,
  * `RateLimited { retry_after: Option<Duration> }`,
  * `Server(u16)`,
  * `Unknown(String)`.
* Helpers:

  * `schema_violation(path, detail)`,
  * `rate_limited(retry_after)`,
  * `from_io(std::io::Error)`.
* `RetryClass` + `retry_class()` / `is_retriable()` encoding policy:

  * retriable: `DeadlineExceeded`, `Transport`, `RateLimited`, 5xx `Server`.
  * non-retriable: caps, schema, 4xx, unknown, etc.

---

### 1.5 Facade (`RonAppSdk`) & example

**Facade:**

* `RonAppSdk { transport: TransportHandle, ctx: SdkContext, metrics: Box<dyn SdkMetrics> }`.

* `new(cfg)`:

  * calls `cfg.validate()`, maps `anyhow::Error` into `SdkError::SchemaViolation("config", ...)`,
  * builds `TransportHandle::new(cfg)` (stub),
  * sets `ctx = SdkContext::new(NodeProfile::Micronode, false)`.

* Exposed methods:

  * `context() -> SdkContext`,
  * `metrics() -> &dyn SdkMetrics`,
  * `metrics_mut() -> &mut dyn SdkMetrics`,
  * `set_metrics(M: SdkMetrics + 'static)`.

* Plane methods (signatures wired, bodies still `todo!()` downstream):

  * `mailbox_send`, `mailbox_recv`, `mailbox_ack`,
  * `edge_get`,
  * `storage_get` / `storage_put`,
  * `index_resolve`.

**Example (`examples/demo.rs`):**

* Loads config via `SdkConfig::from_env()`.
* Prints `ReadyReport` using `check_ready()`.
* Exits with error if not ready (using `missing` labels).
* Constructs `RonAppSdk` and prints context.
* Currently **does not call any plane methods** — by design, because transport/planes are still stubbed.

---

## 2) What’s left to reach **ron-app-sdk beta**

High level, beta needs:

* real network path (OAP/HTTP via `ron-transport`),
* plane implementations with retries + deadlines + caps wired,
* stable error mapping into `SdkError`,
* integration + fuzz + property tests that actually hit a mock gateway,
* API/semver snapshot + CI guardrails.

### 2.1 Transport: `TransportHandle` + `call_oap`

**Current status:**

* `transport/mod.rs` defines a stub-like `TransportHandle` with an `OAP_MAX_FRAME_BYTES` constant but no real I/O.

**Beta tasks:**

1. **Integrate with `ron-transport`**:

   * Mirror the accepted pattern from other crates (svc-overlay, ron-transport demo bins).
   * Provide:

     * `TransportHandle::new(SdkConfig) -> Self`,
     * `async fn call_oap(&self, req: OapRequest, deadline: Duration) -> Result<OapResponse, SdkError>`.
   * Respect:

     * `cfg.transport` (TLS vs Tor),
     * per-call deadlines (`overall_timeout` and passed-in deadline),
     * `OAP_MAX_FRAME_BYTES` (fail early if we’d exceed it).

2. **Error mapping**:

   * Map `ron-transport` errors into `SdkError`:

     * DNS/TCP/TLS → `Transport`/`Tls`,
     * OAP protocol issues → `OapViolation`,
     * HTTP status (if tunneled) → `NotFound`/`Conflict`/`RateLimited`/`Server` with proper status code,
     * timeouts → `DeadlineExceeded`.

3. **Retry wiring**:

   * Wrap `call_oap` with retry logic using:

     * `backoff_schedule(&cfg.retry)` and `SdkError::retry_class()`,
     * idempotency keys where applicable (so retried PUTs/POSTs don’t cause duplicates),
     * a hard cap on overall deadline (never exceed the request’s `deadline`).

4. **Tracing & metrics hooks**:

   * Use `SpanFields` for per-call spans, but keep it lightweight (no heavy tracing dependency inside the SDK itself beyond what’s already in workspace).
   * Increment `SdkMetrics` counters (e.g., calls, failures, retries) once we finalize the metrics trait.

### 2.2 Plane implementations: storage, edge, mailbox, index

Each plane currently has function signatures but `todo!()` bodies. For beta, we need:

**Storage plane (`planes/storage.rs`):**

* Implement:

  * `storage_get(&TransportHandle, &dyn SdkMetrics, Capability, &AddrB3, deadline) -> Result<Bytes, SdkError>`.
  * `storage_put(&TransportHandle, &dyn SdkMetrics, Capability, Bytes, deadline, Option<IdemKey>) -> Result<AddrB3, SdkError>`.
* Enforce:

  * capabilities: every call carries `Capability` (macaroon header, no “anon” path),
  * OAP frame size caps (`OAP_MAX_FRAME_BYTES`),
  * streaming semantics (64 KiB-ish chunks) if the transport exposes streaming,
  * idempotency semantics for PUT (same logical blob + idem key → same `AddrB3`, no double writes).

**Edge plane (`planes/edge.rs`):**

* Implement:

  * `edge_get(&TransportHandle, &dyn SdkMetrics, Capability, &str, Option<ByteRange>, deadline) -> Result<Bytes, SdkError>`.
* Enforce:

  * `ByteRange` → `Range` header mapping,
  * DTO strictness (headers/metadata parsed in a well-defined way),
  * capability requirements (no anonymous GETs).

**Mailbox plane (`planes/mailbox.rs`):**

* Implement:

  * `mailbox_send`,
  * `mailbox_recv`,
  * `mailbox_ack`.
* Respect:

  * capability scopes for send vs recv vs ack,
  * idempotency keys on send (so retries don’t double-send),
  * proper mapping of ack semantics (`Ack`/`Receipt` type).

**Index plane (`planes/index.rs`):**

* Implement:

  * `index_resolve(&TransportHandle, &dyn SdkMetrics, Capability, &IndexKey, deadline) -> Result<AddrB3, SdkError>`.
* Enforce:

  * strictly content-addressed results,
  * capabilities (no resolution outside allowed namespace).

**Cross-cutting for all planes:**

* Per-call deadlines provided by the caller (RonAppSdk methods already take `deadline`).
* Use retry helpers + `SdkError::retry_class()` to respect transient vs permanent failure.
* Ensure no `panic!`s; all enforcement should surface as `SdkError`.

### 2.3 Tests: integration (`tests/i_*`), property, fuzz, benches

Right now, everything under `crates/ron-app-sdk/tests/` is a stub. For beta we need:

**Integration tests (from TESTS.md & README):**

* `i_1_profile_parity.rs`:

  * Ensure Micronode vs Macronode profiles do **not** change semantics at the SDK layer (no branching on profile).
* `i_2_caps_required.rs`:

  * Show that calls without capabilities fail as `SdkError::CapabilityDenied` or similar; non-retriable.
* `i_3_oap_bounds.rs`:

  * Verify >1 MiB payloads are rejected at the SDK boundary with appropriate `OapViolation` (no oversized frames).
* `i_4_content_addressing.rs`:

  * For storage: same content → same `AddrB3`, different content → different id.
* `i_5_retries_deadlines.rs`:

  * Mock transient 5xx/timeouts and ensure retries respect cap and deadlines.
* `i_6_dto_strictness.rs`:

  * Deserialization with `deny_unknown_fields`, valid DTOs round-trip, unknown fields rejected.
* `i_7_transport_agnostic.rs`:

  * Ensure TLS vs Tor config only affects transport, not API semantics or error shapes.
* `i_8_deadlines_everywhere.rs`:

  * Calls without a deadline use config-derived default; calls with explicit deadline do not exceed it.
* `i_9_error_taxonomy.rs`:

  * Map wire errors into `SdkError` variants deterministically (no string matching).
* `i_10_semver_snapshot.rs`:

  * Capture API surface snapshot (e.g., via `cargo public-api` or custom script) and assert it matches expected baseline for the current version.
* `i_11_no_persistence.rs`:

  * Prove SDK does not create directories, files, or long-lived persistent state; caches are in-memory only.
* `i_12_canon_deps.rs`:

  * Assert dependency graph matches policy (no off-canon crates, correct versions, no `native-tls`, etc.).

**Property-based tests (`prop_retry_bounds.rs`, `prop_oap_frames.rs`, `interop_vectors.rs`):**

* `prop_retry_bounds.rs`:

  * property tests over `RetryCfg` → `backoff_schedule`, ensuring monotone, capped, non-negative durations.
* `prop_oap_frames.rs`:

  * property tests that frame splitting/joining respects max size and payload integrity once OAP framing is wired.
* `interop_vectors.rs`:

  * Use `tests/vectors/*.json`/`*.bin` to validate SDK behavior against fixed interop vectors (e.g., min OAP request, capability envelopes, etc.).

**Fuzz (`fuzz/fuzz_targets/*.rs`):**

* `oap_frame_parser.rs`:

  * fuzz OAP frame parsing at the SDK boundary (with defined size caps).
* `dto_roundtrip.rs`:

  * fuzz DTO encoders/decoders to ensure `decode(encode(x)) == x` and no panics for valid inputs.

**Benches (`benches/sdk_benches.rs`):**

* Benchmark typical SDK call patterns using Criterion:

  * idempotency key derivation,
  * backoff schedule generation,
  * (later) end-to-end mocked storage PUT/GET.
* Record baseline using `cargo bench -p ron-app-sdk -- --save-baseline sdk-YYYYMMDD`.

### 2.4 Config/env UX alignment

There is a small drift between docs and implementation:

* **Docs/demo comments** suggest `RON_APP_SDK_*` with human-readable durations (`30s`).
* **Implementation** currently uses `RON_SDK_*_MS` with ms integers.

For beta, we should:

* Support **both** families:

  * `RON_APP_SDK_*` with `humantime` parsing (e.g., `30s`, `500ms`),
  * `RON_SDK_*_MS` as a legacy / low-level override in ms.
* Update `SdkConfig::from_env` to:

  * prefer `RON_APP_SDK_*` if present, else fall back to `RON_SDK_*_MS`,
  * ensure `validate()` still enforces the same invariants.
* Update `docs/CONFIG.md` + README examples to reflect the final behavior.

### 2.5 Observability, docs, CI

**Docs:**

* Ensure `docs/API.md` exactly matches the implemented public surface:

  * `RonAppSdk` methods,
  * `SdkError` variants,
  * `types.rs` aliases.
* Confirm `docs/IDB.md` invariants are each linked to concrete tests (unit/property/integration).
* Update `docs/RUNBOOK.md` for:

  * how to configure env vars,
  * smoke-checking `check_ready`,
  * verifying connectivity via the demo.

**Observability:**

* Finalize `SdkMetrics` trait:

  * counters for calls, retries, failures per plane,
  * histograms for latency where appropriate.
* Ensure we have a “no-op by default” stance, with easy plugging into host metrics infrastructure (Prometheus, OTEL, etc.) via the trait.

**CI workflows (.github/workflows):**

* `ci.yml`:

  * `cargo fmt`, `cargo clippy` (no warnings), `cargo test -p ron-app-sdk`,
  * run property tests (if not too heavy) and fuzz smoke jobs.
* `perf-guardrails.yml`:

  * run minimal benches and check P95 doesn’t regress beyond a threshold.
* `api-semver.yml`:

  * run public API diff against `docs/api-history/ron-app-sdk/v0.1.0.txt`.

---

## 3) Suggested next steps (for the next instance)

To keep compile errors small and progress measurable, I’d resume in this order:

1. **Transport path first:**

   * Implement `TransportHandle::new` + `call_oap` with real `ron-transport` hookup, error mapping, and retry wrapper.
   * Keep planes still `todo!()` for the first pass; just get a “ping” through.

2. **Then one plane end-to-end (storage):**

   * Implement `storage_get` + `storage_put` using the new `call_oap`.
   * Add a minimal mock gateway harness just for storage and flesh out:

     * `i_3_oap_bounds.rs`,
     * `i_4_content_addressing.rs`,
     * `i_5_retries_deadlines.rs` (storage flavor).

3. **Align config/env and docs:**

   * Add `RON_APP_SDK_*` aliases and update docs/README/demo to match.

4. **Expand to other planes (edge, mailbox, index) and tests:**

   * Implement plane logic using the same patterns,
   * Fill in remaining `i_*` tests and property/fuzz targets.

Once 1–4 are done and CI is green (fmt, clippy, tests, basic fuzz, quick benches, API snapshot), we can honestly declare **ron-app-sdk beta** and then move remaining roadmap items (TS/Py/Swift parity tests, PQ-hybrid rollout, perf flamegraphs) into “post-beta (Silver/Gold)” work.



### END NOTE - NOVEMBER 14 2025 - 18:29 CST





### BEGIN NOTE - NOVEMBER 14 2025 - 20:24 CST

---

# CARRY-OVER NOTES — `ron-app-sdk` (Beta Push, planes wired)

**Beta completion estimate:** **~55%** (previous: ~40%).
**Why the bump:** we still don’t talk to a real node yet, but:

* `RonAppSdk` facade methods now delegate into plane helpers.
* Storage/edge/mailbox planes have real validation and error paths instead of `todo!()`.
* Index plane is at least typed and hooked up.
* Unit test count jumped (24 tests green) and now includes plane-level invariants (OAP cap, byte ranges, mailbox deadlines, etc.).

We’re still missing the “real” bits: **transport I/O with ron-transport, error mapping, retries, and integration / fuzz tests**. But internally, the SDK surface is much more solid.

---

## 0) TL;DR (where we are now)

**What currently works:**

* `cargo build -p ron-app-sdk` — ✅
* `cargo test -p ron-app-sdk --lib` — ✅

  * 24/24 tests green, covering:

    * cache (LRU + TTL behavior, enabled flag, eviction),
    * idempotency (stable keys, logical changes change fingerprint),
    * retry schedule (monotonic and capped),
    * tracing redaction (query-string stripping, safe path collapsing),
    * readiness (TLS / Tor config combos),
    * **planes invariants** (edge byte ranges, storage OAP caps, mailbox deadline checks).
* `cargo run -p ron-app-sdk --example demo` with env:

  ```bash
  RON_SDK_GATEWAY_ADDR="https://example.invalid"
  RON_SDK_TRANSPORT="tls"
  RON_SDK_OVERALL_TIMEOUT_MS="30000"
  cargo run -p ron-app-sdk --example demo
  ```

  Prints:

  * `ReadyReport { config_ok: true, transport_ok: true, tor_ok: None, missing: [] }`
  * `SDK context: profile=Micronode, amnesia=false`

**High-level status:**

* Config + Ready + DTOs + cache/idempotency/retry/tracing: **solid**.
* Plane modules (storage/edge/mailbox/index): **have real logic and guardrails**, but still use **stub transport** — no real gateway/overlay calls yet.
* Integration tests, fuzz/property tests, and interop vectors: still scaffolds.

---

## 1) What we shipped *in this instance*

### 1.1 Facade (`lib.rs`) surface

* `RonAppSdk` now exposes async methods that line up with the IDB / README:

  * Storage:

    * `storage_get(cap: Capability, addr: &AddrB3, deadline: Duration) -> Result<Bytes, SdkError>`
    * `storage_put(cap: Capability, blob: Bytes, deadline: Duration, idem: Option<IdemKey>) -> Result<AddrB3, SdkError>`

  * Edge:

    * `edge_get(cap: Capability, path: &str, range: Option<ByteRange>, deadline: Duration) -> Result<Bytes, SdkError>`

  * Mailbox:

    * `mailbox_send(cap: Capability, mail: Mail, deadline: Duration, idem: Option<IdemKey>) -> Result<Receipt, SdkError>`
    * `mailbox_recv(cap: Capability, inbox: MailInbox, deadline: Duration) -> Result<Vec<MailInbox>, SdkError>`
    * `mailbox_ack(cap: Capability, ack: Ack, deadline: Duration) -> Result<(), SdkError>`

  * Index:

    * `index_resolve(cap: Capability, key: &IndexKey, deadline: Duration) -> Result<AddrB3, SdkError>`

* All these methods:

  * Take a **capability** (no anonymous calls).
  * Take a **deadline** at the SDK surface, ready to be enforced in transport.
  * Are wired to `planes::*` helpers and the internal `TransportHandle` + `SdkMetrics` trait.

This means consumers can already code against the intended async API even though the underlying transport is still stubbed.

---

### 1.2 Plane modules (storage / edge / mailbox / index)

#### Storage plane

* Implemented `storage_put` with **OAP frame cap enforcement**:

  * Rejects blobs larger than `OAP_MAX_FRAME_BYTES` with a **non-panic** `SdkError` (something like `SdkError::OapViolation` / `TooLarge`).
  * New unit test: `storage_put_rejects_payload_larger_than_oap_cap` exercises this boundary.

* Implemented `parse_addr_b3` helper:

  * Sanitizes/validates BLAKE3 content IDs parsed from strings.
  * New unit test: `parse_addr_b3_rejects_garbage` verifies invalid input fails cleanly.

* `storage_get` / `storage_put` are now thin shims around:

  * capability + deadline enforcement,
  * (future) call to `TransportHandle::call_oap`,
  * mapping responses into `Bytes` and `AddrB3`.

At the moment they still rely on stub transport but the call shape is correct.

---

#### Edge plane

* Implemented **ByteRange → HTTP Range** mapping and validation:

  * `ByteRange { start, end }` now has helpers/tests that:

    * Build `Range` header strings like `"bytes=START-END"` (inclusive).
    * Reject inverted ranges (start > end).

* Tests added:

  * `byte_range_header_format_is_inclusive` — ensures `ByteRange { start: 0, end: 9 }` maps to `"bytes=0-9"`, not off by one.
  * `byte_range_validation_accepts_well_formed_range` — valid ranges pass.
  * `byte_range_validation_rejects_inverted_range` — invalid ranges surface as errors, not panics.

* `edge_get` signature is aligned with README:

  * Takes `Capability`, `&str` path, optional `ByteRange`, and a deadline.
  * Prepared to convert to the correct OAP/HTTP call once transport is wired.

---

#### Mailbox plane

* Implemented mailbox helpers with **deadline guardrails**:

  * `mailbox_send`
  * `mailbox_recv`
  * `mailbox_ack`

* All three enforce **non-zero deadlines** and return a structured `SdkError` if you pass `Duration::from_millis(0)` (or equivalent). This keeps us from accidentally making “fire-and-forget” calls with no timeout.

* New tests:

  * `mailbox_send_rejects_zero_deadline`
  * `mailbox_recv_rejects_zero_deadline`
  * `mailbox_ack_rejects_zero_deadline`

These are early invariants; actual OAP payload construction + response mapping will be filled once transport is in.

---

#### Index plane

* `index_resolve` is fully typed and hooked into the SDK facade:

  * Accepts `Capability`, `&IndexKey`, and a deadline.
  * Returns `Result<AddrB3, SdkError>`.

* Implementation is still minimal (no real transport call yet), but the method is in place, ready to be backed by `ron-index`/`svc-index` via gateway once the OAP path is implemented.

---

### 1.3 Tests & readiness

* Lib test suite expanded from 16 → **24 tests** — all green.

* Existing modules remain well covered:

  * **cache**: LRU capacity, TTL eviction, enabled/disabled behavior.
  * **idempotency**: deterministic `IdemKey` generation, case/whitespace normalization, logical key changes altering fingerprint.
  * **retry**: `backoff_schedule` is monotonic and capped; jitter logic is stable.
  * **tracing**: redaction behavior (stripping queries, collapsing slashes, `Redaction::None` vs `Redaction::Safe`).
  * **ready**: config combos for TLS/Tor, missing SOCKS address, etc.
  * **metrics**: `NoopSdkMetrics` satisfies `SdkMetrics` + `Send + Sync + 'static`.

* New plane tests (storage/edge/mailbox) lock in invariants that we will rely on when wiring the real transport.

* The demo example now consistently loads config from `RON_SDK_*` env vars and passes readiness checks for TLS.

---

## 2) What’s still left for **ron-app-sdk beta**

This hasn’t changed conceptually from the earlier note, but the remaining work is now **tighter** and more concrete.

### 2.1 Transport: `TransportHandle` + `call_oap`

**Current state (after this instance):**

* `TransportHandle` exists with `OAP_MAX_FRAME_BYTES` and a stub body; facade + planes *assume* a `call_oap`-like API but real I/O isn’t wired.
* We still don’t depend on `ron-transport` here; all network behavior is theoretical.

**For beta we still need:**

1. **Real client transport implementation**:

   * Either:

     * (Phase 1) **direct TLS client** via `reqwest` or `hyper` (boring HTTPS client) behind `TransportHandle`, or
     * (Phase 2) **integrate `ron-transport` client** once that API is finalized.

   * Provide:

     ```rust
     impl TransportHandle {
         pub fn new(cfg: &SdkConfig) -> Self { ... }
         pub async fn call_oap(&self, /* OAP request */) -> Result</* OAP response */, SdkError> { ... }
     }
     ```

   * Respect:

     * `cfg.transport` (`Transport::Tls` vs `Transport::Tor`).
     * `cfg.timeouts` — connect/read/write, plus per-call `deadline`.
     * `OAP_MAX_FRAME_BYTES` — early reject too-large payloads.

2. **Error mapping**:

   * Map underlying failures into `SdkError`:

     * DNS/TCP/TLS failures → `SdkError::Transport` / `SdkError::Tls`.
     * OAP protocol violations (bad framing, size bounds) → `SdkError::OapViolation`.
     * HTTP status codes (if we use HTTP tunnel) → `SdkError::NotFound` / `Conflict` / `RateLimited` / `Server` etc.
     * Timeouts → `SdkError::DeadlineExceeded`.

3. **Retry wiring**:

   * Wrap `call_oap` with retry logic using:

     * `backoff_schedule(&cfg.retry)`,
     * `SdkError::retry_class()` to decide which errors are safe to retry,
     * per-call deadline: never exceed the supplied `deadline` no matter how many retries happen.

4. **Tracing / metrics hooks**:

   * Use `SpanFields` to construct per-call spans (minimal, no heavy dependencies in this crate).
   * Invoke `SdkMetrics` hooks for:

     * total calls,
     * failures,
     * retries,
     * latency buckets (once we decide on metric names).

---

### 2.2 Planes: complete the OAP round-trips

All planes now have **shape + invariants**, but they still need to actually talk to the node.

* **Storage**:

  * Use `TransportHandle::call_oap` to:

    * `PUT`/`POST` blobs to storage (honoring 1 MiB frame and stream chunking).
    * `GET` by `AddrB3`.
  * Attach capability on every call.
  * Wire **idempotency**:

    * if `IdemCfg.enabled`, derive/generate an idempotency key and pass it.
    * Ensure repeated `storage_put` with same idem key returns same `AddrB3`.

* **Edge**:

  * Implement `edge_get` against gateway’s edge surface:

    * convert `ByteRange` to `Range: bytes=...` header.
    * enforce DTO strictness on responses (no unknown fields).
    * map 404/403/5xx into the right `SdkError` variants.

* **Mailbox**:

  * Implement real:

    * `mailbox_send` (cap + idem + payload),
    * `mailbox_recv` (paging / cursor semantics when we finalize them),
    * `mailbox_ack` (ack semantics).
  * Preserve the existing deadline guardrails and capability requirements.

* **Index**:

  * Implement `index_resolve` using the canonical index surface via gateway:

    * `IndexKey` → `AddrB3` lookup.
    * enforce content-addressed results; capability scopes; nice error mapping (e.g., “not found in namespace” vs “network busted”).

---

### 2.3 Tests: integration, interop vectors, property, fuzz

Still to do:

* **Integration tests** (`crates/ron-app-sdk/tests/i_*`):

  * Upgrade the current scaffolds into **real tests** against:

    * a tiny in-process mock gateway (Axum) or
    * a real Micronode/Macronode test harness later.
  * Specifically:

    * `i_1_profile_parity` — same behavior for Micronode vs Macronode configs.
    * `i_2_caps_required` — no mutating call without a capability.
    * `i_3_oap_bounds` — 1 MiB cap enforced; ~64 KiB chunking honored.
    * `i_4_content_addressing` — storage uses `AddrB3` correctly.
    * `i_5_retries_deadlines` — retries stop at deadlines; no unbounded backoff.
    * `i_6_dto_strictness` — bogus responses are rejected, not accepted.
    * `i_7_transport_agnostic` — TLS vs Tor yields identical semantics.
    * `i_8_deadlines_everywhere` — every public method respects deadlines.
    * `i_9_error_taxonomy` — SdkError variants are exercised end-to-end.
    * `i_10+` — semver snapshot, no persistence, canon deps, etc.

* **Interop vectors**:

  * Fill in `tests/vectors/*.json` with real capabilities/OAP examples mirrored from gateway / node services.
  * Add tests that deserialize/round-trip those vectors through ron-app-sdk.

* **Property/fuzz tests**:

  * `prop_oap_frames` — no malformed frames produced; obey frame caps.
  * `prop_retry_bounds` — retries never exceed config bounds and deadlines.
  * Fuzz harness for OAP frame parsing and SDK’s request builder.

* **Perf benches**:

  * Upgrade `benches/sdk_benches.rs` from `println!` to Criterion benches:

    * end-to-end `storage_put`/`storage_get` (with a mock transport),
    * `edge_get` under various payload sizes,
    * mailbox send/recv/ack microbenchmarks.

---

## 3) Suggested next moves for the *next* instance

If we want to keep the “small increments, no explosion of errors” pattern:

1. **Step 1 – Transport skeleton w/ reqwest (no ron-transport yet):**

   * Implement `TransportHandle::new` + `call_oap` backed by `reqwest` over TLS.
   * Support only `Transport::Tls` initially; map `Transport::Tor` to a “not yet implemented” `SdkError` so the rest of the SDK doesn’t panic.
   * Keep OAP request/response shape minimal (e.g., simple JSON body or `/ping`-style stub).

2. **Step 2 – Wire storage plane to that transport:**

   * Implement a minimal `storage_put`/`storage_get` roundtrip against a tiny Axum server in an integration test.
   * Validate OAP cap (payload rejected at 1 MiB+).

3. **Step 3 – Error taxonomy + retries for that storage path:**

   * Use actual HTTP status codes in the mock to drive `SdkError::retry_class()`.
   * Validate `backoff_schedule` + deadlines in `i_5_retries_deadlines`.

Once those three are done, we’ll be **very close** to a legit beta: SDK will actually talk to something, invariants will be test-enforced, and the facade will be largely stable.

---

## 4) Command cheat sheet (current)

Just to avoid hunting:

```bash
# Build
cargo build -p ron-app-sdk

# Lib/unit tests
cargo test -p ron-app-sdk --lib

# Demo (TLS)
RON_SDK_GATEWAY_ADDR="https://example.invalid" \
RON_SDK_TRANSPORT="tls" \
RON_SDK_OVERALL_TIMEOUT_MS="30000" \
cargo run -p ron-app-sdk --example demo
```


### END NOTE - NOVEMBER 14 2025 - 20:24 CST




### BEGIN NOTE - NOVEMBER 15 2025 - 19:23 CST

---

# CARRY-OVER NOTES — `ron-app-sdk` (Beta Push — planes wired to transport)

**Beta completion estimate:** **~78%** (prev: ~55%).
**Why the bump:** façade is stable; transport enforces OAP/1 cap, deadlines, retries; planes (storage/edge/index/mailbox) now issue real requests via `call_oap_json`; idempotency newtype fixed; unit tests green.

---

## 0) TL;DR

* **Build + tests:** green (last run 29/29 unit tests).
* **Facade (`RonAppSdk`)** exposes async methods for storage/edge/mailbox/index; capability-first; per-call deadlines.
* **Transport** (`TransportHandle`):

  * Enforces **1 MiB OAP/1 frame cap**.
  * Honors **per-call deadline** clamped by `overall_timeout`.
  * **Retries** with exponential backoff via `backoff_schedule` + `RetryCfg.max_attempts`.
  * TLS path live via `reqwest`; Tor path returns `SdkError::TorUnavailable` (OK for beta if documented).
* **Planes wired**:

  * **Mailbox** — fixed lifetime bug; `IdemKey` passed as `&str`.
  * **Storage** — `GET/PUT` JSON round trips; addr parse validation; OAP cap enforced on PUT.
  * **Edge** — `GET` w/ optional inclusive byte range; strict validation.
  * **Index** — resolve logical key → `AddrB3`; strict schema and parse validation.
* **Idempotency** — `IdempotencyKey` has `as_str` + `AsRef<str>` and redacted `Debug`.

---

## 1) What’s landed (this instance + recent)

### 1.1 Façade

* `RonAppSdk::new(cfg)` validates config; constructs `TransportHandle`; stub context (`Micronode`, amnesia=false).
* Public async methods:

  * `storage_get(cap, addr_b3_hex, deadline) -> Bytes`
  * `storage_put(cap, blob, deadline, idem) -> AddrB3`
  * `edge_get(cap, path, range?, deadline) -> Bytes`
  * `mailbox_send(cap, mail, deadline, idem?) -> Receipt`
  * `mailbox_recv(cap, deadline) -> Vec<MailInbox>`
  * `mailbox_ack(cap, receipt, deadline) -> ()`
* Metrics sink pluggable (`SdkMetrics` trait; `NoopSdkMetrics` default).

### 1.2 Transport

* **`transport/handle.rs`**

  * `OAP_MAX_FRAME_BYTES = 1_048_576` (1 MiB).
  * `call_oap(endpoint, payload, deadline)` — size cap, TLS, clamped deadline, HTTP status→`SdkError`.
  * `call_oap_with_retry(endpoint, payload, deadline)` — outer deadline guard + backoff.
* **`transport/json.rs`**

  * `call_oap_json(endpoint, &impl Serialize, deadline) -> Result<Vec<u8>, SdkError>` (serialize → `call_oap`).
* **`transport/mapping.rs`**

  * Maps `reqwest::Error` and status codes (404/409/429/5xx) to `SdkError` (+ optional `Retry-After`).

### 1.3 Planes

* **Mailbox (`planes/mailbox.rs`)**

  * JSON DTOs; deadline guard; `idem: Option<&str>` (safe borrow).
  * Metrics hooks (`observe_latency`, `inc_failure`) with stable endpoint labels.
* **Storage (`planes/storage.rs`)**

  * `GET`: JSON `{ blob }` or raw bytes fallback; deadline guard.
  * `PUT`: enforces OAP cap locally; idem key optional; response `addr_b3` re-parsed/validated.
* **Edge (`planes/edge.rs`)**

  * Validates `ByteRange` (inclusive semantics); deadline guard; JSON `{ blob }`.
* **Index (`planes/index.rs`)**

  * Deadline guard; strict schema; returned `addr_b3` must parse; metrics `oap_call_started/observe_oap_*` (see §2.1 to align trait).

### 1.4 Idempotency (`idempotency.rs`)

* Newtype `IdempotencyKey(String)`:

  * `as_str`, `AsRef<str>`, redacted `Debug`, `Display`.
  * `derive_idempotency_key(cfg, method, endpoint, logical_key?)` → `Option<IdempotencyKey>`.
  * Header helper `into_header()` (`Idempotency-Key`).

### 1.5 Tests

* Unit tests cover:

  * Cache (LRU/TTL/enabled).
  * Idempotency (determinism, input normalization).
  * Retry schedule (monotonic + capped).
  * Tracing redaction (query stripping; slash collapse).
  * Ready (TLS/Tor combos).
  * Transport invariants (OAP cap, zero deadline, outer deadline).
  * Planes basic invariants (byte range inclusive, mailbox deadline guards, storage cap).
* Last run: **29/29** passed in ~0.02s on 2019 MBP 13".

---

## 2) What remains for **beta** (speed-run plan)

### 2.1 Metrics trait alignment (quick win)

**Goal:** consistent metrics calls across all planes.

**Current:** storage/edge/mailbox use `observe_latency + inc_failure`; index uses `oap_call_started/observe_oap_*`.
**Do one of:**

* **Option A (simplify):** remove `oap_call_started/observe_oap_*` from index; switch to `observe_latency + inc_failure` like others.
* **Option B (richer):** extend `SdkMetrics` with:

  * `fn oap_call_started(&self, ep: &str);`
  * `fn observe_oap_success(&self, ep: &str, dur: Duration);`
  * `fn observe_oap_failure(&self, ep: &str, kind: &str);`
  * Implement no-op in `NoopSdkMetrics`; update storage/edge/mailbox to use them too.

**Recommendation:** Option A for beta speed; Option B can be a post-beta polish.

### 2.2 DTO strictness (cheap, high value)

Add `#[serde(deny_unknown_fields)]` on all response DTOs:

* `EdgeGetResp`, `PutResp`, `ResolveResp`, `SendResp`, `RecvResp`
  Catches drift early and ensures strict schema.

### 2.3 Integration tests (tiny mock gateway)

Spin a minimal **Axum** mock (in `tests/` or `tests/mock_gateway/`) that exposes:

Routes:

* `POST /storage/get` → returns `{"blob": <bytes>}`
* `POST /storage/put` → returns `{"addr_b3":"b3:<hex64>"}` or 413-ish (map to `SdkError::OapViolation`) to validate cap in client (still fail fast client-side).
* `POST /edge/get` → `{"blob": <subrange>}` (honor `range`).
* `POST /mb/send` → `{"receipt":{...}}`
* `POST /mb/recv` → `{"inbox":[...]}` (empty ok)
* `POST /mb/ack` → `{}`

Scenarios:

* **Happy paths** for each plane.
* **Error taxonomy**: return 404/409/429/500 and assert `SdkError::{NotFound,Conflict,RateLimited,Server}`.
* **Retry + deadline**: return two 500s then 200; assert backoff and total elapsed < outer deadline (or flips at deadline).
* **Schema strictness**: inject an extra JSON field; expect failure due to `deny_unknown_fields`.

### 2.4 Docs + Examples

* Update `README.md` with runnable snippets for each plane (capability stub OK).
* Ensure `examples/demo.rs` uses env:

  ```
  RON_SDK_GATEWAY_ADDR="https://example.invalid"
  RON_SDK_TRANSPORT="tls"
  RON_SDK_OVERALL_TIMEOUT_MS="30000"
  cargo run -p ron-app-sdk --example demo
  ```

### 2.5 Tor posture (OK to defer, just document)

* Keep `Transport::Tor` → `SdkError::TorUnavailable` for beta.
* Document as “coming soon” with SOCKS5 config placeholder.

### 2.6 CI/Script hygiene

Add `scripts/smoke_sdk.sh`:

* fmt + clippy + tests
* (Optionally) spins the mock Axum server in background, runs the integration suite, kills it.

---

## 3) How to build / test / run (current)

### 3.1 Build

```
cargo build -p ron-app-sdk
```

### 3.2 Unit tests

```
cargo test -p ron-app-sdk --lib
```

### 3.3 Demo (current placeholder)

```
RON_SDK_GATEWAY_ADDR="https://example.invalid" \
RON_SDK_TRANSPORT="tls" \
RON_SDK_OVERALL_TIMEOUT_MS="30000" \
cargo run -p ron-app-sdk --example demo
```

### 3.4 Suggested smoke script (paste-ready)

Create `crates/ron-app-sdk/scripts/smoke_sdk.sh`:

```
#!/usr/bin/env bash
set -euo pipefail

echo "[STEP] fmt + clippy + unit tests"
cargo fmt -p ron-app-sdk
cargo clippy -p ron-app-sdk --no-deps -- -D warnings
cargo test  -p ron-app-sdk --lib

# Optional: integration with mock server when added
# echo "[STEP] start mock gateway"
# MOCK_PORT=18080
# RUST_LOG=info cargo run -p ron-app-sdk --bin mock_gateway -- --port ${MOCK_PORT} &
# GW_PID=$!
# trap "kill ${GW_PID} || true" EXIT
# sleep 0.5

# echo "[STEP] run integration tests"
# cargo test -p ron-app-sdk --test i_suite

echo "[OK] smoke passed"
```

Grant exec:

```
chmod +x crates/ron-app-sdk/scripts/smoke_sdk.sh
```

Run:

```
crates/ron-app-sdk/scripts/smoke_sdk.sh
```

---

## 4) API/DTO reference (current state)

**Transport endpoints used by planes (through gateway):**

* Storage:

  * `POST /storage/get` — body: `{ cap, addr_b3 }` → `{"blob": <bytes>}` (or raw bytes)
  * `POST /storage/put` — body: `{ cap, blob, idem? }` → `{"addr_b3":"b3:<hex64>"}`
* Edge:

  * `POST /edge/get` — body: `{ cap, path, range?{start,end} }` → `{"blob": <bytes>}`
* Mailbox:

  * `POST /mb/send` — body: `{ cap, mail, idem? }` → `{"receipt":{...}}`
  * `POST /mb/recv` — body: `{ cap }` → `{"inbox":[...]}`
  * `POST /mb/ack` — body: `{ cap, receipt }` → `{}`
* Index:

  * `POST /index/resolve` — body: `{ cap, key }` → `{"addr_b3":"b3:<hex64>"}`

**Invariants:**

* Every call carries a **Capability**.
* Per-call **deadline > 0** (validated in each plane).
* **1 MiB frame cap** enforced client-side before I/O.
* **Idempotency** is optional on PUT/send; when enabled, passed as `&str`.

---

## 5) Common pitfalls + quick triage

* **“no method observe_oap_* on SdkMetrics”**
  Align metrics (see §2.1). Either add these methods to the trait or switch index plane to `observe_latency/inc_failure`.
* **“as_deref() not found on IdemKey”**
  Use `idem.as_ref().map(|k| k.as_ref())` or `k.as_str()`; the newtype now implements `AsRef<str>`.
* **Serde schema failures**
  Add `#[serde(deny_unknown_fields)]` to responses to surface drift; adjust DTOs or mocks accordingly.
* **Tor not implemented**
  Expect `SdkError::TorUnavailable` if `Transport::Tor` is set; document this for beta.

---

## 6) Minimal acceptance criteria for **beta DONE**

* ✅ All unit tests green.
* ✅ Integration suite against mock gateway covering:

  * Happy path round-trips for all planes.
  * Error taxonomy mapping (404/409/429/5xx).
  * Retry + deadline behavior proven (time-bounded).
  * DTO strictness (`deny_unknown_fields`) enforced.
* ✅ README updated with clear usage snippets + env vars.
* ✅ Smoke script runs clean (`fmt`, `clippy`, units, integrations).
* ✅ Tor path documented as “unavailable” for beta (no panics).
* ✅ Semver-safe public API (façade signatures stable).

---

## 7) Next files to land (recommended order)

1. **Metrics trait alignment** (either simplify or extend)

   * `src/metrics.rs` (trait + `NoopSdkMetrics`)
   * touch planes to match.

2. **Integration mock**

   * `tests/mock_gateway.rs` (Axum server)
   * `tests/i_storage.rs`, `i_edge.rs`, `i_mailbox.rs`, `i_index.rs`.

3. **DTO strictness**

   * Add `#[serde(deny_unknown_fields)]` to response DTOs across planes.

4. **README/example refresh**

   * Small code blocks per plane; env setup; “known limitations”.

Once those are in, we should be ~90%+ and sprint the final polish.

---

## 8) Command cheat sheet

Build:

```
cargo build -p ron-app-sdk
```

Unit tests:

```
cargo test -p ron-app-sdk --lib
```

Run demo:

```
RON_SDK_GATEWAY_ADDR="https://example.invalid" \
RON_SDK_TRANSPORT="tls" \
RON_SDK_OVERALL_TIMEOUT_MS="30000" \
cargo run -p ron-app-sdk --example demo
```

Smoke:

```
crates/ron-app-sdk/scripts/smoke_sdk.sh
```

### END NOTE - NOVEMBER 15 2025 - 19:23 CST