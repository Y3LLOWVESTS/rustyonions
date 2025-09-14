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
