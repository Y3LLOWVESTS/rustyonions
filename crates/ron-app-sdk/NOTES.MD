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