---

crate: ron-app-sdk
path: crates/ron-app-sdk
role: sdk
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A lightweight, async Rust client SDK for apps to speak **OAP/1** to a RustyOnions node/gateway—handling TLS, HELLO negotiation, capability tokens, and streaming PUT/GET of content-addressed blobs.

## 2) Primary Responsibilities

* Provide a **safe, ergonomic client** to perform OAP requests (e.g., `put`, `get`, `hello`) with connection lifecycle, retries, and backpressure.
* Implement **OAP/1 framing** via the `oap` crate, including canonical DATA header packing (`obj: "b3:<hex>"`), capabilities, and negotiated limits.
* Offer **streaming I/O** APIs (file/reader/writer) so large payloads do not load fully into memory; compute BLAKE3 on the fly for PUT.

## 3) Non-Goals

* Not a server, proxy, or transport daemon; it’s a **client library** only.
* No business policy enforcement (quotas, authz rules); it **carries** capability credentials but does not adjudicate them.
* No persistent cache by default (optional, pluggable is fine but out of core scope).

## 4) Public API Surface

*(naming may differ slightly; organized by intent rather than exact symbol names)*

* **Re-exports:**

  * From `oap`: frame constants/flags, error types used in results (so apps don’t import `oap` directly).
  * From hashing: `blake3::Hash` (or a thin newtype like `ObjId`).

* **Key types / functions / traits:**

  * `Client` / `RonClient`

    * `new(config: Config) -> Result<Self>`
    * `hello(&self) -> Result<HelloInfo>` (populates negotiated `max_frame`, flags).
    * `put(&self, reader: impl AsyncRead, len: u64) -> Result<ObjId>` (streams, hashes, optional compression).
    * `put_file<P: AsRef<Path>>(&self, path: P) -> Result<ObjId>` convenience.
    * `get(&self, id: &ObjId) -> Result<Bytes>` (bounded by policy; small objects).
    * `get_to_writer(&self, id: &ObjId, w: impl AsyncWrite) -> Result<()>` (streaming for large objects).
  * `Config` (builder): endpoint(s), TLS mode/roots, timeouts, in-flight limit, `tenant`, `app_id`, `capability` (macaroon or token), `enable_comp`.
  * `ObjId` (newtype around BLAKE3 hex).
  * `SdkError` (rich error enum with `RetryAdvice`, see §10).
  * `SdkMetrics` (optional feature) or trait-based hooks for instrumentation.

* **Events / HTTP / CLI:**

  * None directly (library), but **feature-gated** dev helpers like `sdk::bin::put` for local testing are fine.

## 5) Dependencies & Coupling

* **Internal crates**

  * `oap` (tight by design): single source of truth for framing/limits. **Replaceable:** yes in principle, but it’s the protocol—so keep tight.
  * `ron-kernel` **not** a dependency (avoid layering inversion).
  * `gateway` / `svc-*` consume the SDK indirectly (at app edges), not vice-versa.

* **External crates (top set; keep minimal)**

  * `tokio` (net + io-util) — async runtime.
  * `tokio-rustls` + `rustls-pemfile` (TLS 1.3, ALPN `ron/1`).
  * `bytes` (zero-copy buffers).
  * `blake3` (content IDs).
  * `serde`/`serde_json` (HELLO payload and optional DATA header JSON).
  * Optional: `zstd` (COMP), `webpki-roots`/`rustls-native-certs` (roots), `thiserror` (errors).
    *Risks:* keep TLS stack versions aligned with workspace pins; `zstd` behind a feature to keep the default build small.

* **Runtime services**

  * **Network:** TCP/TLS client sockets (and optionally SOCKS5 if Tor is used—feature-gated).
  * **Storage/OS/Crypto:** none persistent; uses OS to open files for convenience helpers and `blake3` for hashing.

## 6) Config & Feature Flags

* **Env vars:** none required; allow reading TLS roots or endpoint from env in helpers/tests (`RON_ENDPOINT`, `RON_TLS_INSECURE=1` for dev only).
* **Config struct:**

  * `endpoint: Endpoint` (`host:port`), `alpn: "ron/1"`, `tls: TlsMode` (`SystemRoots | WebPki | InsecureDev`), `tenant`, `app_id`, `capability: Option<Vec<u8>>`,
  * networking knobs: `connect_timeout`, `read_timeout`, `write_timeout`, `hello_timeout`, `inflight_limit`, `retry: RetryPolicy { max_retries, base_backoff, max_backoff }`,
  * `max_frame_hint` (upper bound to clamp local allocations until HELLO arrives).
* **Cargo features:**

  * `comp` (zstd COMP frames; enforce ≤8× decompressed bound),
  * `native-roots` / `webpki-roots`,
  * `tor-socks` (dial via SOCKS5),
  * `metrics` (expose counters/histograms via callbacks),
  * `tracing-logs` (lightweight spans/logs).

## 7) Observability

* **Client-side golden metrics (emit through trait/callback if `metrics` enabled):**

  * `requests_total{op,code}`; `bytes_{in,out}_total{op}`; `latency_seconds{op}`; `inflight{op}`; `retries_total{op,reason}`; `rejected_total{reason}` (oversize, auth, quota).
* **Logging:** `tracing` spans per request with correlation id (OAP corr\_id) and app/tenant tags (redact secrets).
* **Health:** `hello()` result exposed so apps can log negotiated limits and supported flags at startup.

## 8) Concurrency Model

* **Connection management:** a small **async pool** (N connections) honoring `inflight_limit` per connection (via OAP flags/ACK if used) and global **semaphore** for total in-flight.
* **Backpressure:** acquire permit → encode → send → await response or ACK; if pool exhausted, new requests **wait** (bounded queue) or fail fast based on `RetryPolicy`.
* **Timeouts & retries:** per-stage timeouts (connect/hello/read/write). Retries only for **idempotent** ops (GET, HELLO) and only on transient errors (timeout, connection lost before write finished). PUT retries require idempotence with content-hash verification (safe if server side is “put-if-absent by obj id”).
* **Locks:** avoid long-held mutexes; use `parking_lot` or `tokio::sync::Mutex` around small state; channel fan-out for response routing keyed by `corr_id`.

## 9) Persistence & Data Model

* **None by default.**
* Optional **ephemeral LRU** (feature `cache`) could be offered for recent GETs, keyed by `ObjId` with size cap; but keep out of core if not already present.

## 10) Errors & Security

* **Error taxonomy (suggested `SdkError`):**

  * `Config`, `Tls`, `Connect`, `Protocol(OapError)`, `Timeout(Stage)`, `CapRejected(Code)`, `Quota`, `Oversize{got, max}`, `DecompressTooLarge`, `NotFound`, `Io`, `Canceled`, `Other`.
  * Include `RetryAdvice` (`Yes`, `No`, `After(Duration)`), and `is_transient()` helper.
* **Security posture:**

  * TLS 1.3 with ALPN `ron/1`; no TLS compression/renegotiation; SNI as host.
  * Capability token (e.g., macaroon) carried in OAP capability bytes; **do not** log or echo it.
  * Optional end-to-end app payload encryption signaled by `APP_E2E` flag (opaque body to intermediary nodes).
  * **PQ-readiness:** SDK agnostic to cipher suite; when the transport adopts PQ TLS (or hybrid), no API change expected.

## 11) Performance Notes

* **Hot paths:** OAP frame encode/decode, TLS writes/reads, BLAKE3 hashing on PUT, (optional) zstd compression.
* **Targets (initial SLOs):**

  * Under local node conditions: `GET p95 < 20 ms`, `p99 < 60 ms` for ≤64 KiB objects; `PUT p95 < 40 ms` for ≤1 MiB with fsync-off on server side.
  * Maintain zero-copy where possible (`Bytes`), stream to avoid buffering whole payloads, and clamp decompression expansion to ≤8× negotiated `max_frame`.

## 12) Tests

* **Unit:** encode/decode round-trips for request/response frames; DATA header pack/unpack; error mapping (`SdkError` ↔ underlying).
* **Integration:** talk to a **fake OAP server** (in-process) and a **real gateway** in CI; verify HELLO negotiation, in-flight windows, and size bounds.
* **E2E:** file PUT/GET to a dev node over TLS with real certificates; optional Tor-SOCKS path.
* **Property/Fuzz:** fuzz OAP parsing via `oap` crate and the SDK’s request router; proptests for retry/backoff invariants.
* **Concurrency (loom):** minimal loom checks for channel routing and permit logic around cancel/timeout races.

## 13) Improvement Opportunities

* **Unify protocol logic:** ensure all OAP framing is delegated to the `oap` crate; eliminate any duplicate constants/types in the SDK.
* **Typed errors + retry hints:** if today returns `anyhow::Result`, replace with `SdkError` + `RetryAdvice` to make caller behavior precise.
* **Pool & backpressure knobs:** expose `inflight_limit`, `pool_size`, `queue_depth`, per-op timeouts in `Config` with sane defaults.
* **Compression & bounds:** only enable `comp` when both sides negotiate it; enforce decompressed-size clamp and surface an explicit error variant.
* **Observability hooks:** thin trait to emit metrics without pulling `prometheus` into the app’s dependency tree.
* **Test vectors:** mirror `oap`’s canonical vectors in the SDK tests to prevent drift.
* **Security hygiene:** redaction of secrets in `Debug`, optional `Zeroize` on capability buffers, and explicit `danger_insecure_tls` dev toggle.

## 14) Change Log (recent)

* 2025-09-14 — Drafted deep analysis; aligned expected SDK behavior with `oap` (HELLO, 1 MiB default `max_frame`), added retry/backpressure guidance, and defined initial SLOs.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 (core shape is clear; needs a crisp `Client` + `Config` and typed errors finalized).
* **Test coverage:** 2 (assumed basic tests; prioritize integration + round-trips + retry logic).
* **Observability:** 3 (tracing present; add metric hooks to hit golden set).
* **Config hygiene:** 4 (builder + sensible defaults; document insecure dev options).
* **Security posture:** 4 (TLS 1.3 + ALPN; capability bytes; E2E flag; PQ neutral).
* **Performance confidence:** 3 (design supports streaming; need CI perf smoke and bounds).
* **Coupling (lower is better):** 4 (depends only on `oap` and TLS stack; keep kernel/gateway out of the SDK to avoid cycles).

