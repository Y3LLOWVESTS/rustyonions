<!-- Generated: 2025-09-14 20:11:07Z -->
# Crate Summaries (Combined)

This file is generated from markdown files in `docs/crate-summaries`.

## Table of Contents
- [accounting](#accounting)
- [arti transport](#arti-transport)
- [common](#common)
- [gateway](#gateway)
- [index](#index)
- [kameo](#kameo)
- [naming](#naming)
- [node](#node)
- [oap](#oap)
- [overlay](#overlay)
- [ron-app-sdk](#ron-app-sdk)
- [ron-bus](#ron-bus)
- [ron-kernel](#ron-kernel)
- [ryker](#ryker)
- [svc-crypto](#svc-crypto)
- [svc-index](#svc-index)
- [svc-omnigate](#svc-omnigate)
- [svc-overlay](#svc-overlay)
- [svc-storage](#svc-storage)
- [tldctl](#tldctl)
- [transport](#transport)


---

# accounting

---

crate: accounting
path: crates/accounting
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Tiny, std-only byte accounting: a `CountingStream<S>` wrapper and shared `Counters` that track total bytes in/out and rolling per-minute buckets (60-slot ring).

## 2) Primary Responsibilities

* Provide a thread-safe in-memory counter for bytes read/written (totals + 60-minute ring).
* Offer a zero-alloc snapshot API for quick export to logs/metrics.
* Make it trivial to instrument any `Read`/`Write` with `CountingStream<S>`.

## 3) Non-Goals

* No Prometheus/HTTP export layer (consumers must expose).
* No quotas/rate-limiting or policy enforcement.
* No persistence or cross-process aggregation.

## 4) Public API Surface

* **Re-exports:** (none)
* **Key types / functions / traits:**

  * `CountingStream<S>`: wraps any `S: Read/Write` and increments counters on IO.

    * `CountingStream::new(inner, counters)`
    * `CountingStream::into_inner()`
    * `CountingStream::counters() -> Counters`
    * Implements `Read` and `Write` (increments on successful ops).
  * `Counters(Arc<Mutex<State>>)`:

    * `Counters::new()`
    * `add_in(u64)`, `add_out(u64)`
    * `snapshot() -> Snapshot`
    * `reset_minutes()` (preserves totals, clears ring)
  * `Snapshot`:

    * `total_in`, `total_out`
    * `per_min_in: [u64; 60]`, `per_min_out: [u64; 60]`
* **Events / HTTP / CLI:** None.

## 5) Dependencies & Coupling

* **Internal crates:** none (intentionally standalone). Stability: **tight isolation**, replaceable: **yes**.
* **External crates (top):**

  * `anyhow` (workspace): not required by current code path; low risk, MIT/Apache.
  * `parking_lot` (workspace): declared; current code uses `std::sync::Mutex`; consider switching. Mature, MIT.
  * `workspace-hack`: build graph hygiene only.
* **Runtime services:** Uses OS time (`SystemTime`) to bucket by epoch minute; no network, storage, or crypto.

## 6) Config & Feature Flags

* **Env vars:** none.
* **Config structs:** none.
* **Cargo features:** none.
* Effect: behavior is deterministic apart from system clock.

## 7) Observability

* In-memory counters + `snapshot()`; no direct logging or metrics export.
* No `/healthz`/`/readyz`; not a service.
* Intended for consumers (gateway/services) to scrape/emit.

## 8) Concurrency Model

* Shared state behind `Arc<Mutex<State>>` (single small critical section per update).
* Minute rotation occurs on write/read accounting when the observed minute changes; intermediate buckets zeroed.
* No async tasks/channels; backpressure is the caller’s concern.
* No internal timeouts/retries; purely local mutation.

## 9) Persistence & Data Model

* **Persistence:** none (process-local).
* **Data model:** `State { total_in, total_out, ring_in[60], ring_out[60], idx, last_minute }`.
* **Retention:** last 60 minutes of per-minute counts.

## 10) Errors & Security

* **Error taxonomy:** updates are infallible; IO errors come from wrapped `Read/Write` and pass through. Mutex poisoning possible with `std::sync::Mutex` (switching to `parking_lot::Mutex` avoids poisoning semantics).
* **Security:** no secrets, authn/z, TLS, or network I/O here. PQ-readiness N/A.

## 11) Performance Notes

* **Hot path:** `add_in/add_out` called on every successful read/write; O(1) updates and occasional minute-rotation work.
* **Targets:** micro-overhead suitable for per-request and stream accounting.
* **Potential wins:** replace `std::sync::Mutex` with `parking_lot::Mutex`; optionally split totals and ring into separate locks or move to atomics with a clock-advance CAS design if contention appears.

## 12) Tests

* **Unit:** verifies counting and snapshot rotation; verifies `reset_minutes` clears rings but preserves totals.
* **Integration/E2E:** none here (should be covered by consumers).
* **Fuzz/loom:** none.

## 13) Improvement Opportunities

* **Switch to `parking_lot::Mutex`** (already a dependency) to avoid poisoning and improve perf.
* **Document bucket semantics** (index 0 meaning, ordering in `Snapshot`, and how gaps are zeroed) to remove ambiguity for dashboards.
* **Add a light Prometheus adapter** (optional module or feature) to export counters without coupling other crates.
* **Clock abstraction for tests** (injectable time source) for deterministic rotation tests.
* **Atomic design option** if needed: per-minute cell as `AtomicU64` + epoch-minute guard, trading complexity for contention reduction.
* **Remove unused deps** (confirm `anyhow` usage; if unused, drop).

## 14) Change Log (recent)

* 2025-09-14 — Ring buffer minute reset preserves totals; rotation behavior validated by unit tests.
* 2025-09-14 — `CountingStream` wires `Read/Write` to counters; snapshot includes 60-minute arrays.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Small surface, could use more doc comments/examples.
* **Test coverage:** 2 — Core behavior covered; needs more edge cases and time abstraction.
* **Observability:** 2 — Snapshots exist; no exporter.
* **Config hygiene:** 4 — No config; predictable defaults.
* **Security posture:** 4 — In-proc counters; minimal risk.
* **Performance confidence:** 3 — Fine for now; lock choice can be improved.
* **Coupling (lower is better):** 1 — Self-contained and easily swappable.



---

# arti transport

---

crate: arti\_transport
path: crates/arti\_transport
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

SOCKS5-based outbound (Tor/Arti) plus a tiny Tor control-port client to publish a v3 hidden service and hand connections to the core transport `Handler`.

## 2) Primary Responsibilities

* Dial arbitrary `host:port` through a SOCKS5 proxy (Tor/Arti) and return a `ReadWrite` stream.
* Publish a Tor v3 hidden service (ephemeral by default, persistent if configured) and accept inbound connections, piping each to the provided `Handler`.
* Track I/O with shared byte counters (`accounting::Counters` / `CountingStream`).

## 3) Non-Goals

* No direct async runtime integration (uses blocking `std::net` + one background thread).
* No integrated Prometheus/HTTP metrics export, rate limiting, or access control.
* No Tor process management (expects an already-running Tor/Arti with SOCKS & control ports).

## 4) Public API Surface

* **Re-exports:** none.
* **Key types / functions / traits:**

  * `ArtiTransport`: concrete `Transport` impl.

    * `pub fn new(socks_addr: String, tor_ctrl_addr: String, connect_timeout: Duration) -> Self`
    * `pub fn counters(&self) -> Counters`
  * `impl Transport for ArtiTransport` (from local `transport` crate):

    * `fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>>`

      * Dials via `socks::connect_via_socks()`, wraps with `CountingStream`, sets read/write timeouts.
    * `fn listen(&self, handler: Handler) -> Result<()>`

      * Calls `hs::publish_and_serve()` to ADD\_ONION and spawn accept loop; passes each accepted stream to `handler`.
* **Events / HTTP / CLI:** none (library-only).

## 5) Dependencies & Coupling

* **Internal crates:**

  * `transport` → Provides `Transport`, `ReadWrite`, `Handler`. **Tight** (core trait boundary). Replaceable: **no** (by design).
  * `accounting` → Byte counters wrapper. **Loose** (can be replaced with any tallying). Replaceable: **yes**.
* **External crates (top 5):**

  * `socks = "0.3"` → SOCKS5 client used for outbound. Mature, MIT/Apache; moderate maintenance risk (simple, but not high-churn).
  * `anyhow` (workspace) → error plumbing; low risk.
  * `tracing = "0.1"` → logging facade (currently underused in this crate); low risk.
  * (No TLS libs; control port is plain TCP to localhost.)
* **Runtime services:** Requires Tor/Arti SOCKS (e.g., `127.0.0.1:9050`) and ControlPort (e.g., `127.0.0.1:9051`) reachable on localhost. No DB or crypto done locally (keys handled as strings passed to Tor).

## 6) Config & Feature Flags

* **Env vars:**

  * `RO_HS_KEY_FILE` (optional): path to persist the private key for the v3 HS.

    * **Unset** → Ephemeral onion (`Flags=DiscardPK`).
    * **Set & file exists** → Reuse the exact Tor key string (`ED25519-V3:...`).
    * **Set & file missing** → Request NEW key and write it to the path.
* **Constructor args:** `socks_addr`, `tor_ctrl_addr`, `connect_timeout`.
* **Cargo features:** none.

## 7) Observability

* `accounting::Counters` wraps all streams (in/out totals + 60-minute ring).
* Logs: primarily `eprintln!` in accept errors; `tracing` is present but not consistently used here.
* No `/healthz` or readiness endpoints (library).

## 8) Concurrency Model

* **Outbound:** blocking connect via `socks::Socks5Stream`; per-stream socket timeouts set to `connect_timeout`.
* **Inbound HS:** creates a `TcpListener` on `127.0.0.1:0`; publishes ADD\_ONION; spawns **one background thread** that:

  * Loops over `ln.incoming()` and, for each connection, wraps with `CountingStream` and calls `handler(stream)`.
  * Note: calls `handler` **inline** on the accept thread (no per-connection thread/pool). A long-running `handler` will serialize accepts.
* **Ctrl-port:** blocking `TcpStream` client; sends `PROTOCOLINFO`, parses AUTH methods, then `AUTHENTICATE` (SAFECOOKIE or NULL).

## 9) Persistence & Data Model

* No database. Optional persistence of the **exact** Tor private key string into `RO_HS_KEY_FILE` (if configured). Retention: indefinite (left to filesystem).

## 10) Errors & Security

* **Error taxonomy:** Network/IO errors from SOCKS or control port bubble up via `anyhow::Result`. HS publish failures (5xx from Tor) are surfaced with contextual messages. No internal retry/backoff.
* **AuthN/AuthZ:** Auth to control port via Tor’s SAFECOOKIE (preferred) or NULL (if Tor permits). No app-layer auth on inbound connections.
* **Secrets:** The HS private key (if persisted) is stored as a plaintext line (as Tor emits it). **File permissions are not forced** here—caller should ensure secure perms (0600).
* **TLS:** None at this layer (SOCKS/control are plaintext localhost).
* **PQ-readiness:** N/A in this crate (delegated to Tor; onion keys are Ed25519).

## 11) Performance Notes

* **Hot paths:** Per-IO accounting increments; SOCKS connect; HS accept → `handler`.
* **Targets:** Suitable for low/medium throughput; overhead minimal except when `handler` blocks.
* **Bottlenecks / wins:**

  * Single accept thread + inline `handler` → backpressure if `handler` is slow. Consider spawning per-conn worker threads or handing off to a channel/async runtime.
  * Switch `eprintln!` to `tracing` with structured fields (service id, peer addr, duration).
  * Optional: move to non-blocking/Tokio for better scalability.

## 12) Tests

* **Unit:** none present in this crate zip.
* **Integration/E2E:** none here (E2E expected in higher layers’ smoke tests).
* **Fuzz/loom:** none.

## 13) Improvement Opportunities

* **Concurrency:** Offload each accepted connection to a worker (thread or async task) so accept loop stays hot.
* **Observability:** Use `tracing::{info,warn,error}`; attach counters snapshots to periodic logs; expose simple adapters for Prometheus (optional module).
* **Security:** When persisting HS key, ensure the file is created with `0600` (platform-appropriate) and avoid leaving it world-readable.
* **Resilience:** Add limited retries/backoff for control-port operations (authenticate, ADD\_ONION, DEL\_ONION).
* **Config hygiene:** Consider optional envs for SOCKS/CTRL addresses or a tiny Config struct to avoid hardcoding in call sites.
* **Error clarity:** Normalize Tor control errors (250/550 parsing) into typed errors (retryable vs terminal).
* **Overlap signals:** This crate is the Tor/Arti transport; avoid duplicating HS logic elsewhere (e.g., gateway/node overlay). If another crate publishes onions, factor a shared `tor_ctrl` util.

## 14) Change Log (recent)

* 2025-09-14 — Initial `Transport` impl with SOCKS outbound, HS publish via control port; added byte accounting on all streams; ephemeral/persistent HS modes via `RO_HS_KEY_FILE`; best-effort `DEL_ONION` on drop.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Small, clear surface; needs doc comments and examples.
* **Test coverage:** 1 — No unit/integration tests yet.
* **Observability:** 2 — Counters present; tracing underused; no exporter.
* **Config hygiene:** 3 — Constructor args + one env; could consolidate in a config struct.
* **Security posture:** 3 — SAFECOOKIE support; improve key file permissions/handling.
* **Performance confidence:** 3 — Fine for modest load; accept loop serialization is a known limiter.
* **Coupling (lower is better):** 2 — Intentional tie to `transport`; other deps are lightweight and swappable.



---

# common

---

crate: common
path: crates/common
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Shared foundation crate: hashing (BLAKE3), address formatting/parsing, `NodeId`, and a small `Config` loader used across services.

## 2) Primary Responsibilities

* Provide stable utilities: BLAKE3 helpers (`b3_hex`, `b3_hex_file`), address helpers (`format_addr`, `parse_addr`, `shard2`), and `NodeId`.
* Define a repo-wide `Config` with defaults and disk loader (TOML/JSON).
* Re-export commonly used helpers so higher layers don’t duplicate logic.

## 3) Non-Goals

* No networking, async runtime, or service hosting.
* No metrics/HTTP endpoints; no persistence beyond reading config files.
* No cryptographic key management beyond hashing helpers.

## 4) Public API Surface

* **Re-exports:** `b3_hex`, `b3_hex_file`, `format_addr`, `parse_addr`, `shard2`.
* **Key types / functions / traits:**

  * `NodeId([u8; 32])` (`Serialize`, `Deserialize`, `Clone`, `Copy`, `Eq`, `Hash`)

    * `NodeId::from_bytes(&[u8]) -> Self` (BLAKE3 of input → 32 bytes)
    * `NodeId::from_text(&str) -> Self`
    * `NodeId::to_hex(&self) -> String`
    * `NodeId::as_bytes(&self) -> &[u8; 32]`
    * `impl FromStr for NodeId` (expects 32-byte hex; errors otherwise)
    * `impl fmt::Debug` prints hex
  * `Config` (serde-friendly)

    * Fields:
      `data_dir: PathBuf`,
      `overlay_addr: SocketAddr`,
      `dev_inbox_addr: SocketAddr`,
      `socks5_addr: String`,
      `tor_ctrl_addr: String`,
      `chunk_size: usize`,
      `connect_timeout_ms: u64`,
      `hs_key_file: Option<PathBuf>`
    * `impl Default` with sane localhost defaults (9050/9051, etc.)
    * `Config::connect_timeout() -> Duration`
    * `Config::load(path: impl AsRef<Path>) -> anyhow::Result<Self>` (TOML or JSON)
  * Hash/address helpers (in `hash` module):

    * `b3_hex(bytes: &[u8]) -> String` (64-hex)
    * `b3_hex_file(path: &Path) -> io::Result<String>` (streams with 1 MiB buffer)
    * `format_addr(hex64: &str, tld: &str, explicit_algo_prefix: bool) -> String`

      * Yields `b3:<hex64>.<tld>` when `explicit_algo_prefix` is true; otherwise `<hex64>.<tld>`
    * `parse_addr(s: &str) -> Option<(String /*hex64*/, String /*tld*/)>`

      * Accepts with or without `b3:` prefix; lowercases hex; validates 64 hex chars
    * `shard2(hex64: &str) -> &str` (first two hex chars for sharding)
* **Events / HTTP / CLI:** None.

## 5) Dependencies & Coupling

* **Internal crates:** none (intentionally base/leaf). Replaceable: **yes**.
* **External crates (top 5; via `Cargo.toml`):**

  * `blake3` — hashing; mature, Apache/MIT; low risk.
  * `serde`, `serde_json`, `toml` — config and DTOs; very stable; low risk.
  * `anyhow` — ergonomic error contexts at API edges; low risk.
  * `hex` — encoding; tiny, stable.
  * `thiserror` — available for typed errors (not heavily used yet).
  * (`workspace-hack` is build-graph hygiene only.)
* **Runtime services:** Filesystem only (config read; optional file hashing). No network or crypto services.

## 6) Config & Feature Flags

* **Env vars:** none.
* **Config structs:** `Config` (see fields above). `Default` uses localhost for overlay/inbox, Tor SOCKS (9050) and control (9051), `chunk_size = 1<<16`, and `connect_timeout_ms = 5000`.
* **Cargo features:** none.
* **Effect:** Deterministic behavior; TOML/JSON auto-detect in `Config::load`.

## 7) Observability

* None built-in (no metrics/log/health). Intended to be imported by services that expose metrics/logging.

## 8) Concurrency Model

* None; purely synchronous helpers and types. No internal threads, channels, or locks.

## 9) Persistence & Data Model

* **Persistence:** None (reads config from disk when asked).
* **Data model:** Simple types (`NodeId`, `Config`) and address strings; address sharding via first two hex chars.

## 10) Errors & Security

* **Errors:** `anyhow::Error` for `Config::load` (read/parse issues). `NodeId::from_str` enforces exact 32-byte hex. Address parsing returns `Option` for invalid formats.
* **Security:** No secrets, no TLS. Hashing is BLAKE3 (not a password hash; fine for content addressing).
* **PQ-readiness:** N/A (no key exchange or signatures here).

## 11) Performance Notes

* `b3_hex`/`b3_hex_file` are O(n) with minimal overhead; file hashing uses a 1 MiB buffer to reduce syscalls.
* `shard2` is constant time; address format/parse do small string ops.
* Suitable for hot paths in overlay/gateway; no global state or locks.

## 12) Tests

* **In-crate:** none in this archive.
* **Recommended additions:**

  * Golden tests for `parse_addr`/`format_addr` (with/without `b3:`; invalid hex; bad TLD).
  * `Config::load` round-trips for TOML/JSON; missing fields → defaults or errors as intended.
  * `NodeId` determinism (bytes/text) and `FromStr` failure modes.
  * `b3_hex_file` on small/large files (0B, 1B, >1 MiB).

## 13) Improvement Opportunities

* **Typed address:** Introduce a `struct ObjAddr { hex64, tld }` with `Display/FromStr` to replace raw tuples and reduce mistakes.
* **Strengthen parsing:** Validate TLD charset (e.g., `[a-z0-9]{1,16}`) and reject mixed-case hex; already lowercased but codify constraints.
* **Error taxonomy:** Use `thiserror` for `ConfigError`, `AddrParseError` instead of `anyhow` for library callers.
* **Docs/examples:** Add rustdoc examples for all public fns; explain sharding convention and `b3:` prefix semantics.
* **Config cohesion:** Consider a `common::prelude` and/or a single `Config` read path (env var for path, `RO_CONFIG`), plus `Display` for human-friendly dumps.
* **Test clock/file abstractions:** Not needed here except to make file hashing tests cleaner (temp files) and config loading predictable.

## 14) Change Log (recent)

* 2025-09-14 — Added `Config::load` (TOML/JSON), `Default` values; exported hash/address helpers; introduced `NodeId` with hex/parse helpers.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Compact and sensible; add rustdoc + typed address to reach 4–5.
* **Test coverage:** 1 — No in-crate tests yet.
* **Observability:** 0 — By design (library only).
* **Config hygiene:** 4 — Defaults + loader are clean; could add env override and stricter validation.
* **Security posture:** 4 — No secrets; safe helpers; clarify non-use of hashing for authentication.
* **Performance confidence:** 4 — Tiny, CPU-bound hashing/string ops; unlikely bottleneck.
* **Coupling (lower is better):** 1 — Leaf utility crate; widely depended on, but itself loosely coupled.



---

# gateway

---

crate: gateway
path: crates/gateway
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

HTTP façade and OAP/1 ingress for RustyOnions: serves `b3:<hex>.<tld>` bundles over Axum with ETag/Range/precompressed negotiation, optional payment enforcement, quotas, and Prometheus metrics; plus a lightweight OAP server for framed ingest/telemetry.

## 2) Primary Responsibilities

* Serve immutable bundle artifacts via `GET/HEAD /o/:addr/*rel` by fetching bytes from `svc-overlay` over UDS (ron-bus envelope).
* Enforce basic edge policies: conditional GET (ETag), byte ranges, precompressed selection (`.br`, `.zst`), per-tenant token-bucket quotas, and optional `402` per `Manifest.toml`.
* Expose health/readiness and `/metrics` (Prometheus); provide an OAP/1 server (`oapd`) that validates BLAKE3 digests and emits kernel events.

## 3) Non-Goals

* No direct storage or on-disk indexing; all object lookup comes via services.
* No TLS termination, auth, or complex access control (beyond quotas/payment guard).
* No write path for bundles (OAP server is protocol echo/validate + eventing, not persistence).

## 4) Public API Surface

* **Re-exports:**

  * `routes::router()` → `axum::Router<()>` (stateless; state injected at serve-time).
  * `state::AppState` (shared clients and flags).
  * `oap::OapServer`.
* **Key types / functions / traits:**

  * `routes::router()`: mounts `/o/:addr/*tail`, `/healthz`, `/readyz`, `/metrics`, and a JSON 404 fallback; applies metrics middleware.
  * `routes::object::serve_object(Extension<AppState>, Path<(addr, rel)>, ...) -> IntoResponse`:

    * ETag (`"b3:<hex>"`) derivation from address, If-None-Match short-circuit (`304`).
    * Accept-Encoding negotiation for `.br`/`.zst` sibling artifacts.
    * Single-range (`bytes=a-b`) parsing and `206` slicing; invalid ranges → `416`.
    * Content-Type guess from extension (html/js/css/wasm/img/…).
    * Optional `402 Payment Required` via `pay_enforce` on `Manifest.toml`.
  * `state::AppState { index: Arc<IndexClient>, overlay: Arc<OverlayClient>, enforce_payments: bool }`.
  * `overlay_client::OverlayClient::from_env_or(...).get_bytes(addr, rel) -> Result<Option<Vec<u8>>>` (UDS, rmp-serde framed).
  * `index_client::IndexClient::resolve_dir(addr) -> Result<PathBuf>` (UDS helper; not in the hot GET path).
  * `quotas::{check(tenant) -> Option<retry_after_secs>}` (global token-bucket).
  * `metrics::{metrics_handler, record_metrics, ...}` (Prometheus registry + middleware).
  * `oap::OapServer::serve(addr) -> (JoinHandle, bound_addr)`; framed OAP/1: `Hello` → `Start{topic}` → many `Data{hdr+body}` (b3 verified) → `End`, with windowed ACKs and concurrency gating.
* **Events / HTTP / CLI:**

  * HTTP: `/o/:addr/*rel`, `/healthz`, `/readyz`, `/metrics`.
  * CLI (bin): `gateway` (bind address, `--enforce-payments`), `gateway-oapd` (OAP server with ack window/max-frame/concurrency).
  * Bus events: emits `KernelEvent::{Health,ConfigUpdated,ServiceCrashed}` from OAP server.

## 5) Dependencies & Coupling

* **Internal crates → why / stability / replaceable?**

  * `oap` (tight): frame codec + helpers (`OapFrame`, `read_frame`, `ack_frame`, `b3_of`). Replaceable: **no** (protocol boundary).
  * `ron-kernel` (loose): event bus types; used for OAP telemetry. Replaceable: **no** (kernel contract).
* **External crates (top 5; pins via workspace):**

  * `axum` (0.7.x via workspace): HTTP server/routing. Risk: low; active.
  * `tokio`: async runtime. Risk: low.
  * `tower` / `tower-http` (trace/compression): middleware. Risk: low.
  * `prometheus`: metrics registry/encoders. Risk: low; text encode stable.
  * `rmp-serde` / `serde`: message framing for UDS calls. Risk: low.
  * Others used: `tracing`, `clap` (bins), `reqwest` in tests.
* **Runtime services:**

  * **Network:** TCP listener for HTTP; optional OAP TCP listener.
  * **Storage:** none (delegated to services).
  * **OS:** UDS clients to `svc-overlay` (`RON_OVERLAY_SOCK`) and `svc-index` (`RON_INDEX_SOCK`).
  * **Crypto:** BLAKE3 verification via `oap` helper (OAP path only).

## 6) Config & Feature Flags

* **Env vars:**

  * `RON_OVERLAY_SOCK` (default `/tmp/ron/svc-overlay.sock`).
  * `RON_INDEX_SOCK` (default `/tmp/ron/svc-index.sock`).
  * `RON_QUOTA_RPS` (float), `RON_QUOTA_BURST` (float) for token-bucket quotas.
* **Config structs:** `AppState` assembled in `main.rs`; router itself is stateless.
* **Cargo features:** `legacy-pay` (wraps older enforcer type; default off).
* **Effect:** Env controls sockets/quotas; CLI controls bind address and payment enforcement.

## 7) Observability

* **Metrics (Prometheus):**

  * `requests_total{code}`, `bytes_out_total`, `request_latency_seconds`,
    `cache_hits_total` (304s), `range_requests_total` (206s),
    `precompressed_served_total{encoding}`, `quota_rejections_total` (429s).
  * `/metrics` handler exports text format; middleware records status/latency/bytes.
* **Health/readiness:** `/healthz` (liveness), `/readyz` (checks UDS reachability to overlay/index with 300ms timeout).
* **Logging:** `tracing` used (e.g., request summaries in object route, server bind info). Some error paths still use best-effort mapping helpers.

## 8) Concurrency Model

* **HTTP path:** Axum/Tokio. Handlers are `async`, but UDS clients use **blocking** `std::os::unix::net::UnixStream` I/O; this can pin a runtime worker thread under load. No per-request concurrency limiter; backpressure is via Tokio + quotas (if enabled).
* **Quotas:** global `OnceLock<Quotas>`; per-tenant token buckets stored behind a `tokio::sync::Mutex<HashMap<...>>`; time arithmetic via `Instant`.
* **OAP server:** One `TcpListener`, per-connection task gated by a `Semaphore` (`concurrency_limit`). In-task loop enforces `max_frame`, ACK windowing, and frame sequencing. Control errors publish `ServiceCrashed`.
* **Retries/Timeouts:** HTTP path defers to service timeouts; readiness uses 300ms connect timeout. OAP path responds protocol errors and closes.

## 9) Persistence & Data Model

* **Persistence:** none (stateless gateway).
* **Artifacts:** serves raw bundle bytes fetched by `(addr, rel)`; supports sibling precompressed variants.
* **Retention:** N/A at this layer.

## 10) Errors & Security

* **HTTP error taxonomy:** JSON envelopes for `404 not_found`, `413 payload_too_large`, `429 quota_exhausted (+ Retry-After)`, `503 unavailable (+ Retry-After)`. `416` with `Content-Range: */len` for invalid ranges. `304` for conditional GET.
* **Payment guard:** `Manifest.toml` `[payment] { required, price, currency, price_model }` → `402` when enabled; best-effort and explicitly permissive on malformed/absent manifests.
* **AuthN/Z:** none built-in (beyond quotas and payment policy flag).
* **TLS:** none (delegate to fronting proxy/load balancer).
* **Secrets:** none handled here.
* **PQ-readiness:** N/A (no key exchange); OAP digest validation relies on BLAKE3 (content addressing), not cryptographic signatures.

## 11) Performance Notes

* **Hot paths:** overlay UDS fetch + response assembly; conditional/range logic; precompressed selection.
* **Targets:** suitable for medium throughput; response bytes currently buffered in memory (`Vec<u8>`).
* **Potential bottlenecks / wins:**

  * **Blocking UDS** inside async handlers → convert to `tokio::net::UnixStream` or offload to `spawn_blocking`.
  * **Streaming** large payloads using `axum::body::Body::from_stream` instead of full-buffer returns.
  * Increment `cache_hits_total` on `304` paths (present but not currently bumped).
  * Consider small read-side cache for manifest reads if `--enforce-payments` is common.

## 12) Tests

* **Integration:** `tests/http_read_path.rs` (requires `OBJ_ADDR` and gateway URL envs), covering: 200 read, conditional `304`, basic range `206`, invalid range `416|200`, Accept-Encoding negotiation sanity.
* **Unit:** helpers (range parsing, content-type, etag derivation) are tested implicitly; could use focused unit tests.
* **E2E:** exercised by repo smoke scripts (services+gateway).
* **Fuzz/loom:** none.

## 13) Improvement Opportunities

* **Async UDS & streaming:** Replace blocking `UnixStream` in `OverlayClient`/`IndexClient` with Tokio UDS; introduce streaming bodies for large assets.
* **Metrics completeness:** Wire `cache_hits_total` (on `304`) and `quota_rejections_total` (on `429`) from the route paths; add histogram buckets.
* **Error surface unification:** There are two JSON error helpers (`routes/errors.rs` and `http/error.rs`); consolidate into one canonical module.
* **Config hygiene:** Promote a small `GatewayConfig` (sockets, quotas, enforce\_payments) and env override (`RO_*`) to reduce ad hoc reads.
* **Backpressure:** Optional per-route concurrency limiter or `tower::limit` to smooth spikes, complementary to quotas.
* **Remove unused coupling:** `IndexClient` is not needed on the hot read path—only `/readyz`. Consider narrowing `AppState`.
* **Security hooks:** Pluggable auth (e.g., HMAC token or mTLS via front proxy) if exposure expands.
* **Payment guard hardening:** Typed price model and clearer error envelope for `402`; optional `Link`/docs in response.

## 14) Change Log (recent)

* 2025-09-14 — Added precompressed selection (`.br`, `.zst`) and single-range parsing; consolidated health/readiness; Prometheus metrics + middleware.
* 2025-09-14 — Introduced OAP/1 server (`gateway-oapd`) with BLAKE3 validation, ACK windowing, and concurrency gating.
* 2025-09-13 — Refactored index resolution to UDS `svc-index`; gateway read path now uses `svc-overlay` UDS exclusively.

## 15) Readiness Score (0–5 each)

* **API clarity:** 4 — Router/state/OAP surfaces are clear; consolidate error module to reach 5.
* **Test coverage:** 3 — Solid integration test; add unit tests for helpers and negative cases.
* **Observability:** 4 — Rich Prometheus set + health/ready; wire remaining counters.
* **Config hygiene:** 3 — Env/CLI split works; a typed config would help.
* **Security posture:** 3 — Minimal edge checks; no TLS/auth (by design); payment guard is best-effort.
* **Performance confidence:** 3 — Fine now; blocking UDS and full-buffer responses are the main risks.
* **Coupling (lower is better):** 2 — Proper service boundaries; minor internal duplication (errors) and an unnecessary state field (index) on hot path.



---

# index

---

crate: index
path: crates/index
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny library that maintains and queries the mapping from a content address (`b3:<hex>`) to currently known providers, enforcing TTLs and signatures, so callers can resolve where to fetch bytes without speaking the DHT directly. &#x20;

## 2) Primary Responsibilities

* Resolve `b3:<hex>` → provider list (node\_id + addr), honoring TTL and limits.&#x20;
* Accept provider announcements and validate Ed25519 signatures + expirations.&#x20;
* Offer a fast, local cache and persistence backing used by `svc-index` and tools like `tldctl pack`. &#x20;

## 3) Non-Goals

* Running a network-facing API (that’s `svc-index`; this crate is the embeddable core).&#x20;
* Implementing DHT lookups or peer gossip (delegated to `svc-dht`/discovery).&#x20;
* Storing or serving object bytes (that’s Storage/Overlay).&#x20;

## 4) Public API Surface

* **Re-exports:** (none yet; keep minimal)
* **Key types / fns (proposed for clarity):**

  * `Provider { node_id: String, addr: SocketAddr, expires_at: u64, sig: Vec<u8> }` (matches announce schema).&#x20;
  * `Index::announce(hash: &str, provider: Provider) -> Result<()>` (validates TTL/signature).&#x20;
  * `Index::resolve(hash: &str, limit: usize) -> Vec<Provider>` (sorted/stable, cap by `limit`).&#x20;
  * `Index::gc(now_epoch: u64)` (purges expired records).
* **Events / HTTP / CLI:** none in the lib; `svc-index` exposes `GET /resolve` and `POST /announce` using this crate.&#x20;

## 5) Dependencies & Coupling

* **Internal crates:**

  * *ron-kernel* (loose; for health/metrics patterns if embedded) — replaceable: **yes**.&#x20;
  * *svc-index* (service wrapper uses this lib; tight but directional).&#x20;
* **External crates (expected top 5):**

  * `sled` (embedded KV store for provider sets). Risk: moderate (maintenance), permissive license. (Implied by `RON_INDEX_DB` workflows.)&#x20;
  * `blake3` (addressing; verification alignment). Low risk.&#x20;
  * `ed25519-dalek` (signature verification on announces). Moderate risk; widely used.&#x20;
  * `serde` (+`rmp-serde`/`rmpv` for compact storage) — low risk. (Used broadly across services.)&#x20;
  * `parking_lot` (locks) — low risk.
* **Runtime services:** local disk (Sled DB), system clock (TTL), optional bus for health. &#x20;

## 6) Config & Feature Flags

* **Env vars:** `RON_INDEX_DB` → path to the Sled database used by pack/index/services (must match across tools).&#x20;
* **Cargo features (suggested):**

  * `verify-sig` (on by default) toggles Ed25519 checks.&#x20;
  * `inmem` switches to an in-memory map for tests.
* **Constants alignment:** `b3:<hex>` addressing, OAP/1 `max_frame = 1 MiB` (docs alignment; not used directly here). &#x20;

## 7) Observability

* Expected to surface counters via the host service (`svc-index`): `requests_total{code}`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`.&#x20;
* Health/readiness endpoints exist at the service layer; the lib should expose cheap `stats()` for wiring.&#x20;

## 8) Concurrency Model

* Library is synchronous over a `RwLock` around the store (Sled is thread-safe); no background tasks required.
* Backpressure and timeouts live in the service layer; the lib enforces `limit` and TTL checks to keep ops O(log n) per key. (Service backpressure/429 rules per blueprint.)&#x20;

## 9) Persistence & Data Model

* **Store:** Sled at `RON_INDEX_DB`. Keep keys small; one key per object hash.&#x20;
* **Suggested keys:**

  * `prov/<b3hex>` → `Vec<Provider>` (sorted by freshness; dedup by `node_id`).
  * `meta/<b3hex>` → compact stats (optional).
* **Retention:** purge on read or periodic GC using `expires_at` from announces.&#x20;

## 10) Errors & Security

* **Error taxonomy (service level):** map to JSON envelope `{code,message,retryable,corr_id}`; 400/404/413/429/503. (Lib returns typed errors for the service to translate.)&#x20;
* **Security controls:**

  * Verify announce signatures (Ed25519) and reject expired records.&#x20;
  * Addressing is **BLAKE3-256** `b3:<hex>`; services must verify digest before returning bytes. (Consistency with the rest of the system.)&#x20;
* **PQ-readiness:** none required here; PQ plans land in OAP/2 and e2e layers.&#x20;

## 11) Performance Notes

* Hot path is `resolve(hash, limit)` — aim for p95 < 40 ms intra-node (matches internal API SLOs).&#x20;
* Keep provider vectors small (≤ 16) per `limit` to avoid copying cost.&#x20;

## 12) Tests

* **Unit:** TTL pruning; signature validation; dedup/sort order.
* **Integration:**

  * With `svc-index` HTTP: `/resolve` limit handling; `/announce` rejects bad sigs/expired TTLs.&#x20;
  * With `tldctl` pack → resolve → gateway fetch (ensures a single `RON_INDEX_DB` path to kill phantom 404s).&#x20;
* **E2E:** covered indirectly by `gwsmoke` (Gateway↔Index↔Overlay↔Storage read path).&#x20;
* **Fuzz/loom:** N/A (lib is data-structure focused); optional proptests for announce/resolve invariants.

## 13) Improvement Opportunities

* **Gaps / tech debt:**

  * No canonical on-disk schema defined yet (document key prefixes + version byte).
  * Sled-lock footgun when tools and daemon share the DB; prefer daemonized access.&#x20;
* **Overlap & redundancy signals:**

  * Announce/resolve logic is also sketched in `svc-discovery`/DHT; ensure this lib remains the **single source** for provider-record validation to avoid drift.&#x20;
* **Streamlining:**

  * Add an in-proc cache layer (LRU) with TTL to reduce Sled hits.
  * Provide a tiny UDS façade so `tldctl` can talk to the daemon instead of opening Sled directly (eliminates lock contention).&#x20;
  * Ship golden metrics in `svc-index` using the standard set.&#x20;

## 14) Change Log (recent)

* **2025-09-14** — First deep-dive crate analysis; aligned with `svc-index` spec (resolve/announce, TTL/sig).&#x20;
* **2025-09-05–09-06** — Gateway↔Index↔Overlay read path proven by `gwsmoke`; integration tests for gateway read-path landed (context for index consumers). &#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — spec is crisp, but types/traits need to be finalized in-code.&#x20;
* **Test coverage:** 2 — E2E exists around it; direct unit/integration for this lib are TBD.&#x20;
* **Observability:** 3 — patterns defined; needs wiring in `svc-index`.&#x20;
* **Config hygiene:** 3 — `RON_INDEX_DB` pattern is established; formal config struct not yet standardized.&#x20;
* **Security posture:** 3 — Ed25519 verify + TTL planned; PQ not in scope here.&#x20;
* **Performance confidence:** 3 — local lookups should meet SLOs; needs micro-bench + LRU.&#x20;
* **Coupling (lower is better):** 2 — clean separation from DHT and gateway; used by `svc-index`.&#x20;




---

# kameo

---

crate: kameo
path: crates/kameo
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A lightweight, in-process actor toolkit (mailboxes, supervisors, and typed request/response) for building RustyOnions services with predictable concurrency, backpressure, and restarts.

## 2) Primary Responsibilities

* Provide a minimal actor runtime: `Actor` trait, `Addr<T>` handles, bounded mailboxes, and a typed `ask` pattern.
* Enforce supervision & backoff policies (crash isolation, restart limits, jitter) with metrics/log signals.
* Offer ergonomic helpers to integrate actors into our kernel (spawn, graceful shutdown, bus/health hooks).

## 3) Non-Goals

* Not a distributed actor system (no remote transport, routing tables, sharding).
* Not a replacement for the kernel bus (kameo is in-proc; bus is cross-service).
* Not a general scheduler/executor (relies on Tokio; no custom runtime).

## 4) Public API Surface

* **Re-exports:** (keep minimal) `tokio::task::JoinHandle`, `tokio::sync::{mpsc, oneshot}` when feature-gated.
* **Key types / functions / traits (expected/stable surface):**

  * `trait Actor`: `type Msg; async fn handle(&mut self, msg: Self::Msg, ctx: &mut Ctx<Self>) -> Result<()>;`
  * `struct Addr<A: Actor>`: `send(msg)`, `try_send(msg)`, `ask<R>(msg) -> Result<R>`, `close()`.
  * `struct Ctx<A: Actor>`: access to timers, spawn\_child, stop, actor name/id, supervised restarts.
  * `struct Supervisor`: strategies (`Always`, `OnPanic`, `Never`), `Backoff{min,max,jitter,reset_after}`.
  * `spawn_actor(actor, MailboxCfg) -> (Addr<A>, JoinHandle<()>)`.
  * `MailboxCfg { capacity, overflow: DropNewest|DropOldest|Reject, warn_hwm }`.
  * `AskCfg { timeout, buffer }` for typed request/response via oneshot.
* **Events / HTTP / CLI:** none; logs/metrics published for the host service to expose.

## 5) Dependencies & Coupling

* **Internal crates:**

  * *ron-kernel* (loose): optional integration to publish `KernelEvent::{ServiceCrashed,Health}` and use `Metrics` counters; replaceable: **yes**.
  * *ron-bus* (loose): not required; bus remains inter-service IPC, while kameo is in-proc.
* **External crates (top 5; likely pins/features):**

  * `tokio` (rt, sync/mpsc, time); low risk, core dependency.
  * `tracing` (spans, errors, actor ids); low risk, widely maintained.
  * `thiserror` or `anyhow` (error taxonomy); low risk.
  * `parking_lot` (fast locks for registry/counters); low risk.
  * `futures`/`async-trait` (if trait methods are async); moderate risk (dyn overhead), standard.
* **Runtime services:** none beyond process resources & clock. No network/crypto/storage in this crate.

## 6) Config & Feature Flags

* **Cargo features:**

  * `metrics` (default): register Prometheus counters/histograms in the hosting service.
  * `tracing` (default): structured spans with actor name/id and message types.
  * `bus-hooks`: emit `KernelEvent` signals via the kernel bus on crash/restart.
  * `loom-tests`: enable loom models in tests only.
* **Env vars:** none directly; consumers decide sampling/verbosity via standard `RUST_LOG` and service config.

## 7) Observability

* **Metrics (by actor label):**

  * `kameo_messages_total{actor,kind=received|handled|rejected}`
  * `kameo_mailbox_depth{actor}` (gauge) and high-watermark.
  * `kameo_handle_latency_seconds{actor}` (histogram).
  * `kameo_restarts_total{actor,reason}`; `kameo_failures_total{actor,kind=panic|error}`.
* **Health/readiness:** expose a cheap `stats()`/`is_idle()` for wiring into `/readyz` at the service layer.
* **Tracing:** span per message handle; include cause chain on failure; supervisor restart spans with backoff.

## 8) Concurrency Model

* **Execution:** one Tokio task per actor; single-threaded `handle` guarantees (no concurrent `handle` on same actor).
* **Mailboxes:** bounded `tokio::mpsc` per actor; overflow policy selected via `MailboxCfg`.
* **Backpressure:** callers use `send` (awaits when full) or `try_send` (fail fast) based on path criticality; `ask` uses bounded in-flight with timeout.
* **Supervision:** parent/child tree; on panic or error return, supervisor applies backoff (exponential + jitter) until `max_restarts` within `window` trips to `Stopped`.
* **Cancellation/Shutdown:** cooperative: `close()` stops intake; drain tail up to budget; `on_stop()` hook for cleanup.

## 9) Persistence & Data Model

* None. Kameo manages ephemeral in-memory state only (actor state + mailboxes). No DB or schema.

## 10) Errors & Security

* **Error taxonomy:**

  * `SendError::Full|Closed|Rejected` (retryable vs terminal);
  * `AskError::Timeout|MailboxClosed|Canceled` (retryable depends on caller);
  * `ActorError::Fatal|Recoverable` (guides supervisor policy).
* **Security:** not a trust boundary; no TLS/keys. Avoids `unsafe`. Guards against unbounded growth (bounded queues, HWM warnings).
* **PQ-readiness:** N/A (in-proc). Downstream services handle crypto.

## 11) Performance Notes

* **Hot paths:** mailbox enqueue/dequeue and `handle` dispatch.
* **Targets (guidance, p95 on dev iron):** enqueue < 5µs, context switch \~10–20µs, `handle` user-work dominates; end-to-end `ask` p95 < 5ms for local actors under nominal load.
* **Techniques:** avoid message serialization; prefer move semantics; batch draining (`recv_many`) when available; pre-allocate small vectors; minimize per-msg logging (sample).

## 12) Tests

* **Unit:** mailbox overflow policies; `ask` timeout; supervisor backoff windows; restart limits; `close()` drain semantics.
* **Integration:** actor trees (parent/child failure propagation), metrics labels emitted; interaction with kernel bus (feature `bus-hooks`).
* **E2E (service-level):** wire a demo service using kameo actors for request handling and assert `/readyz`/`/metrics` stability under burst.
* **Loom/proptests:** model mailbox send/close races; ensure no lost-wake deadlocks; proptest restart/backoff invariants.

## 13) Improvement Opportunities

* **Known gaps / tech debt:**

  * Clarify `Actor::handle` cancellation semantics (what happens if a long `await` is canceled during shutdown).
  * Document overflow policy guidance per call-site (when to choose `try_send` vs `send`).
  * Add `Addr::map_err` helpers to unify error mapping at service boundaries.
* **Overlap & redundancy signals:**

  * Potential duplication with `ron-kernel` “supervisor hooks” and restart logic—decide single source of truth for backoff formulas and event emission to avoid drift.
  * If any in-crate “registry” mirrors kernel registries, collapse into one.
* **Streamlining:**

  * Provide a tiny `kameo::service` adapter: actor as HTTP handler (Axum) with bounded concurrency per route.
  * Optional small LRU “inbox shadow” for prioritization (e.g., drop duplicate keys).
  * Add `recv_many`/micro-batching feature to reduce per-message overhead on hot actors.

## 14) Change Log (recent)

* 2025-09-14 — First formal crate review; aligned surface to kernel patterns; documented metrics and supervision policies.
* 2025-09-05 — Feature-gate `bus-hooks` to emit `KernelEvent` on restarts without hard coupling.
* 2025-09-01 — Bounded mailbox defaults and overflow policy introduced; ask timeout made configurable.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — surface is small and conventional (`Actor`, `Addr`, `Supervisor`), needs final trait bounds & docs.
* **Test coverage:** 2 — core paths outlined; loom/proptests planned.
* **Observability:** 3 — metrics/tracing design is clear; needs wiring + exemplars.
* **Config hygiene:** 4 — feature flags are straightforward; no env coupling.
* **Security posture:** 4 — no network/crypto; bounded resources; panic isolation.
* **Performance confidence:** 3 — bounded queues & Tokio mpsc are solid; add micro-benchmarks and `recv_many`.
* **Coupling (lower is better):** 2 — optional bus/metrics hooks only; otherwise standalone.




---

# naming

---

crate: naming
path: crates/naming
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Defines the **canonical content address syntax** for RustyOnions (`b3:<64-hex>.<tld>`), plus helpers to parse/validate/normalize addresses and to read/write **Manifest v2** metadata.

## 2) Primary Responsibilities

* **Addressing:** Parse and format RustyOnions addresses (BLAKE3-256 only), normalize case/prefix, and verify hex length/characters.
* **Typing:** Enumerate and parse **TLD types** (e.g., `image`, `video`, `post`, …) to keep addresses semantically scoped.
* **Manifests:** Provide a stable **Manifest v2** schema and a writer for `Manifest.toml` (core fields, optional encodings/payments/relations/ext).

## 3) Non-Goals

* **No resolution** from address → providers (that’s the index service).
* **No byte serving/storage** (overlay/storage handle bytes).
* **No network/API** surface; this is a pure library with light filesystem I/O for manifest writing only.

## 4) Public API Surface

* **Re-exports:** none currently.
* **Key types / functions / traits:**

  * `hash` module:

    * `const B3_LEN: usize = 32`, `const B3_HEX_LEN: usize = 64`
    * `fn b3(bytes: &[u8]) -> [u8; 32]`
    * `fn b3_hex(bytes: &[u8]) -> String`
    * `fn parse_b3_hex(s: &str) -> Option<[u8; 32]>`
  * `tld` module:

    * `enum TldType { Image, Video, Audio, Post, Comment, News, Journalist, Blog, Map, Route, Passport, ... }`
    * `impl FromStr` and `Display` with lower-case mapping; `TldParseError`
  * **Address** (duplicated—see “Improvement Opportunities”):

    * In `address.rs`: `struct Address { hex: String, tld: TldType }` with `impl FromStr` expecting `<hex>.<tld>` and `AddressParseError`.
    * In `lib.rs`: a second `struct Address { hex: String, tld: String }` with comments allowing optional `b3:` and canonical `Display` as `b3:<hex>.<tld>`.
  * `manifest` module:

    * `struct ManifestV2 { schema_version, tld, address, hash_algo, hash_hex, bytes, created_utc, mime, stored_filename, original_filename, encodings[], payment?, relations?, license?, ext{} }`
    * `struct Encoding { coding, level, bytes, filename, hash_hex }`
    * `struct Payment { required, currency, price_model, price, wallet, ... }`
    * `struct Relations { reply_to?, thread?, source?, ... }`
    * `fn write_manifest(dir: &Path, &ManifestV2) -> Result<PathBuf>`
* **Events / HTTP / CLI:** none.

## 5) Dependencies & Coupling

* **Internal crates:** none required at compile time (good isolation). Intended consumers: `gateway`, `svc-index`, `svc-overlay` (loose coupling).
* **External crates (top 5 and why):**

  * `blake3` — content addressing; **low risk**, active.
  * `serde`, `thiserror`, `anyhow` — data model & errors; **low risk**, standard.
  * `toml` — manifest encoding; **low risk**.
  * `mime` / `mime_guess` — present in `Cargo.toml`, likely for future MIME derivation; **currently unused** (see debt).
  * `uuid`, `chrono`, `base64` — also declared; **currently unused** in visible code (drift).
* **Runtime services:** none (pure CPU & small FS writes for manifests). No network/crypto services beyond BLAKE3 hashing.

## 6) Config & Feature Flags

* **Env vars:** none.
* **Cargo features:** none declared. (Opportunity: feature-gate optional `mime_guess`, `uuid`, etc., or remove until used.)

## 7) Observability

* No metrics/logging inside the crate (appropriate for a pure library). Downstream services should tag failures when parsing/validating addresses or manifests.

## 8) Concurrency Model

* Not concurrent; pure functions and data types. No async, no background tasks. Thread-safe by construction (no globals).

## 9) Persistence & Data Model

* **Address:** `b3:<64-hex>.<tld>` canonical form (lower-case hex, lower-case TLD). Accepting input with or without `b3:` is implied by comments, but behavior diverges between the two `Address` impls (see below).
* **Manifest v2:** TOML document with **stable core** (schema\_version=2, tld, address, hash, size, mime, filenames, timestamp) and **optional sections**:

  * `encodings[]` (e.g., zstd/br precompressions)
  * `payment?` (simple micropayment hints)
  * `relations?` (reply/thread/source)
  * `license?` (SPDX or freeform)
  * `ext{}` (namespaced TLD-specific extras)
* **Retention/Artifacts:** `write_manifest()` writes `Manifest.toml` into a bundle directory; no reads provided yet (writer-only API).

## 10) Errors & Security

* **Errors:**

  * `AddressParseError { Invalid, Tld(TldParseError) }`
  * `TldParseError { Unknown(String) }`
  * `manifest::write_manifest` returns `anyhow::Error` on serialization/FS failures.
* **Security posture:**

  * Strict hash length and hex-char checks (`parse_b3_hex`) and lower-case normalization reduce ambiguity.
  * No signatures/PKI here (by design). No unsafe code.
  * Recommend constant-time equality when comparing digests (callers).

## 11) Performance Notes

* Hot paths are string parsing and hex validation; all O(n) over short inputs—**negligible** vs I/O.
* BLAKE3 helpers are thin wrappers; rely on upstream performance.

## 12) Tests

* Present: `hash` unit tests (hex length, lowercase guarantee).
* Missing/needed:

  * `Address` round-trips (with and without `b3:` prefix; mixed case input).
  * TLD parsing (valid set + unknown TLD rejection).
  * Manifest round-trip: build → write → parse (when a reader is added).
  * Property tests: random hex strings of non-64 length must be rejected; case normalization.

## 13) Improvement Opportunities

### A) **Eliminate duplication and drift (highest priority)**

* There are **two different `Address` definitions**:

  * `address.rs`: `tld: TldType` with structured TLD + strong parse errors.
  * `lib.rs`: `tld: String` with comments about accepting `b3:`; uses `anyhow`.
* **Action:** Keep **one canonical `Address`** in `address.rs` (typed TLD), re-export it from `lib.rs`, and implement:

  * `impl Display` → `b3:<hex>.<tld>`
  * `impl FromStr` that accepts **both** `b3:<hex>.<tld>` and `<hex>.<tld>` and normalizes to canonical.
  * `fn parse_loose(s) -> Result<Address, AddressParseError>` (optional) if we need a forgiving parser for UX, else keep `FromStr` strict with optional prefix allowed.
* Add `pub mod tld; pub mod hash; pub mod address; pub mod manifest;` in `lib.rs` and remove the stray `Address` duplicate there.

### B) **Tighten validation & helpers**

* Provide `Address::from_bytes(tld: TldType, payload: &[u8]) -> Address` using `b3_hex`.
* Provide `Address::verify_payload(&self, payload: &[u8]) -> bool` (hash checks).
* Add `fn validate_manifest(v: &ManifestV2) -> Result<()>` to ensure:

  * `v.hash_algo == "b3"`, `len(hash_hex)==64`, lowercase.
  * `v.address == format!("b3:{hash_hex}.{tld}")`.
  * `mime` aligns with `tld` (soft check) using `mime_guess` if we keep it.

### C) **Dependencies hygiene**

* Remove or feature-gate currently unused deps (`uuid`, `chrono`, `mime`, `mime_guess`, `base64`).
* If we keep `chrono`, switch `created_utc` to `DateTime<Utc>` in API and serialize to RFC3339 via serde (or keep as `String` but justify).

### D) **Manifest ergonomics**

* Add `fn read_manifest(path: &Path) -> Result<ManifestV2>`.
* Provide builders: `ManifestV2::new_basic(addr, bytes, mime, filenames...)` and `with_encoding(...)`, `with_payment(...)`, etc.
* Document the `ext{}` namespace conventions per TLD (short guide in README).

### E) **MIME ↔ TLD policy**

* If MIME is involved, create a small mapping:

  * e.g., `image/* → TldType::Image`, `video/* → Video`, `audio/* → Audio`, `application/geo+json → Map/Route`, etc.
* Keep it **non-authoritative** (advisory only), exposed via helper fns.

### F) **Consistency & docs**

* README shows intent (“gateway/overlay use canonical form”) but is truncated; expand with examples:

  * Valid/invalid addresses, acceptance of `b3:` prefix, and canonical rendering.

## 14) Change Log (recent)

* **2025-09-14** — First deep review; identified **Address type duplication** and unused dependencies; documented Manifest v2 structure and validation needs.
* **2025-09-13..09-06** — (Implied) Iterations on TLD set and manifest fields; added `encodings`, `payment`, `relations`, and `ext` blocks.

## 15) Readiness Score (0–5 each)

* **API clarity:** 2 — useful pieces exist, but **duplicate `Address`** types and missing `lib.rs` re-exports create confusion.
* **Test coverage:** 2 — some `hash` tests; missing address/TLD/manifest round-trips.
* **Observability:** 3 — not needed here; downstream services should log failures.
* **Config hygiene:** 2 — unused deps suggest drift; no features to scope them.
* **Security posture:** 4 — strict hex checks, no unsafe; recommend adding manifest/address cross-validation helpers.
* **Performance confidence:** 5 — tiny string/hex ops; BLAKE3 is fast; no hot-path risks.
* **Coupling (lower is better):** 1 — clean, standalone; consumed by multiple services without runtime ties.




---

# node

---

crate: node
path: crates/node
role: lib (primarily a CLI binary with a tiny placeholder lib)
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A developer-facing CLI that runs a single-process RustyOnions **overlay listener** and offers **PUT/GET** helpers (TCP and optional Tor), plus a tiny JSON stats socket for quick local testing.

## 2) Primary Responsibilities

* **Serve**: start an overlay listener bound to a local TCP address (or Tor HS when enabled) using the embedded store path.
* **Client ops**: PUT a local file and GET a blob by `b3:<hex>` from a listener for demos/smoke tests.
* **Dev stats**: expose minimal, ad-hoc JSON counters (bytes in/out, store size) for local inspection.

## 3) Non-Goals

* Not a microkernel-managed “service”; no supervision tree or bus integration.
* Not the production gateway (no HTTP API surface, no auth, no multi-tenant features).
* Not a persistence authority beyond the overlay’s own sled store; no index, no naming resolution.

## 4) Public API Surface

* **Re-exports**: none (the lib target is intentionally empty/placeholder).
* **Key functions (binary/CLI)**

  * `Serve { bind, transport, store_db }` → runs overlay listener (`TCP` today; legacy Tor path in old commands).
  * `Put { to, path }` → connect and upload file; prints content hash.
  * `Get { from, hash, out }` → fetch by hex hash; write to file.
* **Legacy (unwired code under `src/commands/*`)**

  * `serve(config, transport = "tcp"|"tor")` using `TcpTransport` or `ArtiTransport`, spawns a tiny metrics socket on `dev_inbox_addr`.
  * `put/get(..., transport)`, `tor_dial`, `init` (writes a default `config.toml`), `stats_json`.
* **Events / HTTP / CLI**: CLI only (clap). The dev stats socket speaks a minimal HTTP/1.1 response with a JSON body; there’s **no** Prometheus endpoint here.

## 5) Dependencies & Coupling

* **Internal crates**

  * `overlay` (tight, replaceable: **yes**): provides `run_overlay_listener`, `client_put/get`, `Store` (sled) and transport-agnostic variants in legacy path.
  * `common` (loose, replaceable: **yes**): reads `Config` (used by legacy commands).
  * `transport` (loose, replaceable: **yes**): TCP transport; Tor control helpers in legacy path.
* **External crates (top)**

  * `tokio` (multithread + macros + net + signal) — runtime for Ctrl-C and async clients.
  * `clap` — CLI parser (derive).
  * `tracing`, `tracing-subscriber` — logs and `RUST_LOG` env filter.
  * `color-eyre`/`anyhow` — human-friendly error context.
  * *(Legacy code)* `arti_transport` — Tor SOCKS + HS publish; currently **not in Cargo.toml**, indicating drift.
* **Runtime services**:

  * **Network**: TCP listener on `bind`; optional Tor SOCKS + control when using the legacy path.
  * **Storage**: sled DB directory (overlay store).
  * **OS**: signals (Ctrl-C), threads for periodic counter logs in legacy serve.

## 6) Config & Feature Flags

* **CLI flags (current `main.rs`)**: `--bind`, `--transport` (accepts only `tcp`), `--store-db`.
* **Legacy config (via `common::Config`)**:

  * `data_dir`, `overlay_addr`, `dev_inbox_addr`, `socks5_addr`, `tor_ctrl_addr`, `chunk_size`, `connect_timeout_ms`, optional `hs_key_file` (via `RO_HS_KEY_FILE` env).
* **Cargo features**: none today (opportunity: `tor`, `metrics`, `legacy-commands`).
* **Env**: `RUST_LOG` for logging; `RO_HS_KEY_FILE` honored by legacy Tor serve.

## 7) Observability

* **Current**: `tracing` logs; if using legacy `serve`, a tiny TCP socket on `dev_inbox_addr` returns JSON: `{ store: {n_keys,total_bytes}, transport: {total_in,total_out} }`.
* **Gaps**: no Prometheus metrics, no standardized `/healthz`/`/readyz` hooks, and the JSON stats endpoint is non-standard and ad-hoc.

## 8) Concurrency Model

* **Serve path (current)**: overlay listener runs in its own async stack (inside `overlay`); the CLI awaits Ctrl-C via `tokio::signal::ctrl_c()`.
* **Legacy path**: spawns a thread every 60s to log counters; simple accept loop for the stats socket (blocking `TcpListener`).
* **Backpressure**: bounded by overlay/transport internals; CLI PUT/GET calls are synchronous from the user’s perspective.
* **Timeouts/Retries**: client ops rely on overlay/transport defaults; legacy Tor path sets connect timeout via config.

## 9) Persistence & Data Model

* **Store**: sled, rooted at `--store-db` (current) or `config.data_dir` (legacy), chunk size configurable via `chunk_size`.
* **Schema**: managed by `overlay::Store` (content-addressed blobs); node does not define its own keys.
* **Artifacts**: files written during GET; `config.toml` scaffolding via `init`.

## 10) Errors & Security

* **Error taxonomy**: `anyhow` contexts for CLI failures; PUT/GET surface “NOT FOUND” vs IO/transport errors; legacy Tor path differentiates socket vs Tor control errors.
* **Security posture**: no TLS for TCP; optional Tor HS for privacy when legacy serve is used; no authentication/authorization; no secret management beyond HS key file env.
* **PQ**: N/A here—no crypto primitives in this crate proper; relies on transport/overlay choices.

## 11) Performance Notes

* **Hot paths**: overlay read/write (delegated), file IO for PUT/GET, simple JSON stats generation.
* **Targets**: dev-grade—suitable for local smoke (tens to low hundreds of MB/s on loopback via TCP). Tor adds expected latency (hundreds of ms).
* **Tips**: align `chunk_size` to overlay defaults; put sled DB on SSD; avoid running two processes against the same DB (sled lock).

## 12) Tests

* **Present**: none visible in the package; behavior is exercised indirectly by smoke scripts (`gwsmoke`, etc.) outside this crate.
* **Recommended**:

  * **Integration**: spin a temporary listener on an ephemeral port, PUT/GET a temp file (happy path + 404).
  * **Transport matrix**: behind a `tor` feature, mock or noop Arti to validate CLI dispatch without requiring a Tor daemon.
  * **Stats**: ensure the JSON stats socket returns valid JSON and includes monotonic counters.
  * **Config**: `init` writes example config and refuses overwrite.

## 13) Improvement Opportunities

**A. Eliminate drift (highest priority)**

* Two CLIs coexist: the **new** minimal `main.rs` (TCP-only) and a **legacy** `src/cli.rs` + `src/commands/*` tree (TCP+Tor+stats).
* Choose one surface:

  * **Option 1 (lean)**: keep `main.rs` only; delete `src/cli.rs` and `src/commands/*`.
  * **Option 2 (featureful)**: merge the legacy commands into the new clap surface and add features:

    * `--transport {tcp,tor}` with a `tor` Cargo feature, bringing `arti_transport` into `Cargo.toml`.
    * Promote the stats socket to standard `/metrics` Prometheus (reuse `ron-kernel::Metrics` patterns).
* At minimum, remove unused module stubs in `lib.rs` (today it exports an empty `cli` module just to avoid external breakage).

**B. Config & feature hygiene**

* Introduce a crate feature `tor` (default off). When enabled, compile the Arti path; otherwise exclude Tor code.
* Replace ad-hoc env var `RO_HS_KEY_FILE` with a documented config key (`hs_key_file`) and pass it through explicitly.

**C. Observability**

* Replace dev JSON stats with standard **Prometheus** counters/gauges/histograms and `/healthz`/`/readyz`.
* Publish overlay byte counters via the established metrics registry (labels: transport=`tcp|tor`).

**D. Safer sled access**

* Warn or refuse to open the sled store if another process holds the lock; the legacy `stats_json` tries to cope by hitting the stats socket—make this the **only** code path when locked to avoid footguns.

**E. Boundaries & naming**

* Consider renaming the package/binary to `ronode-dev` or `ronode-cli` to signal its dev-only intent vs. kernel services.

**F. Remove dead code**

* If Option 1 is chosen, delete `src/cli.rs` and `src/commands/*` and add a README note that Tor testing moved to a separate example or to `svc-overlay` with flags.

## 14) Change Log (recent)

* **2025-09-14** — Review found **CLI drift** (TCP-only new CLI vs legacy TCP/Tor commands), non-standard stats endpoint, and missing `arti_transport` pin; proposed feature-gated unification.
* **2025-09-05 … 09-12** — Used in local e2e/smoke runs; JSON stats endpoint referenced by `stats_json`.

## 15) Readiness Score (0–5 each)

* **API clarity:** 2 — binary interface is clear, but duplicated/legacy code confuses the crate’s surface.
* **Test coverage:** 1 — no direct tests; relies on external smoke scripts.
* **Observability:** 2 — logs exist; stats endpoint is non-standard; no Prometheus/healthz.
* **Config hygiene:** 2 — two config styles (flags vs file), ad-hoc env var, missing Tor feature pin.
* **Security posture:** 2 — plain TCP; Tor path exists but not unified; no auth.
* **Performance confidence:** 3 — overlay delegates the heavy lifting; adequate for dev.
* **Coupling (lower is better):** 2 — coupled to `overlay` (intentional) but otherwise standalone from kernel services.




---

# oap

---

crate: oap
path: crates/oap
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny, dependency-light codec + helpers for **OAP/1** that parse/serialize frames, enforce bounds, and provide canonical DATA packing (with `obj:"b3:<hex>"`) used by the SDK, gateway, and services. &#x20;

## 2) Primary Responsibilities

* Implement the **OAP/1** wire format and state machine with strict bounds and errors, plus HELLO negotiation helpers. &#x20;
* Provide canonical **DATA packing** helpers that embed `obj:"b3:<hex>"` in a framed header; both sides use the same logic to prevent drift. &#x20;
* Ship normative **test vectors** and parser tests/fuzz to guarantee interop (SDK parity, conformance suite). &#x20;

## 3) Non-Goals

* Not a transport or TLS layer (uses kernel/transport or SDK for I/O); no business logic, economics, or service-level quotas here.&#x20;
* Not a capability system or verifier (macaroons are referenced/encoded, but verification/enforcement lives at services/gateway).&#x20;

## 4) Public API Surface

* **Re-exports:** OAP constants (version, limits), status codes, flags; canonical test vectors (A–T).&#x20;
* **Key types / functions / traits:**

  * `OapFrame { len, ver, flags, code, app_id, tenant, caplen, corr_id, cap, payload }` with `encode/decode`.&#x20;
  * `Flags` bitset (e.g., `REQ`, `RESP`, `START`, `END`, `ACK_REQ`, `APP_E2E`, `COMP`).&#x20;
  * `HelloInfo { max_frame, max_inflight, supported_flags, version }` and `hello()` probe.&#x20;
  * `data_frame()` + `encode_data_payload` / `decode_data_payload` placing `obj:"b3:<hex>"` into the header. &#x20;
  * `Error` enum with typed reasons (bad\_frame, oversize, unsupported\_flag, decompress\_too\_large, unauthorized, quota, timeout). &#x20;
* **Events / HTTP / CLI:** none directly; consumed by gateway/services; vectors runnable via `ronctl test oap --vectors`.&#x20;

## 5) Dependencies & Coupling

* **Internal crates:**

  * *ron-kernel*: none at compile-time (keep codec independent); used by gateway that consumes oap. Tight runtime coupling avoided by design. Replaceable: **yes** (codec could be swapped with a generated one).&#x20;
  * *gateway/sdk*: depend **on** `oap`, not vice-versa (to avoid layering inversions).&#x20;
* **External crates (expected top 5; minimal pins/features):**

  * `bytes` (frame I/O without copies), `serde`/`serde_json` (HELLO/DATA header JSON), optional `zstd` (COMP), optional `tokio` (demo I/O), `uuid` (tenant). Risks: low/maintained; zstd guarded.&#x20;
* **Runtime services:** none (pure codec). TLS and Tor belong to transport/gateway; ALPN/TLS posture is normative input.&#x20;

## 6) Config & Feature Flags

* **Env/config:** n/a for the core crate (limits negotiated via HELLO). `max_frame` defaults to **1 MiB** unless HELLO lowers it.&#x20;
* **Cargo features:** `comp` (zstd compression; enforce 8× bound), `pq-preview` (future macaroons PQ verifier compatibility, no wire change), `tcp-demo` (tokio helpers). &#x20;

## 7) Observability

* The crate itself is logic-only; metrics are emitted at gateway/services. Golden metrics include `requests_total`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`, `quota_exhaustions_total`.&#x20;
* SDK/gateway should expose HELLO timing and parse errors; kernel emits health/service events.&#x20;

## 8) Concurrency Model

* Pure functions for encode/decode; no interior mutability.
* For I/O demos, **ACK\_REQ** + server window with `max_inflight` backpressure; timeouts/retries are caller policy (SDK/gateway). &#x20;

## 9) Persistence & Data Model

* None (stateless codec). OAP embeds `tenant` and optional capability bytes; DATA header includes `obj:"b3:<hex>"` for content addressing. &#x20;

## 10) Errors & Security

* **Error taxonomy:** oversize (payload > negotiated `max_frame`), malformed header, unsupported version/flag, decompressed size bound exceeded (≤ 8× `max_frame`), unauthorized (cap), quota (service). &#x20;
* **Security posture:** TLS 1.3 with ALPN `ron/1` (transport), no TLS compression/renegotiation; `APP_E2E` means packet body is opaque to kernel/services. PQ readiness planned without changing OAP. &#x20;

## 11) Performance Notes

* Hot path is frame parse/serialize and DATA header (small JSON) read/write; use `bytes::Bytes` to minimize copies.
* System SLOs for OAP under load: **p95 < 40 ms**, **p99 < 120 ms** (integration target); enforce 1 MiB `max_frame` and 64 KiB streaming at storage as distinct knobs. &#x20;

## 12) Tests

* **Unit:** vectors A–T (HELLO, REQ|START, cap present/absent, COMP bounded, error cases).&#x20;
* **Integration:** conformance harness over TCP+TLS and Tor; must match SDK bytes/timings.&#x20;
* **Fuzz/property:** parser fuzz (≥ 1000h cumulative over time) and proptests in CI; persist and replay corpus. &#x20;
* **Formal (optional):** TLA+ sketch of state transitions.&#x20;

## 13) Improvement Opportunities

* **Eliminate duplication risk:** Some plans had a parser in `ron-kernel/overlay/protocol.rs`; consolidate into `oap` as the single source of truth and make kernel/gateway depend on it. &#x20;
* **Create `ron-proto` crate:** Move constants/status codes/headers/vectors there to prevent drift across SDKs/services.&#x20;
* **Observability hooks:** Expose lightweight counters inside `oap` behind a feature (e.g., parse errors by reason) to feed golden metrics upstream.&#x20;
* **Leakage harness:** Add padding/jitter toggles and doc guidance (cross-plane leakage checks).&#x20;

## 14) Change Log (recent)

* **2025-09-14** — Drafted deep-dive and alignment with **GMI-1.6** invariants (1 MiB `max_frame`, DATA packing with `b3:`).&#x20;
* **2025-09-13** — Acceptance checklists added to ensure `max_frame` alignment and rejected reasons.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 4 (wire format & helpers are crisp; finalize code-level docs once `ron-proto` lands).&#x20;
* **Test coverage:** 3 (vectors specified; need CI proptests/fuzz soaking).&#x20;
* **Observability:** 3 (golden metrics defined upstream; add error counters in codec if useful).&#x20;
* **Config hygiene:** 5 (negotiated via HELLO; no env in core).&#x20;
* **Security posture:** 4 (TLS 1.3 + ALPN; APP\_E2E opaque; PQ path planned). &#x20;
* **Performance confidence:** 3 (SLO targets defined; need harness results).&#x20;
* **Coupling (lower is better):** 4 (pure library; ensure kernel doesn’t re-implement parser).&#x20;




---

# overlay

---

crate: overlay
path: crates/overlay
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A legacy library that bundles a simple TCP overlay protocol (PUT/GET), a sled-backed blob store, and convenience client/server helpers into one crate.

## 2) Primary Responsibilities

* Implement a minimal overlay wire protocol (PUT → hash; GET ← blob) over TCP.
* Persist and retrieve binary blobs in a local sled database keyed by BLAKE3 hex.
* Provide helpers to run a listener and to act as a simple client (PUT/GET).

## 3) Non-Goals

* No HTTP surface, no OAP/ALPN/TLS, and no capability/tenant enforcement.
* Not a scalable content distribution service (no chunking, no replication).
* Not a metrics/quotas layer; only basic tracing logs.

## 4) Public API Surface

* Re-exports: `pub use store::Store;`
* Key types / functions / traits:

  * `Store` (sled wrapper): `Store::open<P: AsRef<Path>>(path) -> Result<Self>`, `put(&self, key: &[u8], val: Vec<u8>)`, `get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>`
  * Protocol (TCP helpers):

    * `run_overlay_listener(bind: SocketAddr, store_db: impl AsRef<Path>) -> anyhow::Result<()>` — spawns a background TCP server task.
    * `client_put(addr: &str, path: &Path) -> anyhow::Result<String>` — PUT file, returns BLAKE3 hex.
    * `client_get(addr: &str, hash_hex: &str, out: &Path) -> anyhow::Result<()>` — GET by hash to file.
  * Internal error enum exists (`OverlayError`) but the crate’s public functions mostly return `anyhow::Result`, so the typed error is not propagated publicly.
* Events / HTTP / CLI: none (pure library; logging via `tracing`).

## 5) Dependencies & Coupling

* Internal crates → why, stability, replaceable?

  * (None) — pure library; consumers (e.g., gateway or svc-\* crates) call into this.
  * **Replaceable:** yes; the whole crate is a thin layer that could be superseded by `svc-overlay` (network) + `svc-storage` (DB).
* External crates (top 5; pins/features) → why, risk

  * `tokio` (full) — async TCP I/O & tasks; mature/low risk.
  * `sled` — embedded KV store; stable but maintenance is conservative; watch for long-standing issues (crash safety, compaction).
  * `blake3` — fast hashing for content addressing; low risk.
  * `anyhow` / `thiserror` — error ergonomics; low risk.
  * `tracing` — logging; low risk.
  * (Present but **unused** in code): `lru`, `transport`. These indicate drift/tech debt.
* Runtime services: file-system DB via sled; plain TCP. No TLS/crypto beyond hashing.

## 6) Config & Feature Flags

* Env vars: none.
* Config structs: none; the caller supplies `SocketAddr` and DB path.
* Cargo features: none (no conditional compilation for TLS/OAP/chunking).

## 7) Observability

* Logs: `tracing::info!` on bind and `tracing::warn!` when listener task exits with error.
* No metrics, health/readiness, or structured error counters.
* No request IDs/spans; correlation relies on peer address and op type.

## 8) Concurrency Model

* Server: `run_overlay_listener` spawns a background task; inside it, a `TcpListener` accepts connections; each connection is handled on its own `tokio::spawn`.
* Backpressure: none at protocol level; server reads entire request payload into memory based on a declared 8-byte length prefix.
* Locks/timeouts/retries: none explicit; sled provides internal concurrency; network ops rely on default socket timeouts (i.e., effectively unbounded).
* Failure model: background task’s `JoinHandle` is dropped; errors are only logged (no restart/supervision here).

## 9) Persistence & Data Model

* DB: sled (single tree, default namespace).
* Keys: `hash_hex` (BLAKE3, lowercase hex).
* Values: raw blob bytes.
* Durability: `Store::put` calls `db.flush()` after insert (safer for tests/scripts, slower in bulk).
* TTL/GC/retention: none; unlimited growth until external pruning.

## 10) Errors & Security

* Error taxonomy:

  * I/O, sled errors, UTF-8, early EOF, unknown opcode, and a placeholder `InvalidChunkSize` (unused by current protocol).
  * Public API returns `anyhow::Result`, so callers cannot match on the typed `OverlayError`.
* Security posture:

  * Plain TCP with no TLS, ALPN, or authn/z.
  * No rate limiting or quota.
  * No validation of caller identity; unauthenticated writes/reads possible if exposed.
  * PQ-readiness N/A (no TLS/keys).
* Input bounds:

  * PUT: trusts an 8-byte big-endian length prefix (no max bound enforced) → potential memory blow or DoS if exposed.

## 11) Performance Notes

* Hot paths: PUT hashing (BLAKE3 over full payload) and sled `insert + flush`; GET reads full value and writes it out.
* Current behavior is fully in-memory per request: allocates a `Vec<u8>` of exact length; no streaming/chunking.
* SLOs: none defined in code; practical latency will be dominated by FS and blob size; with `flush()` on every PUT, throughput will be poor under sustained load.

## 12) Tests

* No unit or integration tests included in the crate snapshot.
* No fuzz/property tests, no loom concurrency checks, no E2E harness.
* Manual testing implied via client helpers (`client_put`/`client_get`).

## 13) Improvement Opportunities

**Known gaps / tech debt**

* Remove **unused deps** (`lru`, `transport`) or wire them up (e.g., LRU read-cache).
* The `OverlayError` type is not exposed/used by the public API; either propagate it or delete it to avoid confusion.
* No bounds on PUT size; add a configurable max (e.g., 1 MiB like OAP defaults) to prevent DoS.
* `run_overlay_listener` drops its `JoinHandle`; provide a handle or supervision hook for clean shutdown and error handling.

**Overlap & redundancy signals**

* This crate duplicates responsibilities now planned for `svc-overlay` (network protocol) and `svc-storage` (DB). Keeping all three risks drift.
* It also diverges from our OAP+TLS direction: raw TCP here vs. axum/OAP in newer services.

**Streamlining (merge/extract/replace/simplify)**

* **Option A (recommended):** Freeze this crate as **legacy** (tests + docs only), and route all new code to:

  * `svc-overlay` for network protocol (ideally OAP over TLS/ALPN).
  * `svc-storage` for persistence, with sled/SQLite abstraction and quotas.
* **Option B:** If keeping:

  * Add `max_put_bytes` bound; stream PUT (hash while writing) and avoid full-buffer allocs (`tokio::io::copy` + rolling BLAKE3).
  * Add `GET` streaming (length prefix + framed chunks).
  * Introduce metrics: `requests_total{op}`, `bytes_in/out_total`, `errors_total{reason}`, `latency_seconds{op}`, `inflight`.
  * Add feature flags: `tcp` (current), `oap` (reframe), `tls` (via `tokio-rustls`), `cache` (enable LRU), `metrics`.
  * Expose structured errors (`OverlayError`) in public API.
  * Replace per-PUT `flush()` with batched/periodic fsyncs or make it configurable.

## 14) Change Log (recent)

* 2025-09-14 — Deep audit; documented legacy status, identified unused deps and security/perf gaps; proposed retirement in favor of `svc-overlay` + `svc-storage`.

## 15) Readiness Score (0–5 each)

* API clarity: **3** (surface is tiny and easy to call, but error typing is inconsistent and protocol undocumented externally).
* Test coverage: **1** (no tests observed).
* Observability: **1** (logs only; no metrics/health).
* Config hygiene: **2** (caller-provided bind/path is clean; no bounds/features/env).
* Security posture: **1** (plain TCP; unauthenticated; no quotas/bounds).
* Performance confidence: **2** (works for small blobs; `flush()` per PUT hurts throughput; no streaming).
* Coupling (lower is better): **3** (self-contained, but overlaps with the newer service split; two unused deps signal drift).




---

# ron-app-sdk

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




---

# ron-bus

---

crate: ron-bus
path: crates/ron-bus
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Tiny IPC layer for RustyOnions services: MessagePack-framed request/response envelopes over Unix Domain Sockets with minimal helpers for `listen / recv / send`.

## 2) Primary Responsibilities

* Define the **wire envelope** (`Envelope`) and a few shared reply shapes (e.g., `Ok`, `Bytes`, `NotFound`, `Err`).
* Provide **UDS framing** (length-prefixed MsgPack) and **blocking** helpers: `listen(sock)`, `recv(&mut UnixStream)`, `send(&mut UnixStream, &Envelope)`.
* (Today) Centralize **service RPC enums** (e.g., `StorageReq/Resp`, `IndexReq/Resp`, `OverlayReq/Resp`) so services share one protocol surface.

## 3) Non-Goals

* No async runtime integration (helpers are **blocking** on std UDS).
* No transport security (UDS only; **no TLS**; no network sockets).
* No authorization/verification of capability tokens (only carries `token: Vec<u8>`; verification is a service concern).
* Not the in-process **kernel bus** (`ron-kernel::Bus`); this is **inter-process** IPC.

## 4) Public API Surface

* **Re-exports:** none.
* **Key types / functions / traits:**

  * `pub struct Envelope { service: String, method: String, corr_id: u64, token: Vec<u8>, payload: Vec<u8> }`
  * `pub const RON_BUS_PROTO_VERSION: u32 = 1;`
  * `pub struct CapClaims { sub: String, ops: Vec<String>, exp: u64, nonce: u64, sig: Vec<u8> }` (carried inside `token` as MsgPack, not enforced here)
  * UDS helpers (blocking, Unix-only):

    * `pub fn listen(sock_path: &str) -> io::Result<UnixListener>`
    * `pub fn recv(stream: &mut UnixStream) -> io::Result<Envelope>`
    * `pub fn send(stream: &mut UnixStream, env: &Envelope) -> io::Result<()>`
* **Events / HTTP / CLI:** none.

## 5) Dependencies & Coupling

* **Internal crates:** none (good); however, this crate currently **hosts other services’ RPC enums**, which *functionally* couples `svc-index`, `svc-overlay`, `svc-storage`, `gateway`, etc., to this crate’s releases.

  * Stability: **tight** (any RPC enum change fans out here). Replaceable? **Yes**, with a `ron-proto` split (see §13).
* **External (top 5):**

  * `serde` (workspace) — ubiquitous encoding; low risk.
  * `rmp-serde` (workspace) — MessagePack codec; stable; low risk.
  * `thiserror` (workspace) — error ergonomics; low risk.
  * `workspace-hack` — dep dedupe shim; no runtime impact.
* **Runtime services:** **OS UDS** only; no network, no TLS, no crypto primitives.

## 6) Config & Feature Flags

* No cargo features today.
* No config structs; callers pass socket paths. Services define their own defaults (e.g., `/tmp/ron/svc-storage.sock`).

## 7) Observability

* None built-in. No metrics, tracing, or structured error mapping at the IPC layer.
* Errors bubble as `io::Error` or MsgPack decode errors from `rmp-serde`.

## 8) Concurrency Model

* **Blocking I/O** on `std::os::unix::net::UnixStream`.
* Framing: **u32 length prefix** + MsgPack body; `recv` uses a manual `read_exact` loop; `send` writes a length then bytes.
* **Backpressure:** entirely at the OS socket buffer and the caller’s accept loop; **no internal queues**, limits, or timeouts here.
* **Locks/timeouts/retries:** none implemented; callers must add deadlines and retry policies.

## 9) Persistence & Data Model

* None. No DB integration; IPC envelopes are ephemeral. Any persistence is a service concern.

## 10) Errors & Security

* **Error taxonomy:** thin; essentially `io::Error` for socket and codec failures. Reply conveniences include `NotFound` and `Err{err}` variants but enforcement is up to services.
* **Security:** relies on **process boundary + UDS permissions**. `Envelope.token` can carry `CapClaims` (MsgPack) but this crate does **not** verify signatures/expiry.
* **AuthZ/AuthN:** out of scope here; expected to happen in the service handler.
* **TLS/PQ:** N/A at this layer (local IPC only).

## 11) Performance Notes

* **Hot path:** `recv` → decode → handler → `send`.
* Blocking UDS is fast on a single box, but mixing this inside async services (Axum/Tokio) risks **runtime stalls** unless calls stay off the async threads (use `spawn_blocking`).
* **Serialization:** MessagePack via `rmp-serde` is compact; consider zero-copy `bytes::Bytes` for `payload` to avoid copies on large blobs.
* **Targets (suggested):** p95 `recv+decode` < 30µs, p95 `send+encode` < 40µs for small (≤1 KiB) messages on dev hardware.

## 12) Tests

* **Present:** none visible in the crate; callers (e.g., `svc-storage`, `svc-overlay`) use it in their integration flows.
* **Needed:**

  * Unit: round-trip encode/decode; negative tests for short read, truncated prefix, oversized len.
  * Property/fuzz: random byte streams → ensure graceful decode errors without panics or leaks.
  * Loom: not necessary (blocking I/O), but an invariants test for “no partial frame read returns success” is useful.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **Naming collision / drift risk:** We now have two “bus” concepts:

  * `ron-kernel::Bus` = **in-process broadcast** (Tokio broadcast for `KernelEvent`).
  * `crates/ron-bus` = **inter-process IPC** over UDS.
    This routinely confuses readers and encourages scope creep.

* **Coupling hotspot:** `ron-bus` currently **owns service RPC enums** (Index/Overlay/Storage…). That forces every service-level protocol change through this crate and blurs responsibilities.

* **No timeouts / limits / metrics:** `recv` can block indefinitely; there is no max-frame guard; no `rejected_total{reason}` counters; no structured tracing.

* **Blocking only:** In async services, blocking UDS calls must be quarantined to avoid starving the runtime.

### Streamlining (merge/extract/replace/simplify)

1. **Rename & split (strongly recommended):**

   * Rename this crate to **`ron-ipc`** (or `ron-uds`) to kill the “bus” ambiguity.
   * **Extract all RPC enums** into a new **`ron-proto`** crate with submodules (`proto::index`, `proto::overlay`, `proto::storage`), or one crate per service if we want looser coupling. Keep `ron-ipc` transport-only.

2. **Guards & observability:**

   * Add **max frame size** (e.g., 1 MiB default; configurable by caller); reject with a typed error.
   * Add **read/write timeouts** helpers (or document that callers must wrap with `nix`/`poll`/deadline I/O).
   * Optional **Prometheus hooks**: counters for `ipc_bytes_{in,out}_total`, `ipc_frames_total{dir,service}`, `ipc_decode_errors_total{reason}`.

3. **Async interop:**

   * Offer **Tokio variants** behind a feature flag (`tokio-uds`) using `tokio::net::UnixListener/UnixStream` (no behavior changes).
   * Document that blocking helpers should be used from dedicated threads or `spawn_blocking`.

4. **Versioning on wire:**

   * Today we have `RON_BUS_PROTO_VERSION` but do not put it on the wire. Add a fixed **4-byte magic** (`b"RBUS"`) + `u16 version` ahead of the length prefix to hard-fail mismatches early.

5. **Zero-copy & payload ergonomics:**

   * Switch `Envelope.payload` to `bytes::Bytes` and support **borrowed decode** via `rmp-serde` where possible.

6. **Security posture:**

   * Provide optional **cap verification helpers** (ed25519 verify, `exp`/`nonce` checks) in a *separate* crate (e.g., `ron-cap`) so services can share hardened logic without bloating `ron-ipc`.

### Overlap & redundancy signals

* Overlap with **`ron-kernel::Bus`** in name only: different layers but **same term**, which leads to architectural confusion and doc drift. Fix via rename.
* Overlap with future **`ron-proto`** plan in blueprints: this crate shouldn’t be the “god crate” for all service protocols.

## 14) Change Log (recent)

* 2025-09-14 — Initial review; confirmed blocking UDS helpers (`listen/recv/send`), Envelope shape, and `RON_BUS_PROTO_VERSION = 1`; identified service-enum colocation and naming drift risk.
* 2025-09-05 … (insert if we land the split/rename; otherwise keep this slot empty for next pass)

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Envelope and UDS helpers are clear; service RPCs living here muddy boundaries.
* **Test coverage:** 1 — No tests in-crate; relies on downstream integration.
* **Observability:** 1 — No metrics/tracing; opaque errors.
* **Config hygiene:** 2 — Simple, but lacks size/time guards; callers must roll their own.
* **Security posture:** 2 — UDS only; capability bytes carried but not verified; acceptable for localhost IPC, not for network boundaries.
* **Performance confidence:** 3 — Blocking UDS is fast; no guards/benchmarks; potential runtime stalls in async services if misused.
* **Coupling (lower is better):** 2 — Today it’s a coupling hub because it hosts service RPC enums; after `ron-proto` split it would improve to 4–5.

---

### Appendix — Quick API sketch (from the crate)

* **Envelope**

  ```rust
  #[derive(Serialize, Deserialize, Debug, Clone)]
  pub struct Envelope {
      pub service: String,      // e.g., "svc.index"
      pub method: String,       // e.g., "v1.resolve"
      pub corr_id: u64,         // correlation id for RPC
      pub token: Vec<u8>,       // MsgPack<CapClaims> or empty
      pub payload: Vec<u8>,     // MsgPack-encoded method payload
  }
  ```
* **UDS (blocking)**

  ```rust
  pub fn listen(sock_path: &str) -> io::Result<UnixListener>;
  pub fn recv(stream: &mut UnixStream) -> io::Result<Envelope>;
  pub fn send(stream: &mut UnixStream, env: &Envelope) -> io::Result<()>;
  ```
* **CapClaims container** (carried in `token`, verification out of scope here)

  ```rust
  pub struct CapClaims { pub sub: String, pub ops: Vec<String>, pub exp: u64, pub nonce: u64, pub sig: Vec<u8> }
  ```



---

# ron-kernel

---

crate: ron-kernel
path: crates/ron-kernel
role: core
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny, stable microkernel that provides the project’s event bus, metrics/health surfaces, transport glue, and shutdown/supervision hooks—while exporting a frozen public API for the rest of the system.&#x20;

## 2) Primary Responsibilities

* Define and **freeze** the kernel’s public API (Bus, events, metrics/health, config, shutdown helper).&#x20;
* Offer **lossy broadcast** bus primitives and light supervision/backpressure behaviors that keep core paths non-blocking.&#x20;
* Expose **observability** endpoints (`/metrics`, `/healthz`, `/readyz`) that other services rely on for SLOs and runbooks.&#x20;

## 3) Non-Goals

* No app/business semantics (Mailbox, quotas, SDK ergonomics, payment). Those live outside the kernel.&#x20;
* No protocol spec surface (OAP/1 codec lives in its own crate; kernel only exports stable re-exports & helpers).&#x20;
* No persistent storage logic (DHT/Index/Storage are services; kernel stays stateless beyond in-memory counters).&#x20;

## 4) Public API Surface

* **Re-exports (frozen):** `Bus`, `KernelEvent::{Health {service, ok}, ConfigUpdated {version}, ServiceCrashed {service, reason}, Shutdown}`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.&#x20;
* **Key types / functions / traits:**

  * `Bus`: broadcast-based event channel; `new(capacity)`, `publish`, `subscribe` (+ helpers like `recv_with_timeout`, `recv_matching`).&#x20;
  * `KernelEvent`: canonical system events including structured crash reasons.&#x20;
  * `Metrics`: Prometheus counters/histograms + HTTP service for `/metrics`, `/healthz`, `/readyz`.&#x20;
  * `HealthState`: shared registry behind `/healthz`/`/readyz`.&#x20;
  * `Config` + `wait_for_ctrl_c()`: configuration holder and graceful shutdown helper (public re-export).&#x20;
  * **Transport hook (exposed at kernel root):** uses `tokio_rustls::rustls::ServerConfig`—type choice is part of the contract.&#x20;
* **Events / HTTP / CLI:** kernel itself exposes HTTP for `/metrics`, `/healthz`, `/readyz`; events published on `Bus` include `Health`, `ConfigUpdated`, `ServiceCrashed{reason}`, `Shutdown`.&#x20;

## 5) Dependencies & Coupling

* **Internal crates:** None required at runtime; kernel is intentionally standalone to avoid coupling creep. It coordinates with sibling crates (oap, gateway, sdk, ledger) via the frozen API.&#x20;
* **External crates (top focus):**

  * **tokio** (async runtime, channels) — core concurrency; widely maintained.
  * **axum 0.7** (HTTP for metrics/health) — aligned with workspace stack; handlers `.into_response()` pattern is enforced across services.&#x20;
  * **prometheus** (metrics registry/types) — counters/histograms exposed at `/metrics`.&#x20;
  * **tokio-rustls 0.26** (TLS types) — **contractually** the kernel’s TLS type for transports.&#x20;
  * **serde** (DTOs for JSON health/readiness replies).
    *Risk posture:* all are mature, widely used crates; the TLS type selection is explicitly locked to avoid drift.&#x20;
* **Runtime services:** Network (TCP/TLS listeners), OS (signals for shutdown). No direct DB or crypto beyond TLS type selection.&#x20;

## 6) Config & Feature Flags

* **Config:** The crate exports a `Config` type and `/readyz` capacity gates are expected at the service layer; kernel provides the plumbing and health surfaces.&#x20;
* **Features:** Kernel stays minimal; optional adapters (e.g., accounting) are feature-gated in **other crates** (ledger), not here—by design.&#x20;

## 7) Observability

* **Endpoints:** `/metrics`, `/healthz`, `/readyz` exposed by the kernel HTTP server; overload paths return **429/503** downstream when integrated.&#x20;
* **Golden metrics:** kernel-level `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`; service-level tie-ins for `rejected_total{reason}`.&#x20;

## 8) Concurrency Model

* **Bus:** `tokio::sync::broadcast` with **bounded capacity** (default \~4096) to remain lossy under pressure; **never blocks** kernel critical paths. On overflow, increment a counter and emit a throttled `ServiceCrashed{service:"bus-overflow", reason:...}`.&#x20;
* **Service control:** request/response via bounded `mpsc` + oneshot replies (pattern guidance in blueprint).&#x20;
* **Backpressure:** enforced at boundaries; **no unbounded queues** anywhere.&#x20;

## 9) Persistence & Data Model

* **None in kernel.** State is process-local and ephemeral (metrics, health map). Durable data (index/storage/DHT) is strictly out-of-kernel.&#x20;

## 10) Errors & Security

* **Typed events, not app errors:** kernel emits `ServiceCrashed{reason}` and `Health` for downstream operators; app-facing error envelopes (JSON with `{code,message,retryable,corr_id}`) are implemented in services.&#x20;
* **TLS type discipline:** transports must use `tokio_rustls::rustls::ServerConfig`. This guard prevents integration drift.&#x20;
* **E2E privacy boundary:** kernel treats app payloads as opaque; quotas/DoS are enforced outside the kernel (Gateway), keeping kernel unburdened.&#x20;

## 11) Performance Notes

* **Defaults:** OAP/1 `max_frame = 1 MiB` (spec), storage streaming chunk **64 KiB** (implementation detail, not a protocol limit). Avoid conflating these.&#x20;
* **Throughput posture:** bus buffers tuned ≥8 per subscriber; recommended 64 for active nodes.&#x20;

## 12) Tests

* **Unit/Integration (present):** `bus_basic`, `bus_topic`, `bus_load`, `event_snapshot`, plus overlay/index HTTP integration checks referenced in project status. &#x20;
* **Planned/CI gates:** property/fuzz tests for OAP frames live in `oap` crate; chaos/TLA+/loom are tracked as validation gates to reach “perfect”.&#x20;

## 13) Improvement Opportunities

* **Formal/destructive validation:** Land loom/fuzz/TLA+ for the kernel bus and supervision loops to lift M0→perfect.&#x20;
* **Metric completeness:** Ensure `bus_overflow_dropped_total` is exposed and documented; wire tight coupling with `ServiceCrashed{reason}` labels.&#x20;
* **Docs guardrails in CI:** Keep the grep suite that enforces `b3:<hex>` addressing and `max_frame = 1 MiB` vs. **64 KiB** chunking to prevent drift.&#x20;
* **Config watcher polish:** kernel re-exports a `Config`; add a no-surprises watcher stub and document reload semantics (currently implied, not codified).&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Status marked **“Microkernel complete for M0 (\~95%)”** with bus tests green and observability endpoints in place.&#x20;
* **2025-08-30** — **Public API freeze** restated in Final Blueprint; `ServiceCrashed{reason}` and TLS type contract emphasized. &#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 5 — explicit, frozen re-exports and variants.&#x20;
* **Test coverage:** 4 — kernel tests green; formal/chaos pending.&#x20;
* **Observability:** 4 — metrics/health/ready are present; dashboards/runbooks still evolving. &#x20;
* **Config hygiene:** 3 — contract is present; hot-reload/documented watcher semantics to tighten.&#x20;
* **Security posture:** 3 — TLS type fixed; app E2E boundary is honored; capability/quotas live in services. &#x20;
* **Performance confidence:** 3 — sane defaults; bus tuning guidance present; broader perf sims tracked at service layer. &#x20;
* **Coupling (lower is better):** **2** — intentionally minimal, app-agnostic core with stable surface.&#x20;





---

# ryker

---

crate: ryker
path: crates/ryker
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Prototype pricing/monetization utilities for RustyOnions manifests: parse price models, validate payment blocks, and compute request costs.

## 2) Primary Responsibilities

* Translate `naming::manifest::Payment` policies into executable logic (parse price model, compute cost, validate fields).
* Provide lightweight, dependency-minimal validation primitives for wallets and payment blocks.

## 3) Non-Goals

* No payment processing, settlement, exchange-rate handling, or cryptographic verification.
* No networking, storage, or runtime orchestration.
* Not a stable public API (explicitly experimental/scratchpad).

## 4) Public API Surface

* **Re-exports:** none.
* **Key types / functions / traits (from `src/lib.rs`):**

  * `enum PriceModel { PerMiB, Flat, PerRequest }` + `PriceModel::parse(&str) -> Option<Self>`
    Accepted strings (case-insensitive): `"per_mib" | "flat" | "per_request"`.
  * `compute_cost(n_bytes: u64, p: &naming::manifest::Payment) -> Option<f64>`

    * Returns `None` when `p.required == false`.
    * `PerMiB`: `price * (n_bytes / 1,048,576)`.
    * `Flat`/`PerRequest`: returns `price` (bytes do not affect cost).
  * `validate_wallet_string(&str) -> Result<()>`

    * Currently only checks non-empty; comments outline future heuristics (LNURL, BTC, SOL, ETH).
  * `validate_payment_block(p: &Payment) -> Result<()>`

    * Ensures parseable `price_model`, non-empty `wallet`, and `price >= 0.0`.
* **Events / HTTP / CLI:** none.

## 5) Dependencies & Coupling

* **Internal crates:** `naming` (tight) — uses `naming::manifest::{Payment, RevenueSplit}`. Replaceable: **yes**, by defining an internal trait to decouple from concrete `Payment` or relocating logic into `naming`.
* **External (top):**

  * `anyhow` (workspace) — ergonomic errors; low risk.
  * `serde` (workspace) — not used directly in current code; low risk.
  * `workspace-hack` — dedupe shim.
* **Runtime services:** none (pure compute).

## 6) Config & Feature Flags

* No feature flags, no env vars. Behavior is entirely driven by the `Payment` struct inputs.

## 7) Observability

* None. No `tracing` spans, metrics, or structured error taxonomy beyond `anyhow::Error`.

## 8) Concurrency Model

* None. All functions are synchronous, CPU-bound, and side-effect-free.

## 9) Persistence & Data Model

* None. Operates on in-memory `Payment` values from `naming::manifest`.

## 10) Errors & Security

* **Errors:**

  * `validate_*` return `anyhow::Error` on invalid inputs (unknown model, empty wallet, negative price).
  * `compute_cost` uses `Option` to signal “no charge” when policy is not required or parse fails.
* **Security:**

  * No authn/z, no signature checks, no token parsing; only a non-empty wallet string check.
  * No TLS/crypto; not applicable for this pure function crate.
  * PQ-readiness N/A at this layer.

## 11) Performance Notes

* O(1) arithmetic; microseconds per call even at high volumes.
* `PerMiB` math uses `f64`; precision is adequate for display but not settlement-grade accounting.

## 12) Tests

* **Unit tests present:**

  * Cost computation: `per_mib` (2 MiB at \$0.01/MiB ≈ \$0.02), `flat` invariant w\.r.t bytes.
  * Policy gating: `required=false` → `None`.
  * Validation happy-path (`per_request` with non-empty wallet).
* **Gaps to add:**

  * Unknown `price_model` → `parse(None)` and `validate_payment_block` error.
  * Negative `price` → error.
  * Boundary cases (0 bytes; extremely large `n_bytes` for overflow safety).
  * Wallet heuristics unit tests once implemented.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **Float money math:** `f64` is inappropriate for billing-grade arithmetic. Use fixed-precision integers (e.g., micro-units) or `rust_decimal` for currency amounts; define rounding rules.
* **Coupling to `naming`:** Logic is bound to a specific struct; either:

  1. Move this module into `naming` (e.g., `naming::pricing`), or
  2. Extract a small trait (`PaymentPolicy`) consumed by `ryker`, reducing coupling and enabling reuse in other contexts.
* **Error semantics:** Mixed `Option`/`Result` can hide policy typos (e.g., misspelled `price_model` returns `None` and looks like “free”). Consider a stricter mode (feature flag or function variant) that errors on malformed policy vs “not required”.
* **Observability:** Add optional `tracing` spans (e.g., `pricing.compute_cost`) and counters (rejects, unknown model, negative price), guarded by a feature flag to keep the crate lean.
* **Wallet validation:** Implement basic format checks per scheme (LNURL bech32, BTC bech32/base58, SOL length/base58, ETH `0x` + 40 hex) behind feature flags; keep the default permissive.
* **Naming & purpose:** “ryker” is opaque. If promoted beyond scratchpad, consider renaming to `ron-pricing` or `ron-billing` and documenting versioning/compat promises.

### Overlap & redundancy signals

* Overlaps conceptually with any future “billing” logic that might live in `gateway` or `overlay`. Keeping price computation centralized here (or in `naming`) prevents drift.
* If we later add enforcement in services, ensure they call a single shared function (this crate) to avoid duplicated formulas.

### Streamlining

* Provide a **single façade**: `Pricing::from(&Payment).compute(n_bytes)` returning a typed `Amount` newtype with currency + minor units.
* Introduce a **strict mode** API (e.g., `compute_cost_strict`) that errors on malformed policies instead of returning `None`.
* Add currency & rounding policy hooks (banker’s rounding, min charge thresholds).

## 14) Change Log (recent)

* 2025-09-14 — Initial pricing utilities and validations reviewed; unit tests for `per_mib`, `flat`, `required=false`, and validation.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Small and understandable, but experimental name and mixed `Option`/`Result` semantics need polishing.
* **Test coverage:** 3 — Core paths covered; add negative/edge cases.
* **Observability:** 1 — None yet.
* **Config hygiene:** 3 — No config needed; consider feature flags for strictness/validation depth.
* **Security posture:** 2 — Minimal input checks; no scheme validation; fine for prototype.
* **Performance confidence:** 5 — Trivial compute.
* **Coupling (lower is better):** 2 — Tight to `naming::manifest::Payment`; can be improved with a trait or co-location.




---

# svc-crypto

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




---

# svc-index

---

crate: svc-index
path: crates/svc-index
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Local Unix-socket index service that maps RustyOnions content addresses to bundle directories and answers resolve/put requests for peers.

## 2) Primary Responsibilities

* Maintain the **address → directory** mapping (open the index DB; get/set entries).
* Serve **IPC RPCs** over UDS (MsgPack) for `Resolve`, `PutAddress`, and `Health`.
* Emit concise **structured logs** (JSON via `tracing-subscriber`) and stay simple/robust under kernel supervision.

## 3) Non-Goals

* No HTTP; no direct file I/O of bundle contents (that’s `gateway` / `svc-storage`).
* No overlay/network lookups, caching, or fetch from remote peers.
* No authorization policy or capability enforcement beyond local UDS trust.

## 4) Public API Surface

This is a **service**; its “API” is the on-wire RPC (via `ron-bus`).

* **Re-exports:** none.
* **Key RPC DTOs (from `ron_bus::api`):**

  * `IndexReq::{ Health, Resolve { addr }, PutAddress { addr, dir } }`
  * `IndexResp::{ HealthOk, Resolved { dir }, NotFound, PutOk, Err { err } }`
* **Service wiring (in `src/main.rs`):**

  * `listen(sock_path) -> UnixListener` (blocking), accept loop, **thread-per-connection**.
  * `recv(&mut UnixStream) -> Envelope` → decode `IndexReq` via `rmp_serde`.
  * Handle request via `index::Index` methods:

    * `get_bundle_dir(&Address) -> Result<Option<PathBuf>>`
    * `put_address(&Address, PathBuf) -> Result<()>`
  * Respond with `Envelope { service: "svc.index", method: "v1.ok", corr_id, payload=MsgPack(IndexResp), token=[] }`.
* **Utilities:** `to_vec_or_log<T: Serialize>(..) -> Vec<u8>` (MsgPack encode; log on error).
* **Events / HTTP / CLI:** none (health is an RPC variant, not HTTP).

## 5) Dependencies & Coupling

* **Internal crates**

  * `index` (tight): provides the actual DB engine/operations for the mapping. Replaceable: **yes** (e.g., swap sled/other store behind the `index` crate).
  * `naming` (tight): `Address` parsing/normalization (prevents format drift). Replaceable: **no** in practice (all address semantics live there).
  * `ron-bus` (tight): IPC envelopes + UDS helpers. Replaceable: **yes** if/when we split to `ron-proto` + `ron-ipc` (recommended).
* **External crates (top)**

  * `serde`, `rmp-serde`: MsgPack encode/decode (stable, low risk).
  * `tracing`, `tracing-subscriber` (json, env-filter): structured logs (low risk).
  * `regex`: present in Cargo; not used in this file—likely used in `index` crate or is a leftover (watch for dead dep).
* **Runtime services:** OS UDS and filesystem (index DB path).

## 6) Config & Feature Flags

* **Env vars**

  * `RON_INDEX_SOCK` (default **`/tmp/ron/svc-index.sock`**): UDS path to bind.
  * `RON_INDEX_DB` (default **`.data/index`**): DB path opened by `index::Index::open`.
* **Cargo features:** none declared here.
* **Notes:** Defaults are sensible, but the **DB path must match other services** (e.g., gateway, overlay, storage) or you’ll see spurious `NotFound` on resolve.

## 7) Observability

* **Logs:** JSON logs via `tracing-subscriber` + `EnvFilter` (e.g., `info!` on start, resolve/put; `error!` on decode/DB errors).
* **Metrics:** none.
* **Health/Readiness:** RPC `Health → HealthOk`. No Prometheus endpoints.

## 8) Concurrency Model

* **Blocking UDS** listener; **one OS thread per client** (`std::thread::spawn` in accept loop).
* **Backpressure:** OS socket buffers only. No in-process queues, limits, or rate control.
* **Timeouts/Retries:** none at service layer. A stuck client could hold a thread.
* **Shared state:** `Arc<index::Index>`; internal locking is managed by the `index` crate.

## 9) Persistence & Data Model

* **Store:** handled by `index::Index` (likely a sled-backed KV at `${RON_INDEX_DB}`).
* **Key/Value shape (inferred):** `Address` → `PathBuf` (bundle directory).
* **Retention:** stable mapping; no TTL/GC logic in the service.

## 10) Errors & Security

* **Error taxonomy (on RPC):**

  * Decode/IO errors → log and drop client (no response) or respond with `IndexResp::Err`.
  * Missing mapping → `IndexResp::NotFound`.
  * Put failures → `IndexResp::Err { err }`.
* **Security model:** local **UDS only**, no authn/z; relies on file permissions and process boundaries.
* **Hardening gaps:** no peer-credential checks (`SO_PEERCRED`/`LOCAL_PEERCRED`), no allowlist of caller UIDs/GIDs, no max-frame guard (DoS risk).
* **TLS/PQ:** N/A (local IPC).

## 11) Performance Notes

* Hot path is tiny: decode → `index::Index` get/put → encode.
* **Targets (suggested):**

  * p95 `Resolve` (hit) ≤ 200 µs; `NotFound` ≤ 150 µs (DB miss).
  * p95 `PutAddress` ≤ 300 µs.
* Watch for **thread explosion** under many idle clients; consider a small thread-pool or async UDS in future.

## 12) Tests

* **Present (in this crate):** none visible.
* **Recommended:**

  * **Integration (UDS)**: spawn service on a temp socket + temp DB; exercise `Health/Resolve/PutAddress` happy/err paths.
  * **DB path sanity:** set `RON_INDEX_DB` to a temp dir and verify cross-service compatibility (same path → resolves succeed).
  * **Fuzz/property:** random bytes into `recv`/MsgPack decode should never panic or OOM; malformed Address strings must be rejected.
  * **Concurrency:** parallel `PutAddress` and `Resolve` workloads to smoke-test locking in `index::Index`.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **No access control**: any local process can talk to the socket. Add **peer-cred allowlist** (UID/GID) and strict socket perms.
* **No resource guards**: add **max frame size** and **per-request deadline** (e.g., 2s) to avoid stuck handlers.
* **Metrics absent**: add counters (`index_requests_total{op,ok}`), histograms (`index_latency_seconds{op}`), and byte totals.
* **Name/API ownership**: RPC enums live in `ron-bus::api`; this couples service evolution to that crate.

### Overlap & redundancy signals

* **Address normalization** correctly lives in `naming::Address`—keep it there to prevent drift across services.
* Mapping ownership is **centralized** here; remove any ad-hoc maps from other crates (e.g., gateway) to avoid duplication.

### Streamlining (merge/extract/replace/simplify)

1. **Split protocol from transport**: move `IndexReq/Resp` to a new `ron-proto::index` and keep `ron-bus` transport-only.
2. **Guardrails**: add `MAX_REQ_BYTES` env (default 1 MiB) and per-request timeout.
3. **Authn**: optional shared-secret HMAC on envelopes or peer-cred allowlist.
4. **Observability**: feature-gated Prometheus metrics; keep default lean.
5. **Async option**: optional `tokio` UDS backend to avoid thread-per-client under load.

## 14) Change Log (recent)

* 2025-09-14 — Reviewed: blocking UDS accept loop; JSON logs; RPCs `Health/Resolve/PutAddress`; envs `RON_INDEX_SOCK`, `RON_INDEX_DB`; identified guardrails & auth gaps.

## 15) Readiness Score (0–5 each)

* **API clarity:** 4 — RPC surface is small and obvious.
* **Test coverage:** 2 — Needs integration and negative tests.
* **Observability:** 2 — Good JSON logs; no metrics/health endpoints.
* **Config hygiene:** 3 — Sensible defaults; document DB/socket interplay clearly.
* **Security posture:** 2 — Local-only but open; add peer-cred checks and socket perms guidance.
* **Performance confidence:** 4 — Minimal hot path; thread model is fine at current scale.
* **Coupling (lower is better):** 3 — Tied to `index`, `naming`, and `ron-bus::api`; splitting protocol would improve.




---

# svc-omnigate

---

crate: svc-omnigate
path: crates/svc-omnigate
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Multi-tenant gateway that fronts RustyOnions services, enforcing quotas/backpressure, exposing health/metrics/readiness, and brokering app traffic over the OAP/1 protocol to storage/index/mailbox while staying app-agnostic.&#x20;

## 2) Primary Responsibilities

* Enforce per-tenant capacity (token buckets), overload discipline (429/503 + Retry-After), and capacity-aware `/readyz`.&#x20;
* Terminate the external surface (HTTP/OAP/1), route to internal services (index/storage/mailbox), and keep protocol limits distinct from storage chunking. &#x20;
* Export “golden metrics” for requests, bytes, latency, rejects/quotas, and inflight.&#x20;

## 3) Non-Goals

* No app-specific behavior or business logic (keep kernel/services neutral; use SDK + app\_proto\_id for app needs).&#x20;
* No PQ/QUIC/ZK changes to OAP/1 at this layer (future-tracks in OAP/2/R\&D).&#x20;

## 4) Public API Surface

* **Re-exports:** None required for consumers; this is a service binary exposing endpoints.
* **HTTP Endpoints (expected):**

  * `/healthz` (liveness), `/readyz` (capacity-aware readiness), `/metrics` (Prometheus). &#x20;
  * Object read path (via gateway surface proved by gwsmoke; stable ETag/Range/encoding semantics).&#x20;
* **Protocol:** OAP/1 framing and error mapping; HELLO advertises limits; max\_frame = 1 MiB (distinct from storage 64 KiB streaming). &#x20;
* **Error envelope (target):** `{code,message,retryable,corr_id}`; 400/404/413/429/503 + Retry-After.&#x20;

## 5) Dependencies & Coupling

* **Internal crates (via RPC/UDS/HTTP):**

  * `svc-index`, `svc-storage`, `svc-overlay` (read path; content-addressed GET/HAS/stream). Coupling: *loose* (over network/UDS); replaceable=yes.&#x20;
  * `svc-mailbox` (SEND/RECV/ACK; later DELETE/SUBSCRIBE/visibility). Coupling: *loose*; replaceable=yes.&#x20;
  * `ron-kernel` for service bus events & invariants; OAP/1 codec defined outside kernel (no app logic). Coupling: *loose*.&#x20;
* **External crates (expected by repo invariants):** Axum 0.7 (HTTP), Tokio (async), Prometheus client (metrics), tokio-rustls (TLS), Serde/rmp-serde (wire). Risk moderate (well-maintained); TLS uses rustls.&#x20;
* **Runtime services:** Network (HTTP/UDS), Storage (blob/index), OS (sockets), Crypto (caps/tokens; APP\_E2E opaque).&#x20;

## 6) Config & Feature Flags

* **Env & files:** `RON_QUOTA_PATH` (per-tenant rate config), `RON_NODE_URL` / `RON_CAP` for SDK/clients; capacity gating via `/readyz`. &#x20;
* **Service config (Omnigate):** TOML with DoS PoW toggles, QoS for mailbox (WS subscribe), optional federation (off by default, ZK handshake), gRPC control addr.&#x20;
* **Spec source control:** Local `/docs/specs/OAP-1.md` mirrors GMI-1.6 to prevent drift.&#x20;

## 7) Observability

* **Metrics (golden set):** `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `rejected_total{reason}`, `inflight`, `quota_exhaustions_total` (and cache/range counters when serving objects). &#x20;
* **Health/Readiness:** `/healthz` + capacity-aware `/readyz` that gates load.&#x20;
* **Tracing:** Correlation IDs end-to-end; SDK propagates `corr_id`.&#x20;

## 8) Concurrency Model

* **Tasks:** HTTP acceptor; per-tenant token-bucket middleware; backend client pools for index/storage/mailbox; metrics exporter loop.
* **Backpressure/overload:** Token buckets; 429/503 with Retry-After; bounded inflight; compression guardrails to cap decompressed size/ratio.  &#x20;
* **Timeouts/retries:** Error taxonomy + SDK jittered retries respecting Retry-After.&#x20;

## 9) Persistence & Data Model

* **Omnigate:** Stateless beyond in-memory counters/quotas/metrics. Durable content and messages live in backing services (index/storage/mailbox). Read path ensures 64 KiB streaming and BLAKE3 verification in storage.&#x20;

## 10) Errors & Security

* **Error taxonomy:** Canonical JSON envelope; map 400/404/413 (decompress/ratio caps), 429 (quota), 503 (capacity). Include `corr_id` and `Retry-After` where applicable.&#x20;
* **AuthN/Z:** Capability tokens/macaroons managed outside kernel; Omnigate enforces quotas per tenant and optional PoW on hot paths. &#x20;
* **TLS:** rustls stack per project invariant; APP\_E2E payloads remain opaque to services.&#x20;
* **PQ-readiness (road-map):** PQ-hybrid in E2E layer, not altering OAP/1; federation guarded with ZK handshake when enabled. &#x20;

## 11) Performance Notes

* **SLO targets (dev-box/intra-AZ):** p50 < 10 ms, p95 < 40 ms, p99 < 120 ms for service plane; mailbox p95 < 50 ms local; cache hit p95 < 40 ms. &#x20;

## 12) Tests

* **Status:** gwsmoke shows Gateway→Index→Overlay→Storage read path working (ETag/Range/encoding behaviors validated); health/ready/metrics exist; Bronze ring needs golden metrics + red-team. &#x20;
* **Planned/required:** OAP parser proptests + fuzz corpus; compression bomb suite; golden error tests; SDK retry tests. &#x20;

## 13) Improvement Opportunities

* **Close Bronze:** Ship golden metrics, JSON error envelope, and capacity `/readyz` invariants with tests; finalize Mailbox ops (DELETE/SUBSCRIBE/visibility).&#x20;
* **Config hygiene:** Externalize quotas to `RON_QUOTA_PATH`; document default tenant fallbacks.&#x20;
* **Drift guards:** Keep OAP/1 spec a stub that mirrors GMI-1.6; add CI greps for max\_frame vs 64 KiB chunking and BLAKE3 addressing. &#x20;
* **Redundancy watch:** If a legacy `gateway` crate exists, de-duplicate roles with `svc-omnigate` (same surface/metrics/quotas) or merge under one name to avoid split ownership.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Project plan sets Bronze focus: quotas/readyz, golden metrics, red-team, error taxonomy; gwsmoke verified read path. &#x20;
* **2025-09-01** — Interop blueprint (GMI-1.6) finalized; OAP/1 spec is authoritative reference for Omnigate surface.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.5 (OAP/1/HELLO and HTTP surfaces are clear; error envelope pending finalization).&#x20;
* **Test coverage:** 3 (gwsmoke + read-path checks; need fuzz/property/bronze red-team).&#x20;
* **Observability:** 3.5 (health/readyz live; golden metrics partially listed but not all landed).&#x20;
* **Config hygiene:** 3 (quota file & SDK env planned; Omnigate TOML templates exist). &#x20;
* **Security posture:** 3 (caps/PoW hooks and compression guards planned; federation off by default; APP\_E2E opaque). &#x20;
* **Performance confidence:** 3 (SLOs defined; read path proven locally; storage streaming targets identified).&#x20;
* **Coupling (lower is better):** 3 (loose to services via RPC/UDS; shared invariants prevent drift).&#x20;




---

# svc-overlay

---

crate: svc-overlay
path: crates/svc-overlay
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Thin overlay service that answers RPC `Health` and `Get{addr,rel}` over a local UDS “bus,” resolving content addresses via `svc-index` and returning file bytes from `svc-storage`.

## 2) Primary Responsibilities

* Accept `OverlayReq::Get` and return `OverlayResp::{Bytes,NotFound,Err}`.
* Resolve `addr` → directory via `svc-index`, then read a file (default `payload.bin` if `rel==""`) from `svc-storage`.
* Provide a lightweight health probe (`OverlayReq::Health → OverlayResp::HealthOk`).

## 3) Non-Goals

* No public HTTP surface, quotas, or request shaping (that’s `svc-omnigate`/gateway).
* No durable storage, manifests, DHT routing, or chunk management (owned by `svc-storage` / future DHT).
* No application-level crypto semantics (APP E2E stays opaque above the service plane).

## 4) Public API Surface

* **Wire (bus/UDS, MessagePack via `rmp-serde`):**

  * **Request enums (from `ron_bus::api`):** `OverlayReq::{Health, Get{addr:String, rel:String}}`.
  * **Response enums:** `OverlayResp::{HealthOk, Bytes{data:Vec<u8>}, NotFound, Err{err:String}}`.
  * **Envelope fields set by this service:** `service="svc.overlay"`, `method ∈ {"v1.ok","v1.not_found","v1.err"}`, `corr_id` echoed, `token=[]`.
* **No Rust re-exports intended for external consumers (service binary).**

## 5) Dependencies & Coupling

* **Internal crates (via API types, not direct linking):**

  * `ron-bus` — Envelope + UDS helpers (`listen/recv/send`) and shared request/response enums. *Loose; replaceable=yes.*
  * Communicates with **`svc-index`** (`IndexReq::Resolve → IndexResp::{Resolved,NotFound,Err}`) and **`svc-storage`** (`StorageReq::Read{dir,rel} → StorageResp::{File,NotFound,Err}`) over UDS. *Loose; replaceable=yes.*
* **External crates (workspace-pinned):** `anyhow`, `serde`, `rmp-serde`, `tracing`, `tracing-subscriber`.
* **Runtime services:** Unix domain sockets only (POSIX); no network listeners; local filesystem reads happen in `svc-storage`, not here. No TLS at this hop.

## 6) Config & Feature Flags

* **Env vars:**

  * `RON_OVERLAY_SOCK` (default `/tmp/ron/svc-overlay.sock`) — where this service listens.
  * `RON_INDEX_SOCK` (default `/tmp/ron/svc-index.sock`) — where to reach `svc-index`.
  * `RON_STORAGE_SOCK` (default `/tmp/ron/svc-storage.sock`) — where to reach `svc-storage`.
* **Cargo features:** none declared.
* **Filesystem:** creates parent directory for its UDS path if missing (macOS tmpdirs friendliness).

## 7) Observability

* **Logs:** structured `tracing` at `info`/`error` with high-signal records (`%addr`, `%rel`, bytes length, and error contexts).
* **Health:** RPC health handler (`OverlayReq::Health`). No HTTP `/healthz` and no Prometheus metrics in this crate (yet).
* **Correlation:** echoes `corr_id` from inbound Envelope.

## 8) Concurrency Model

* **Server pattern:** blocking UDS listener; **per-connection thread** (`std::thread::spawn`) that handles exactly one request/response pair (receive → match → reply).
* **Backpressure:** OS accept queue + thread scheduling; no explicit inflight limits besides OS resources.
* **Timeouts/retries:** none at this layer; callers (gateway/SDK) should apply deadlines and retries.
* **Sync I/O:** uses `UnixStream` (no Tokio); simple, portable across POSIX, but each connection ties up a thread.

## 9) Persistence & Data Model

* **Stateless.** Holds no durable state; passes through `addr`/`rel` and returns raw bytes.
* **Default file name:** if `rel==""`, reads `payload.bin` within the directory resolved for `addr`.

## 10) Errors & Security

* **Error taxonomy (current):** maps failures to `OverlayResp::Err{err}`; “not found” is explicit (`NotFound`). No structured retry hints (e.g., `Retry-After`) yet.
* **Security:** local UDS hop, no auth; `Envelope.token` not used. Capability tokens/macaroons (if any) would be enforced at the gateway/omnigate layer.
* **Integrity:** does not perform digest verification; relies on upstream index/storage contracts. (Blueprint calls for BLAKE3 addressing/verification—**not yet implemented here**.)
* **Platform:** `std::os::unix` dependency means no Windows build without a compatibility layer.

## 11) Performance Notes

* **Hot path:** `Get → (index resolve) → (storage read) → respond Bytes`.
* **Current behavior:** reads entire file into memory (`Vec<u8>`) before replying—good for small assets, risky for large payloads (latency and memory spikes).
* **Targets (pragmatic for local UDS):** p50 single-file GET under a few ms; introduce chunked/streamed reads to preserve p95 under load.

## 12) Tests

* **In-crate:** none.
* **System/E2E:** validated indirectly when the gateway smoke test exercises overlay via index/storage.
* **Recommended next:** unit tests for request decoding/response mapping; integration tests against stubbed index/storage; golden tests for edge cases (`rel==""`, long `rel`, not-found, storage/index error). Add property tests for “echo `corr_id`” and “never panic on decode/encode.”

## 13) Improvement Opportunities

* **Streaming:** return framed/chunked responses instead of a single `Vec<u8>` to avoid memory bloat and enable large objects.
* **Time bounds:** add read/connect timeouts to index/storage calls, and per-request deadlines.
* **Metrics:** add Prometheus counters/histograms (requests, bytes, latency, errors) to align with observability standards elsewhere.
* **Integrity:** optionally verify BLAKE3 of returned bytes (configurable), or at least assert size/hint from index when available.
* **Concurrency:** move to Tokio (or a bounded thread-pool) for fewer threads under load; bound inflight work and introduce backpressure signals to callers.
* **Security hooks:** plumb `Envelope.token` through and perform capability checks once repo-wide policy is ready.
* **Windows portability:** abstract UDS behind a transport trait to unlock Windows named pipes.

## 14) Change Log (recent)

* **2025-09-14** — Draft UDS service with `Health` + `Get` end-to-end to index/storage; logging hardened; no metrics yet.

## 15) Readiness Score (0–5 each)

* **API clarity:** 2.5 — Minimal, clear request/response; lacks streaming & structured error envelope.
* **Test coverage:** 1.0 — No unit/integration tests in-crate.
* **Observability:** 2.0 — Good logs; no metrics/readiness endpoints.
* **Config hygiene:** 2.5 — Socket paths are env-configurable; sensible defaults; no feature flags.
* **Security posture:** 2.0 — Local UDS only; no auth; integrity verification deferred.
* **Performance confidence:** 2.5 — Fine for small files; needs streaming/backpressure before scale.
* **Coupling (lower is better):** 3.0 — Loose, via bus enums and UDS; clean boundaries with index/storage.




---

# svc-storage

---

crate: svc-storage
path: crates/svc-storage
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Durable, content-addressed DAG store with rolling-hash chunking and Reed–Solomon erasure that serves objects to the overlay in 64 KiB streams, verifying BLAKE3 before returning bytes.  &#x20;

## 2) Primary Responsibilities

* Store objects as a manifest/DAG (DAG-CBOR) and chunks, addressed by `b3:<hex>` (BLAKE3-256).&#x20;
* Provide read-path primitives (`GET`, `HAS`) over an internal API so Overlay can stream **64 KiB** chunks to Gateway/clients. &#x20;
* Maintain durability & availability via erasure coding and background repair.&#x20;

## 3) Non-Goals

* No public HTTP surface, quotas, or tenant policy (that’s Gateway/Omnigate).&#x20;
* No provider discovery or routing (Index/DHT handle resolve/announce).&#x20;
* No application-level behavior or decryption; services verify bytes and keep APP\_E2E opaque.&#x20;

## 4) Public API Surface

* **Re-exports:** none (service binary).
* **Service endpoints (internal):** read-path API offering `GET(hash=b3:<hex>)` and `HAS(hash)`; streaming in 64 KiB chunks (implementation detail distinct from OAP/1 `max_frame=1 MiB`). &#x20;
* **Manifests/DAG:** objects modeled as manifests (DAG-CBOR) with chunk references.&#x20;
* **Admin plane (repo invariant):** `/healthz`, `/readyz`, `/metrics` exposed across services.&#x20;

## 5) Dependencies & Coupling

* **Internal crates → why; stability; replaceable?**

  * **svc-overlay** (caller): streams objects to edge; *loose coupling* over RPC/UDS; replaceable=yes.&#x20;
  * **svc-index** (peer): stores/serves provider records; storage itself does not resolve; *loose*; replaceable=yes.&#x20;
* **External crates (likely, per blueprint standards):**

  * `blake3` (addressing/integrity), `reed-solomon-erasure` (parity/repair), `tokio` & `bytes` (async/zero-copy), telemetry stack (Prometheus, tracing). These are mainstream/maintained → moderate risk. &#x20;
* **Runtime services:** local disk (object/chunk store), OS (files/sockets), internal RPC (UDS/TCP) from Overlay, crypto (BLAKE3 digest). &#x20;

## 6) Config & Feature Flags

* **Store root path** used by pack/tools and services (e.g., `.onions` in quick-start), must align across writer and storage to avoid phantom 404s. &#x20;
* **Streaming/erasure knobs:** 64 KiB streaming chunk is an implementation detail; erasure coding parameters and **repair pacing ≤ 50 MiB/s** per cluster (operational cap). &#x20;
* **Cargo features:** none called out yet (PQ and federation features live elsewhere / future).&#x20;

## 7) Observability

* **Endpoints:** `/healthz`, `/readyz`, `/metrics` present per repo invariant.&#x20;
* **Golden metrics (target set):** `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `inflight`, plus storage histograms; to be standardized across services.&#x20;

## 8) Concurrency Model

* **Runtime:** Tokio multi-threaded; bounded CPU pool for hashing/erasure/compression; cancellations propagate (parent drops cancel children).&#x20;
* **Backpressure:** Semaphores at dials/inbound/CPU; bounded queues; deadlines on awaits.&#x20;
* **Zero-copy:** use `bytes::Bytes`, vectored I/O on hot paths.&#x20;

## 9) Persistence & Data Model

* **Addressing/integrity:** `b3:<hex>` (BLAKE3-256) over plaintext object/manifest root; full digest **MUST** be verified before returning bytes.&#x20;
* **Layout:** Manifest (DAG-CBOR) → chunk files (rolling-hash 1–4 MiB) with parity shards; background repair jobs maintain RF. &#x20;

## 10) Errors & Security

* **Error taxonomy (converging):** canonical 2xx/4xx/5xx with `Retry-After` on 429/503; storage should map internal conditions accordingly once adopted repo-wide.&#x20;
* **Mutual auth / secrets (non-public planes):** mTLS/Noise on internal planes; amnesia mode and secret handling policies apply across services. &#x20;
* **Integrity:** services verify BLAKE3 equality for full digests prior to serving bytes.&#x20;
* **PQ-readiness:** OAP/1 unchanged; PQ lands later (OAP/2/SDK), not a storage service concern yet.&#x20;

## 11) Performance Notes

* **Hot path:** manifest resolve → chunk reads → stream **64 KiB** → return; keep internal API p95 < 40 ms, p99 < 120 ms. &#x20;
* **Repair pacing:** cap erasure repair at ≤ 50 MiB/s per cluster to avoid cache thrash.&#x20;

## 12) Tests

* **Now:** local end-to-end read path proven (Gateway→Index→Overlay→Storage 200 OK).&#x20;
* **Planned (M2):** implement `GET`/`HAS` streaming w/ BLAKE3 verify; minimal tileserver example; latency histograms.&#x20;
* **Future:** property/fuzz on chunker & manifest traversal; chaos (IO stalls, partial reads) as part of red-team.&#x20;

## 13) Improvement Opportunities

* **Land the read path fully** (GET/HAS + 64 KiB streaming + verify on read) and ship the tileserver example.&#x20;
* **Golden metrics & dashboards**: align counters/histograms with Gateway/Overlay.&#x20;
* **Erasure/repair ops:** enforce pacing; surface repair/backlog metrics.&#x20;
* **Drift killers:** CI greps for BLAKE3 terms and OAP/1 vs 64 KiB chunking to prevent spec drift.&#x20;
* **DX:** keep store root path consistent between packers and storage to eliminate “phantom 404s”.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — M2 tickets defined for **Storage read-path (GET/HAS, 64 KiB streaming)** with tileserver example.&#x20;
* **2025-09-05** — Local **Gateway→Index→Overlay→Storage** manifest fetch validated via `gwsmoke`.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.0 — Role and interfaces are clear; needs a short in-repo API note.&#x20;
* **Test coverage:** 2.5 — E2E smoke proven; unit/property/chaos tests pending for chunker/erasure. &#x20;
* **Observability:** 3.0 — Health/ready/metrics baseline exists; golden set not fully wired. &#x20;
* **Config hygiene:** 3.0 — Clear store-root pattern; alignment required across tools/services; streaming/erasure knobs defined. &#x20;
* **Security posture:** 3.0 — BLAKE3 verification mandated; internal plane auth policies documented; PQ slated for OAP/2. &#x20;
* **Performance confidence:** 3.0 — Targets set; streaming and repair pacing specified; needs sustained tests. &#x20;
* **Coupling (lower is better):** 3.0 — Loosely coupled to Overlay/Index over internal RPC; clear boundaries.&#x20;

---



---

# tldctl

---

crate: tldctl
path: crates/tldctl
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Library + thin CLI for **packing local files into content-addressed bundles** (BLAKE3 `b3:<hex>`) with manifest and precompressed variants, writing to the store root and index DB, and printing the canonical address.  &#x20;

## 2) Primary Responsibilities

* **Pack**: build a bundle (`Manifest.toml`, `payload.bin`, optional `.br`/`.zst`) and compute the BLAKE3 address (`b3:<hex>.<tld>`). &#x20;
* **Persist**: write bundle to a **store root** (e.g., `.onions`) and update **Index DB** at the configured sled path.&#x20;
* **Report**: print the final address to STDOUT for downstream use (e.g., Gateway GET).&#x20;

## 3) Non-Goals

* No HTTP service surface, quotas, or readiness/metrics (this is a local packer, not a server). (Implied by CLI usage only.)&#x20;
* No DHT/provider routing, overlay streaming, or storage durability; those live in services (`svc-index`, `svc-overlay`, `svc-storage`).&#x20;
* No application-level crypto; it uses **content addressing** only (BLAKE3 `b3:<hex>`).&#x20;

## 4) Public API Surface

* **Re-exports:** none (internal lib consumed by its own bin + tests using “reuse the packing routine”).&#x20;
* **Key functions (implied by usage & docs):**

  * `pack(input, tld, index_db, store_root) -> PackedAddr` (builds bundle, writes to store/index, returns `b3:<hex>.<tld>`).&#x20;
  * Helpers for **precompression** (`.br`, `.zst`) and manifest emission (`Manifest.toml`).&#x20;
* **CLI (bin target):** `tldctl pack --tld <text|…> --input <file> --index-db <path> --store-root <dir>`; prints the computed address.&#x20;

## 5) Dependencies & Coupling

* **Internal crates**

  * **svc-index** (writes/reads the **sled** Index DB path used by services). *Coupling: medium (direct DB access today); replaceable: yes (shift to UDS daemon call).* &#x20;
  * **svc-storage** (indirect—consumes bundles later; no direct link at pack time). *Loose; replaceable: yes.*&#x20;
* **External crates (likely top set, by features used):**

  * `blake3` (addressing), `zstd` / `brotli` (precompression), `toml`/`serde` (manifest I/O), `sled` (Index DB). *All mainstream; moderate risk.* &#x20;
* **Runtime services:** **Filesystem** (store root), **sled DB** (index); **no network/TLS**.&#x20;

## 6) Config & Feature Flags

* **Env compatibility (used in docs/scripts):**

  * `RON_INDEX_DB` — sled DB location used by pack and services.&#x20;
  * `OUT_DIR` / `--store-root` — bundle root (e.g., `.onions`).&#x20;
* **CLI args:** `--tld`, `--input`, `--index-db`, `--store-root`.&#x20;
* **Features:** none documented.

## 7) Observability

* **Logs:** standard CLI logs (stderr).
* **No metrics or health endpoints** (library/CLI only); services provide `/healthz` `/readyz` `/metrics`.&#x20;

## 8) Concurrency Model

* **CLI pipeline** (compute → precompress → write → index update) runs **synchronously**; no server tasks/backpressure here. (Implied by one-shot CLI script flow.)&#x20;

## 9) Persistence & Data Model

* **Bundle layout:** `Manifest.toml`, `payload.bin`, optional `payload.bin.br` / `payload.bin.zst`.&#x20;
* **Addressing:** canonical `b3:<hex>.<tld>`; ETags/URLs expect the **BLAKE3** root. &#x20;
* **Index DB:** use the **same sled path** as services to avoid “phantom 404s”.&#x20;

## 10) Errors & Security

* **Common failure:** **sled lock** if `svc-index` is running (DB opened by both). Recommendation: **pack first**, or make `tldctl` talk to the daemon. &#x20;
* **Auth/TLS:** N/A (local tool). Capabilities and TLS live at Gateway/Omnigate/services.&#x20;
* **PQ-readiness:** not applicable to this local CLI; PQ planning applies to OAP/2 and service planes.&#x20;

## 11) Performance Notes

* **Hot path:** hashing + (optional) compression + disk I/O. Precompression is a **feature** (brotli/zstd artifacts) to optimize downstream delivery.&#x20;
* **Operational tip:** avoid concurrent pack + running `svc-index` on the same sled DB (lock contention).&#x20;

## 12) Tests

* **Direct tests:** not documented.
* **Used in system tests:** guidance explicitly says **“reuse the packing routine from `tldctl`”** to build fixtures in Gateway tests.&#x20;

## 13) Improvement Opportunities

* **Daemon mode / RPC to Index:** add `--use-daemon` or default to **UDS** calls to `svc-index` to eliminate sled locks.&#x20;
* **API polish:** expose a stable `pack()` lib API (already implied by reuse in tests) and document manifest schema.&#x20;
* **CLI unification:** consider merging with or aligning to `ronctl` to reduce CLI surface duplication (single “control” tool).&#x20;
* **DX/Docs:** add a `tldctl(1)` manpage; show env+args + examples from Quick-Start.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Quick-start and smoke docs updated to use `tldctl pack`; bundles now include `.br`/`.zst` alongside `Manifest.toml`/`payload.bin`. &#x20;
* **2025-09-05** — Known-issue documented: sled lock when `svc-index` is running during pack.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.0 — Behavior and args are clear from Quick-Start; manifest schema and lib API need a short spec.&#x20;
* **Test coverage:** 2.0 — Encouraged for reuse in tests but no explicit unit tests referenced.&#x20;
* **Observability:** 1.5 — CLI logs only; no metrics/health. (N/A for a lib tool.)
* **Config hygiene:** 3.0 — Consistent **`RON_INDEX_DB`** and `--store-root` usage is documented; needs daemon fallback to avoid locks. &#x20;
* **Security posture:** 2.5 — Local-only; no auth/TLS concerns; relies on service-plane security when published.&#x20;
* **Performance confidence:** 2.5 — Straightforward I/O + compression; no pathologies noted beyond DB lock contention.&#x20;
* **Coupling (lower is better):** 3.0 — Medium today due to **direct sled** coupling to `svc-index`; can be loosened via UDS daemon option.&#x20;

---



---

# transport

---

crate: transport
path: crates/transport
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Async TCP/TLS listener/dialer layer that enforces connection limits and per-socket timeouts, integrates with kernel health/metrics, and hands framed bytes to higher layers without embedding app semantics. &#x20;

## 2) Primary Responsibilities

* Bind and serve a socket (optionally TLS) with **max connections** and **idle/read/write timeouts** as configured. &#x20;
* Provide a clean async accept loop that spawns per-connection tasks and exposes the bound address/handle to the caller. &#x20;
* Surface health/metrics for open conns, bytes, errors, and backpressure drops via the project’s observability conventions.&#x20;

## 3) Non-Goals

* No routing, quotas, or protocol parsing (OAP/1 lives above; gateway/omnigate do edge policy).&#x20;
* No persistence or chunk storage (overlay/storage own that).&#x20;
* No app/business logic inside transport (kernel stays tiny: **transport + supervision + metrics + bus only**).&#x20;

## 4) Public API Surface

* **Re-exports:** Tokio primitives and `tokio_rustls::rustls` TLS config types are accepted but not re-exported (see TLS note below).&#x20;
* **Key types / functions (expected, per project invariants):**

  * `TransportConfig { addr, name, max_conns, read_timeout, write_timeout, idle_timeout }` (maps 1:1 to `[[transport]]` TOML in templates).&#x20;
  * `spawn_transport(cfg, metrics, health, bus, tls: Option<tokio_rustls::rustls::ServerConfig>) -> Result<(JoinHandle<()>, SocketAddr)>`. TLS type choice is a project invariant.&#x20;
* **Events / hooks:** On fatal listener errors publish restart/crash reasons to the Bus; reject on overload per global policy. &#x20;

## 5) Dependencies & Coupling

* **Internal crates → why, stability, replaceable?**

  * `ron-kernel` (health/metrics/bus) — reports readiness and emits `ServiceCrashed{reason}`; coupling *loose*; replaceable *no* (core). &#x20;
* **External crates (top 5; pins/features)**

  * `tokio` (async runtime), `tokio-rustls` (TLS), `bytes` (zero-copy), `tracing` (logs), `prometheus` (metrics). All mainstream/maintained → moderate risk.&#x20;
* **Runtime services:** Network sockets (TCP), optional TLS keypair/cert, optional Tor SOCKS if the embedding app chooses to route via Tor.&#x20;

## 6) Config & Feature Flags

* **Config struct:** mirrors repo templates—`max_conns`, `idle_timeout_ms`, `read_timeout_ms`, `write_timeout_ms`.&#x20;
* **TLS inputs:** PEM files (prod templates show `tls_cert_file`, `tls_key_file`) mapped into `rustls::ServerConfig`.&#x20;
* **Defaults we ship with (baseline):** handshake **2s**, read idle **15s**, write **10s**.&#x20;
* **Features:** none required for core; Tor/arti and QUIC are future/optional at higher layers.&#x20;

## 7) Observability

* **Metrics:** requests/4xx/5xx, latency histograms, queue depths, restarts, bytes in/out, **open conns**, backpressure drops.&#x20;
* **Tracing:** correlation IDs propagated across RPC boundaries.&#x20;
* **Readiness/health:** tied to accept loop liveliness and connection budget.&#x20;

## 8) Concurrency Model

* Tokio multi-threaded runtime; acceptor task + per-connection tasks; **semaphores** for dials/inbound; deadlines on awaits; cancellation on parent drop; bounded queues to avoid collapse.&#x20;
* Global concurrency cap via Tower-style middleware when embedded in HTTP stacks.&#x20;

## 9) Persistence & Data Model

* None (in-memory counters only). Transport is stateless by design.&#x20;

## 10) Errors & Security

* **Error taxonomy:** Listener bind/accept errors escalate through supervision (backoff + restart intensity caps). Overload returns **429/503** at edges using higher-level components. &#x20;
* **TLS:** `tokio_rustls::rustls::ServerConfig` is the required type; mTLS/Noise for non-public planes is handled by services that build atop transport. &#x20;
* **Secrets:** no secrets in logs; rotation and amnesia mode belong to upper layers.&#x20;
* **PQ-readiness:** transport stays stable for OAP/1; PQ changes are planned in **OAP/2** (feature-gated) above transport.&#x20;

## 11) Performance Notes

* **Hot path:** accept → TLS handshake (if any) → framed I/O → shutdown; zero-copy (`bytes::Bytes`) on hot paths.&#x20;
* **Targets:** internal API **p95 < 40 ms**, **p99 < 120 ms** under nominal budgets.&#x20;
* **Budgets:** inbound queue ≈ **\~1k msgs/listener**; tune `max_conns` and timeouts to maintain SLOs.&#x20;

## 12) Tests

* **Recommended patterns:** hermetic duplex I/O (`tokio::io::duplex`) for transport round-trip; chaos tests (slow-loris/packet loss) in harness; property tests for framed boundaries. &#x20;
* **Interop suite:** transport must pass the matrix that runs over **TCP+TLS** & **Tor**, comparing bytes/timings.&#x20;

## 13) Improvement Opportunities

* **QUIC (exploratory):** optional feature for low-latency paths once OAP/1 parity is proven. (Keep transport pluggable.)&#x20;
* **Tor/arti adapter:** align with private onion runtime (`svc-oniond`) when promoted; expose a SOCKS dialer hook cleanly.&#x20;
* **Metrics hardening:** add per-listener inflight gauges and handshake failure counters mapped to reasons.&#x20;
* **Config polish:** publish a tiny schema/doc for `TransportConfig` with the shipped defaults.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Project invariants pinned: **rustls TLS type**, health/ready/metrics required across services; timeouts/defaults documented. &#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.5 — Responsibilities and config shape are clear; publish the exact `TransportConfig`/`spawn_transport` signature.&#x20;
* **Test coverage:** 2.5 — Patterns and harness guidance exist; formalized transport tests (duplex/chaos) should land. &#x20;
* **Observability:** 3.0 — Metrics slate defined; ensure gauges/counters are wired before Silver.&#x20;
* **Config hygiene:** 3.5 — Templates show timeouts/TLS; codify schema + defaults in the crate. &#x20;
* **Security posture:** 3.0 — Correct TLS stack and secret-handling posture via upper layers; PQ defers to OAP/2. &#x20;
* **Performance confidence:** 3.0 — Budgets and SLOs are explicit; needs sustained soak + handshake error histograms. &#x20;
* **Coupling (lower is better):** 2.5 — Thin integration to kernel (bus/metrics/health); otherwise protocol-agnostic.&#x20;


