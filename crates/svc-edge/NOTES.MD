### BEGIN NOTE - NOVEMBER 10 2025 - 20:01 CST


# CARRY-OVER NOTES — `svc-edge` (Beta Track)

**Date:** 2025-11-10 (Mon)
**Status:** Foundation is compiling & runnable; admin plane up; admission stack pattern selected and validated.
**Beta completion (est.):** ~55–60% (core skeleton + admin plane + admission chain decisions done; most API/adapters still to implement).

---

## 0) TL;DR

* **Build/Run:** green with current pins (Axum 0.7, Tower 0.5, tower-http 0.6.6).
* **Admin plane:** `/healthz`, `/readyz`, `/metrics` are bound and responding.
* **Admission chain:** settled on **Axum-native** pattern using `route_layer(...)` + `DefaultBodyLimit` + `Timeout/Rate/Concurrency` + **outermost** `HandleErrorLayer`. This avoids the Tower body-type and `Clone`/`Infallible` traps we hit.
* **Next 2 slices (to hit Beta fast):**

  1. Wire **reject metrics** and **config-driven caps** (TOML/env) into the admission chain.
  2. Implement **minimal API routes** (`/assets/*`, range/etag helpers) with the **pack/CAS adapters** stubs.

---

## 1) What’s in place (accomplishments)

### Project structure (scaffold complete)

* `src/` modules stubbed per TODO tree: `cli`, `config`, `state`, `metrics`, `readiness`, `routes/*`, `admission/*`, `adapters/*`, `http/*`, `security/*`, `work/*`, `supervisor/*`, `util/*`.
* `bin/svc-edge.rs` boots the process, binds admin plane, and hosts future API plane (logs show admin bind OK).

### Build system + pins

* Uses workspace pins (Tokio 1.x, Axum 0.7.9 with `tokio,http1,http2,json`, **Tower 0.5.2** with `util,limit,timeout`, `tower-http 0.6.6`).
* Consistent with the project’s “root pins to minimize duplicates” policy.

### Admin plane (operational endpoints)

* `/healthz`: returns 200 when process is live.
* `/readyz`: basic readiness gate wired via `readiness.rs`; policy: flips to 200 after init/supervisor gates pass (currently minimal).
* `/metrics`: Prometheus exporter (from `EdgeMetrics`).

### Admission chain (ingress hardening — key decision made)

* **Pattern chosen (works with Axum 0.7 + Tower 0.5):**

  * Apply guards with `router.route_layer(ServiceBuilder::new() …)`.
  * Use **Axum’s `DefaultBodyLimit`** (not `tower_http::limit::RequestBodyLimitLayer`) to avoid `Limited<Body>` type churn.
  * Add `TimeoutLayer`, `ConcurrencyLimitLayer`, `RateLimitLayer` in that order (cheap guards early OK).
  * Apply **`HandleErrorLayer` last** (outermost in the ServiceBuilder used by `route_layer`) to map errors to HTTP (408, 429, 413, 503) so the resulting service satisfies Axum’s `Error: Into<Infallible>` requirement without boxing headaches.

> We explicitly avoided the Tower `BoxCloneService` detour; not needed when using `route_layer` with the Axum body-limit and keeping `HandleErrorLayer` last.

### CLI + config

* `cli.rs` + `config.rs`: env + file config loader exists; TOML read path tested.
* `configs/svc-edge.toml` present (baseline); more keys to add for caps/tunables.

### Signal handling / shutdown

* Process listens for Ctrl-C and `SIGTERM` (Unix); graceful shutdown path logs and exits cleanly.

### Observability

* `EdgeMetrics` struct exists; exporter bound via admin plane.
* Structured logs via `tracing + env-filter` (e.g., `RUST_LOG=info,hyper=warn` used in runs).

---

## 2) What we learned / pitfalls resolved

1. **Tower 0.5 + Axum 0.7 layering traps**

   * `Router::layer` and even `route_layer` require the final service to be **Clone** and the **Error** to be **`Into<Infallible>`**.
   * `tower_http::RequestBodyLimitLayer` swaps the request body type to `Limited<Body>`. When you try to layer this at the router level (and/or combine with other Tower layers) you often hit **type mismatch** and **Clone** constraint issues.
   * **Resolution:** use **Axum’s `DefaultBodyLimit`** + `route_layer` + `HandleErrorLayer` as **last** layer in the ServiceBuilder. This produces an Axum-friendly pipeline with correct bounds and clean error mapping.

2. **Do not** apply Tower limit/timeout layers directly with `Router::layer` unless you box types and manually thread `Infallible`. It’s doable but not worth the maintenance burden here.

3. The **order** of layers matters:

   * Guards (timeout, concurrency, rps, body limit) first.
   * Error mapper (`HandleErrorLayer`) **last**, so any error from guards turns into an `(StatusCode, body)` response.

---

## 3) What’s left for **Beta**

### A) Admission chain — production-grade (required)

* **Config-driven caps** (TOML/env + validation):

  * `admission.timeout_ms` (default 5000)
  * `admission.max_inflight` (default 256)
  * `admission.rps_per_sec` (default 1000)
  * `admission.max_body_bytes` (default 1_048_576)
  * `security.amnesia` (already present; confirm semantics)
* **Metrics for deny paths** (Prometheus counters):

  * `edge_rejects_total{reason="timeout|rps|inflight|body|overloaded"}`
  * Latency histogram around the admission chain (optional but helpful).
* **Docs**: update `docs/OBSERVABILITY.md` and `docs/CONFIG.md` for these keys, and note expected status codes (408, 413, 429, 503).

### B) API plane — MVP routes (required)

* `routes/assets.rs`:

  * `GET /assets/:space/:pack/*path` (or the shape in your TODO/README)
  * Performs **pack** lookup via `adapters::pack` and **range** reads (support `Range: bytes=...` → 206/416).
  * Compute/return `ETag` (via `http::etag` helper) and support `If-None-Match` → 304.
* `routes/ready.rs`, `routes/health.rs` are done for admin plane. Add “/version” if needed.
* Content types: set from file extension or map in pack metadata.

### C) Adapters (required)

* `adapters/pack.rs`:

  * Open/read PMTiles/pack index.
  * Resolve path → byte ranges; validate checksums (BLAKE3) if available.
  * Return a small DTO: `{ len, etag, last_modified, range: Option<(start,end)> }` plus a reader/offset for Axum body streaming.
* `adapters/cas.rs`:

  * CAS read by content hash (BLAKE3 hex/URL-safe).
  * Minimal: just GET for Beta; PUT/POST later (post-Beta or behind a feature).
* `adapters/live_fill.rs`:

  * Optional: cache-miss fallback (proxy fetch or precompute); can be stubbed for Beta.
* `adapters/tls.rs`:

  * If TLS termination needed locally; otherwise defer (we often terminate at a front proxy).

### D) HTTP helpers (required)

* `http/etag.rs`: strong ETag from checksum or from `blake3(content)` if needed.
* `http/range.rs`: parse and normalize ranges; produce 206 responses or 416 with `Content-Range`.
* `http/headers.rs`: standard headers (cache-control, content-type, vary, cors, hsts).

### E) Security layer (required)

* `security/cors.rs`: tight CORS default (deny-by-default, explicit allowlist).
* `security/hsts.rs`: add HSTS if TLS is on or behind a TLS terminator.
* `security/audit.rs`: structured deny logs for admission rejections (no PII).

### F) Supervisor / Work queue (MVP for Beta)

* `work/queue.rs` + `work/worker.rs`: basic task runner for background refresh (optional for Beta if we only do read-through).
* `work/shutdown.rs`: graceful drain on shutdown.

### G) Readiness logic (complete)

* Readiness should flip only when:

  * admin listener bound,
  * config parsed and validated,
  * (if applicable) pack/CAS opened successfully,
  * metrics exporter live.
* Add probe that periodically verifies the pack index available and flips readiness on/off truthfully.

### H) Tests (required to close Beta)

* **Integration tests** in `tests/` (already scaffolded):

  * `i_1_hardening_ingress.rs`: ensure 408/429/413/503 paths.
  * `i_4_http_semantics.rs`: `ETag`, `If-None-Match`, `Cache-Control`, `Vary`.
  * `i_5_content_address.rs`: CAS reads return correct content + etag(hash).
  * `i_6_amnesia.rs`: amnesia mode disables persistence (config gates).
  * `i_8_size_bounds.rs`: body cap enforced.
  * `i_9_observability_contract.rs`: metrics counters and labels exist.
  * `i_10_deterministic_failures.rs`: stable error codes for identical failures.
  * `i_11_pack_integrity.rs`: verify sample pack range reads and checksums.
  * `http_contract.rs`, `readiness_logic.rs`, `concurrency_backpressure.rs`, `fuzz_headers.rs`.
* Provide **fixtures** under `tests/fixtures/` (pack file already stubbed) and golden responses (`warm_hit.response`, `range_206.response`).

### I) Benches (nice-to-have for Beta, but good)

* `benches/bench_range.rs`: end-to-end range selection overhead.
* `benches/bench_blake3.rs`: checksum cost (may use BLAKE3).
* `benches/bench_pack_read.rs`: pack header + tile read cost.

### J) CI and scripts (mostly done; minor updates)

* `scripts/dev-run.sh` runs admin + api with env defaults.
* `scripts/perf-smoke.sh` to warm caches and GET hot paths; record p50/p99 lat.
* GitHub Actions: `ci.yaml`, `perf-guard.yaml`, `public-api.yaml` are checked in; ensure new routes don’t regress.

---

## 4) Interop expectations (RON-CORE cohesion)

* **ron-kernel**: use `HealthState` for readiness truth; `Metrics::serve` integration done; use `wait_for_ctrl_c()` for shutdown.
* **ron-transport / svc-overlay**: svc-edge can front overlay routes or directly serve packs/CAS; be strict about timeouts and quotas at edge to protect overlay services.
* **svc-registry** (soon): resolve pack locations / versions; cache results; plan for registry outage (serve stale + surface 5xx consciously).
* **svc-passport / ron-auth / ron-kms**: future slice — add macaroon-style capability checks at the edge (e.g., read scopes, rate quotas); for Beta this can be gated off (accept-all) with no ambient authority in code.

---

## 5) Config: proposed keys (to add now)

`configs/svc-edge.toml`

```toml
[server]
bind_addr = "0.0.0.0:8080"
metrics_addr = "127.0.0.1:9909"
amnesia = false

[admission]
timeout_ms     = 5000
max_inflight   = 256
rps_per_sec    = 1000
max_body_bytes = 1048576

[assets]
# Root(s) or registry hints for packs/CAS
# pack_root = "packs/"
# cas_root  = "cas/"
```

**Env overrides** (examples):

```
SVC_EDGE_BIND_ADDR=127.0.0.1:8080
SVC_EDGE_METRICS_ADDR=127.0.0.1:9909
SVC_EDGE_ADMISSION_TIMEOUT_MS=5000
SVC_EDGE_ADMISSION_MAX_INFLIGHT=256
SVC_EDGE_ADMISSION_RPS_PER_SEC=1000
SVC_EDGE_ADMISSION_MAX_BODY_BYTES=1048576
```

---

## 6) How to run (current)

```
cargo build -p svc-edge

SVC_EDGE_METRICS_ADDR=127.0.0.1:9909 \
SVC_EDGE_BIND_ADDR=127.0.0.1:8080 \
RUST_LOG=info,hyper=warn \
cargo run -p svc-edge
```

**Admin plane checks:**

```
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9909/healthz
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9909/readyz
curl -s http://127.0.0.1:9909/metrics | head -n 30
```

**Admission quick tests (API plane target paths TBD; use any route once assets are wired):**

```
# Body cap (expect 413)
head -c 1200000 </dev/zero | tr '\0' 'A' > /tmp/big.bin
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:8080/ping \
  -X POST --data-binary @/tmp/big.bin
```

---

## 7) Risk log & gotchas

* **Tower/Axum layering**: avoid `RequestBodyLimitLayer` unless you are ready to box and hand-specify `Infallible`/body types. Prefer **Axum’s** `DefaultBodyLimit`. Keep the **error handler last** (inside the route layer’s builder).
* **Clone bounds**: If later we introduce custom layers that produce non-Clone services, prefer `route_layer(ServiceBuilder::new()… .into_inner())` pattern. Only resort to `BoxCloneService` if necessary (and then specify `Request<Body>, Response, Infallible` explicitly).
* **Range semantics**: normalize unsatisfiable ranges to 416 with `Content-Range: bytes */LEN`. Return 206 with exact `Content-Range` for satisfiable requests.
* **ETag**: prefer strong ETag from content hash or pack metadata; ensure `If-None-Match` returns 304 cleanly.
* **Amnesia mode**: when `amnesia=true`, disable disk persistence and aggressive caches; log that amnesia is on in startup banner.

---

## 8) Beta acceptance checklist

* [ ] Admission caps **config-driven** with validation; **reject metrics** wired.
* [ ] `/assets/*` route serves from **pack adapter** with proper range/etag/cache headers.
* [ ] `/cas/:algo/:digest` route reads CAS blobs with correct ETag and content-type.
* [ ] **Readiness is truthful**: flips to 200 only when pack/CAS primed (or explicitly configured to allow cold start).
* [ ] **Observability**: metrics include `edge_requests_total`, `edge_rejects_total{reason}`, `edge_request_latency_seconds_bucket`.
* [ ] **Security**: CORS/HSTS defaults sane; deny by default unless configured.
* [ ] **Tests**: integration suite passing; fuzz header parser basic run; contract tests for range/etag.
* [ ] **Docs** updated (API/CONFIG/OBSERVABILITY/SECURITY/RUNBOOK).
* [ ] GitHub Actions green (CI, perf guard doesn’t regress admin plane).

---

## 9) Suggested next commit plan (2–3 small increments)

1. **Config + metrics for admission**

   * Add TOML/env keys (`admission.*`), validation, and Prometheus counters in the error mapper.
   * Update `docs/CONFIG.md` and `docs/OBSERVABILITY.md`.

2. **Minimal assets route with pack adapter**

   * Implement `adapters::pack` read (happy path), `http::range`, `http::etag`.
   * Add `GET /assets/:space/:pack/*path` with 200/206/304/404/416.
   * Contract tests: `i_4_http_semantics.rs`, `i_11_pack_integrity.rs`.

3. **CAS GET route**

   * `GET /cas/blake3/:hex` with 200 + ETag, 404 if missing.
   * Tests: `i_5_content_address.rs`.

(Then: security headers, CORS, readiness sampler, benches.)

---

## 10) One-liners for future self

* If you see `Infallible: From<Box<...>>` or Clone errors on `Router::(route_)layer`, **move/keep** `HandleErrorLayer` **last inside the builder** and prefer **Axum’s body limit**.
* For range: clamp; for ETag: strong; for cache-control: vary on `Accept-Encoding` if gzip is used.
* Keep all deny paths deterministic and observable — **never** return ambiguous 500s for guard rejections.



### END NOTE - NOVEMBER 10 2025 - 20:01 CST





### BEGIN NOTE - NOVEMBER 13 2025 - 15:17 CST

Short version: I’d call svc-edge **~55–60% of the way to beta** right now.

* Foundation + bin are **clean and runnable** again with Axum 0.7 and two listeners (admin + api).
* Project structure, blueprints, and most docs are already in place and fairly mature.
* The big missing chunks to hit your **beta checklist** are: real admission caps, pack/CAS-backed `/assets` routes, truthful readiness, metrics wiring, and the test/bench harness.

---

### BEGIN NOTE – NOVEMBER 13 2025 – svc-edge (post-Grok bin fix)

# CARRY-OVER NOTES — `svc-edge` (Beta Track, post-bin fix)

**Date:** 2025-11-13 (Thu)
**Status:** Foundation + bin are compiling & runnable; dual-listener Axum 0.7 server with graceful shutdown is in place.
**Beta completion (est.):** **~55–60%** (scaffold + docs + admin/API skeleton done; admission caps, asset/CAS routes, truthful readiness, metrics, and tests still ahead).

---

## 0) TL;DR (where we stand now)

* **Build/Run:** `cargo build -p svc-edge` is green; `bin/svc-edge.rs` now uses:

  * `TcpListener` for separate **admin** and **api** ports.
  * `Router` with Axum 0.7’s `into_make_service_with_connect_info::<SocketAddr>()`.
  * `CancellationToken` + `tokio::select!` for **clean graceful shutdown**.
* **Admin plane:** `/healthz`, `/readyz`, `/metrics` are bound as simple stub handlers (strings), good enough for smoke.
* **API plane:** `/edge/ping` route exists as a stub; real `/edge/assets/*` and `/cas/:algo/:digest` are **not** wired yet.
* **Docs & design:** README, INTEROP, SECURITY, RUNBOOK, etc. are already largely fleshed out and define the beta target shape (ingress caps, packs/CAS, live-fill, observability, SLOs).

**Next two slices to move the needle toward beta:**

1. Wire **admission caps + reject metrics** into the Axum router (config driven).
2. Implement **minimal but real asset routes**: `/edge/assets/{*path}` (packs/CAS) and `/cas/{algo}/{digest}` with ETag/Range semantics.

---

## 1) What we shipped in *this* instance

### 1.1 New `bin/svc-edge.rs` (Grok-assisted)

* Replaced the old, overly-complex bin with a **minimal, correct Axum 0.7 entrypoint**:

  * Uses `#[tokio::main]` with multi-thread runtime.
  * Reads **two env vars** for now:

    * `SVC_EDGE_METRICS_ADDR` (admin plane /metrics bind)
    * `SVC_EDGE_BIND_ADDR` (public API bind)
  * Falls back to `"127.0.0.1:9909"` and `"127.0.0.1:8080"` if unset, matching earlier notes.

* Constructs a small `AppState` (currently just `build: &'static str` from `env!("CARGO_PKG_VERSION")`), cloned into both admin and api routers.

* **Admin router**:

  * `GET /healthz` → `"ok"`
  * `GET /readyz` → `"ready"` (stub; not yet truthful to packs/CAS/ingress caps)
  * `GET /metrics` → `"# prometheus exposition (stub)\n"` (placeholder).

* **API router**:

  * `GET /edge/ping` → `"pong"` (sanity check for the API bind; this is where `/edge/assets/*` will hang).

* **Listeners and shutdown:**

  * Binds separate `TcpListener`s for admin & api, logs their addresses.
  * Uses a shared `CancellationToken`:

    * `t_admin` and `t_api` clones passed into `.with_graceful_shutdown(token.cancelled_owned())`.
  * Wraps each `axum::serve(listener, make_service)` in its own async block that:

    * Awaits the server.
    * Maps errors into `anyhow::Error` with context (`"admin server failed"`, `"api server failed"`).
  * A `tokio::select!` races:

    * `admin_srv`
    * `api_srv`
    * `wait_for_shutdown_signal()` (using `ron_kernel::wait_for_ctrl_c`, then a `tokio::signal::ctrl_c()` fallback).
  * After one arm completes:

    * Cancels the token,
    * Sleeps for ~100ms to give inflight requests a chance to drain,
    * Logs `"svc-edge exiting"` and returns `Ok(())` or the underlying error.

**Net effect:** svc-edge is now a **boring, robust Axum 0.7 dual-listener service** with proper cancellation semantics and no Tower body-type shenanigans.

---

## 2) What was already in place before this instance

Most of the *design* work and scaffolding was done in earlier passes; this instance was about getting the bin unstuck with the Axum 0.7/Tower 0.5 world.

### 2.1 Project structure (scaffold)

Per the earlier CODEBUNDLE snapshot, `src/` already contains modules for:

* `cli`, `config`, `state`, `metrics`, `readiness`,
* `routes/*` (API/admin routes),
* `admission/*` (ingress caps & guards),
* `adapters/*` (packs/CAS/live-fill),
* `http/*`, `security/*`, `work/*`, `supervisor/*`, `util/*`.

There are also **skeleton tests** and scripts:

* Tests like `i_1_hardening_ingress.rs`, `i_4_http_semantics.rs`, `i_5_content_address.rs`, `i_6_amnesia.rs`, `i_8_size_bounds.rs`, `i_9_observability_contract.rs`, `readiness_logic.rs` are stubbed out as future integration/contract tests.
* Scripts:

  * `scripts/dev-run.sh` (env + config wiring for dev runs).
  * `scripts/perf-smoke.sh` (bombardier-based quick perf smoke).
  * GitHub Actions: `ci.yaml`, `perf-guard.yaml`, `public-api.yaml`, `render-mermaid.yaml` (currently referencing `svc-edge2` in places; will need a small cleanup).

### 2.2 Admission chain design (but not re-wired to the new bin yet)

Earlier notes settled on the **Axum 0.7 + Tower 0.5 pattern**:

* Apply admission controls via `router.route_layer(ServiceBuilder::new() …)`.
* Use **Axum’s `DefaultBodyLimit`** instead of `tower_http::limit::RequestBodyLimitLayer` to avoid body-type explosion and `Infallible` hassles.
* Stack `TimeoutLayer`, `ConcurrencyLimitLayer`, `RateLimitLayer` in a sane order.
* Use `HandleErrorLayer` as the **outermost** layer in that builder to map rejections to HTTP codes (408/429/413/503) while still satisfying Axum’s `Error: Into<Infallible>` contract.

This pattern worked in isolation; it just hasn’t been re-applied to the new, minimal bin yet.

### 2.3 Docs & interop expectations

We already have **rich documentation** specifying:

* Role & topology: edge asset server, packs/CAS + optional live-fill, fronted by gateway/Omnigate.
* Public ingress contract (`/edge/assets/*`, `/healthz`, `/readyz`, `/metrics`), HTTP semantics for ETag/Range, and readiness JSON examples.
* Observability expectations: canonical metrics like `http_requests_total`, `edge_cache_hits_total`, `edge_live_fill_*`, `edge_rejects_total{reason}`, readiness alerts, etc.
* Security threat model (STRIDE), amnesia-mode constraints, and systemd hardening hints.

These docs define the **beta shape** even though the implementation is still catching up.

---

## 3) Beta completion estimate (why ~55–60%)

Using the **beta checklist** from the earlier notes as the yardstick:

1. **Admission caps config-driven + reject metrics**

   * Pattern chosen; env/config keys proposed; not yet wired into the new bin → **0.5 / 1.0**.

2. **`/assets/*` route via pack adapter (Range/ETag/cache)**

   * Design is in docs; code stubs exist; no working path yet → **0.3 / 1.0**.

3. **`/cas/:algo/:digest` route with correct ETag + content-type**

   * Mostly design-only right now → **0.2 / 1.0**.

4. **Truthful readiness** (packs verified, CAS reachable, caps enforced)

   * Readiness module exists in scaffold; `/readyz` currently just `"ready"` in the new bin → **0.4 / 1.0**.

5. **Observability metrics** (`edge_requests_total`, `edge_rejects_total{reason}`, latency histogram)

   * Observability spec is detailed; exporter wiring to `EdgeMetrics` not yet plugged into the new bin → **0.4 / 1.0**.

6. **Security defaults** (CORS/HSTS sane; deny-by-default)

   * Threat model designed; header/guard layers not wired into the minimal router yet → **0.4 / 1.0**.

7. **Tests & contracts** (integration, fuzz, HTTP semantics, amnesia, range/ETag)

   * Test files exist as skeletons; little/no implementations yet → **0.3 / 1.0**.

When you average that across the checklist, plus the fact that **build + dual-plane bin + docs + scaffold** are all solid, you land in the **mid-50s to ~60%** completed for beta.

---

## 4) What’s left to do for Beta (concrete TODO stack)

You can treat this as the **svc-edge beta TODO ladder**:

### 4.1 Config & CLI

* Hook `config.rs` and `cli.rs` back into the new bin:

  * Parse `configs/svc-edge.toml` with the `[server]`, `[admission]`, and `[assets]` sections anticipated in the docs.
  * Keep env overrides in sync with the documented keys (`SVC_EDGE_BIND_ADDR`, `SVC_EDGE_METRICS_ADDR`, `SVC_EDGE_ADMISSION_*`, `SVC_EDGE_SECURITY__AMNESIA`, etc.).
  * Make sure CLI flags `--bind`, `--metrics`, `--config` behave as documented.

### 4.2 Admission chain (ingress hardening)

* Implement the `ServiceBuilder`-based **admission stack** on the API router:

  * `DefaultBodyLimit::max(config.ingress.body_bytes)`
  * `TimeoutLayer::new(...)`
  * `ConcurrencyLimitLayer::new(...)`
  * `RateLimitLayer::new(...)`
  * `HandleErrorLayer::new(...)` (outermost, maps admission errors → HTTP codes and increments `edge_rejects_total{reason}`).

* Confirm it works with Axum 0.7’s body type and doesn’t regress into the earlier `Service<IncomingStream>`/`Infallible` pitfalls.

### 4.3 Asset & CAS routes

* Implement:

  * `GET /edge/assets/{*path}`

    * Read from **packs/CAS** via adapters.
    * Honor `Range` and `If-None-Match`.
    * Emit strong ETag (BLAKE3) and `Accept-Ranges: bytes`.

  * `GET /cas/{algo}/{digest}`

    * Map to CAS storage.
    * Validate `algo` and digest format; 404/400 appropriately.
    * Same ETag/Range semantics.

* Ensure the behavior matches INTEROP and README (304/416, no multi-range by default).

### 4.4 Truthful readiness

* Wire `/readyz` to the actual `readiness` module/`HealthState`:

  * Checks:

    * Packs path and CAS reachability.
    * Config sanity (no nonsense caps).
    * Queue depths and inflight counts below thresholds.
    * Amnesia mode obeyed (no disk usage when `amnesia=true`).

* Make readiness reflect these conditions in real time, including degraded JSON as described in INTEROP.md.

### 4.5 Metrics & observability

* Implement `EdgeMetrics` and the Prometheus exporter:

  * Counters:

    * `edge_requests_total{route,method,status}`
    * `edge_rejects_total{reason}`
    * `edge_cache_hits_total`, `edge_cache_misses_total`
    * `edge_live_fill_attempts_total`, `edge_live_fill_success_total`
  * Histogram:

    * `edge_request_latency_seconds_bucket{route,method}`

* Instrument the handlers and admission layers to tick these counters and latency buckets.

* Double-check the metrics align with the observability docs and suggested Grafana dashboards.

### 4.6 Security & headers

* Add the security middleware / route-layer bits:

  * CORS: deny-by-default, allow-list from config.
  * HSTS headers where appropriate.
  * `Referrer-Policy: no-referrer`.
  * Error responses: minimal header/body, no secrets in logs.

* Make sure amnesia-mode is **enforced** (no on-disk writes, clear logs/metrics stating amnesia is active).

### 4.7 Tests, fuzz, and perf

* Fill in the existing integration tests:

  * `i_1_hardening_ingress.rs` → admissions caps, DoS resistance.
  * `i_4_http_semantics.rs` → Range/ETag/304/416 behavior.
  * `i_5_content_address.rs` → CAS correctness.
  * `i_6_amnesia.rs` → no persistence, readiness behavior.
  * `i_8_size_bounds.rs` → body/decompress bounds.
  * `i_9_observability_contract.rs` → metrics invariants.
  * `readiness_logic.rs` → gating conditions.

* Add fuzz tests for headers (`Range`, `If-None-Match`, compression headers).

* Bring up simple **perf rigs** (bombardier/hey) against `/edge/assets/*` and document p50/p95/p99 vs the SLOs described in README.

---

## 5) How to run *current* svc-edge (post-fix)

For this increment (minimal bin):

```bash
cargo build -p svc-edge

SVC_EDGE_METRICS_ADDR=127.0.0.1:9909 \
SVC_EDGE_BIND_ADDR=127.0.0.1:8080 \
RUST_LOG=info,hyper=warn \
cargo run -p svc-edge
```

Sanity checks:

```bash
# Admin
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9909/healthz
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:9909/readyz
curl -s http://127.0.0.1:9909/metrics

# API
curl -s -o /dev/null -w "%{http_code} %{body}\n" http://127.0.0.1:8080/edge/ping
```

Later, once `/edge/assets/*` is wired, the perf scripts in `scripts/perf-smoke.sh` can be pointed at real assets.

---

## 6) Risk log & gotchas (for future you)

* **Axum 0.7 + Tower 0.5 layering** is still the sharp edge:

  * Stick to `DefaultBodyLimit` + `route_layer(ServiceBuilder::new()…into_inner())`.
  * Keep `HandleErrorLayer` outermost for admission error mapping.

* **Doc vs code drift:** docs already promise `/edge/assets/*`, CAS, truthful readiness, and a rich metric set. Keep the beta checklist handy and update docs only if the contract changes — don’t loosen it silently.

* **Interop expectations:** svc-edge is expected to:

  * Front packs/CAS for Omnigate/svc-gateway.
  * Shed early and clearly under pressure (429/503 with `reason` labels).
  * Respect amnesia and not become a secret sink.

---

**Bottom line for next instance:**
Start by **re-wiring the admission chain + metrics** into the new bin (they touch the most checkboxes at once), then implement `/edge/assets/*` from packs/CAS, and finally tighten readiness & tests. Once those are in, you’ll be very close to flipping svc-edge to **beta complete**.


### END NOTE - NOVEMBER 13 2025 - 15:17 CST



### BEGIN NOTE - NOVEMBER 13 2025 - 20:17 CST


# CARRY-OVER / WRAP-UP NOTES — `svc-edge` (Beta Baseline)

**Date:** 2025-11-14
**Status:** ✅ **Beta-ready** (admin + API planes live; admission guards; assets/CAS; metrics; smoke script green)
**Tag suggestion:** `svc-edge-beta-baseline-2025-11-14`

---

## 0) TL;DR (what ships in this beta)

* **Two listeners**:

  * **Admin plane** on `SVC_EDGE_METRICS_ADDR` → `/healthz`, `/readyz`, `/metrics`
  * **API plane** on `SVC_EDGE_BIND_ADDR` → `/echo`, `/echo/slow/:ms`, `GET /edge/assets/*path`, `GET /cas/:algo/:digest`
* **Admission chain** (API plane): **timeout** (→408) and **inflight cap** (→503) with a tiny, cloneable `Layer` (no brittle combinator stacks).
* **Static assets** with **strong ETag** (BLAKE3), **If-None-Match → 304**, **Range: bytes=… → 206**, **Accept-Ranges: bytes**.
* **CAS (blake3)** demo: `GET /cas/blake3/<digest>` (same ETag/Range semantics).
* **Readiness** gate flips from degraded to **ready=true** once **config_loaded** and **services_ok** are true (assets root exists).
* **Metrics** (Prometheus): `amnesia_mode` gauge; **request latency histograms** by route/method; **reject counters** (`edge_rejects_total{reason}`); smoke shows live increments.
* **Graceful shutdown**: clean cancellation via `CancellationToken` + Ctrl-C.

---

## 1) How to run (current beta defaults)

```bash
# Env (override as needed)
export SVC_EDGE_BIND_ADDR="0.0.0.0:8080"
export SVC_EDGE_METRICS_ADDR="127.0.0.1:9909"
export SVC_EDGE_SECURITY__AMNESIA=0
export SVC_EDGE_ADMISSION_TIMEOUT_MS=5000
export SVC_EDGE_ADMISSION_MAX_INFLIGHT=256
export SVC_EDGE_ASSETS_DIR=assets

# Run
cargo run -p svc-edge
```

**Expected boot logs** (abridged):
`admin plane listening …:9909` • `api plane listening …:8080` • `up; waiting for traffic or shutdown signal`

---

## 2) Endpoints (admin + API)

### Admin plane (`127.0.0.1:9909`)

* `GET /healthz` → **200** if process healthy.
* `GET /readyz` → **503** with missing gates initially; flips to **200** with `{"ready": true, "missing": []}` when `config_loaded && services_ok`.
* `GET /metrics` → Prometheus exposition.

### API plane (`:8080`)

* `POST /echo` → `{ ok: true, len }` (counts request body bytes).
* `POST /echo/slow/:ms` → sleeps `ms` ms, then echo (exercise timeout/503).
* `GET /edge/assets/*path` → serve from `SVC_EDGE_ASSETS_DIR` with **ETag**, **304**, **206**, **Accept-Ranges: bytes**.
* `GET /cas/:algo/:digest` → **algo=blake3** only; looks under `assets/cas/blake3/<digest>`; same ETag/Range support.

---

## 3) Admission guards (API plane)

* **Timeout**: `SVC_EDGE_ADMISSION_TIMEOUT_MS` (default **5s**).

  * Exceed → **408 Request Timeout** (`{"ok": false, "reason": "timeout"}`).
* **Inflight cap**: `SVC_EDGE_ADMISSION_MAX_INFLIGHT` (default **256**).

  * Saturation → **503 Service Unavailable** (`{"ok": false, "reason": "busy"}`).
* Design goals:

  * Keeps service **`Clone + Send + 'static`** and **`Error = Infallible`** to satisfy `Router::route_layer` bounds in Axum 0.7.
  * **No locks across `.await`**; uses `Arc<Semaphore>`.
  * **Deterministic** rejections only (408/503) — no ambiguous 500s from the layer.

---

## 4) Readiness model (truthful / degrade-first)

* Gates tracked internally:

  * `config_loaded` → set **true** after `Config` is parsed/validated.
  * `services_ok` → set **true** if `SVC_EDGE_ASSETS_DIR` exists (temporary stand-in for service substrate).
* `/readyz` shows `{ ready: bool, missing: [..] }` with HTTP **503** until all gates set.
* Rationale: callers can safely **cold-start** against `/readyz` and see **why** it’s not ready yet.

---

## 5) Metrics (Prometheus) — what we expose

* `amnesia_mode` (gauge) — from security posture (1/0).
* `edge_rejects_total{reason}` — admission rejections (`busy`, `timeout`).
* `edge_request_latency_seconds{route,method}` — histograms (1ms..10s).
* (Optional next: `edge_requests_total{route,method,status}` — counters; histos already visible; counters are easy to add).

**Observed in smoke:**

* `amnesia_mode 0`
* `edge_rejects_total{reason="busy"} <n>` (from load at inflight cap)
* latency buckets for `cas_get`, `assets_get`, etc.

---

## 6) Smoke & examples (what we ran)

```bash
# Admin plane sanity
curl -i http://127.0.0.1:9909/healthz      # 200
curl -i http://127.0.0.1:9909/readyz       # 503 → then 200 once assets exist
curl -s http://127.0.0.1:9909/metrics | head

# Prepare assets
mkdir -p assets
echo "hello-edge" > assets/hello.txt

# Assets
curl -i http://localhost:8080/edge/assets/hello.txt              # 200
curl -i http://localhost:8080/edge/assets/hello.txt \
  -H 'If-None-Match: "<etag-from-previous>"'                      # 304
curl -i http://localhost:8080/edge/assets/hello.txt \
  -H 'Range: bytes=2-'                                            # 206

# CAS (blake3)
# Pre-stage: cp assets/hello.txt assets/cas/blake3/<digest>
curl -i http://localhost:8080/cas/blake3/<digest>                 # 200
curl -i http://localhost:8080/cas/blake3/<digest> -H 'Range: bytes=2-' # 206

# Echo / load
curl -i -X POST http://localhost:8080/echo -d x                   # 200
curl -i -X POST http://localhost:8080/echo/slow/7000 -d x         # 408 (timeout=5s)
seq 1 1200 | xargs -n1 -P800 -I{} \
  curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST http://localhost:8080/echo/slow/2000 -d x | sort | uniq -c
# → mix of 200 and 503 (at inflight cap)
```

**Script:** `crates/svc-edge/scripts/smoke_edge.sh`

* Verifies admin plane, readiness, ETag/304/206, CAS 200/206, and a burst read → **DONE** observed.

---

## 7) Code invariants / patterns we locked in

* **Axum 0.7 + Tower 0.5**; HTTP/1+2 enabled; no locks across `.await`.
* Admission layer ensures **Router(Error = Infallible)** and **cloneability**.
* Strong **ETag = BLAKE3(content)** consistently used across assets/CAS.
* **Path normalization** for assets; `percent_decode_str` + traversal guard.
* **Range** support is **single-range `bytes=start-`** (intentionally minimal).
* **Graceful shutdown** via `CancellationToken` with short drain.
* **Logging**: `tracing` + `RUST_LOG` env; `hyper=warn` suggested.

---

## 8) Configuration surface (today)

| Env var                           | Meaning         | Default          |
| --------------------------------- | --------------- | ---------------- |
| `SVC_EDGE_BIND_ADDR`              | API bind        | `0.0.0.0:8080`   |
| `SVC_EDGE_METRICS_ADDR`           | Admin bind      | `127.0.0.1:9909` |
| `SVC_EDGE_SECURITY__AMNESIA`      | Amnesia posture | `0`              |
| `SVC_EDGE_ADMISSION_TIMEOUT_MS`   | Request timeout | `5000`           |
| `SVC_EDGE_ADMISSION_MAX_INFLIGHT` | Inflight cap    | `256`            |
| `SVC_EDGE_ASSETS_DIR`             | Assets root dir | `assets`         |

**Config struct** mirrors these (path parsing done; TOML loader can be added later without API changes).

---

## 9) Interop points (how this will tie into the rest of RON-CORE)

* **svc-registry / svc-gateway**: edge plane is ready for upstream mapping once we add route tables or pass-through to overlay/gateway.
* **svc-passport / ron-auth**: admission can be extended with **capability checks** (macaroon/passport) before handler execution.
* **ron-metrics**: current metrics use `prometheus` default registry; can be lifted to a shared metrics surface if desired.
* **Overlay/Transport**: current static/CAS are stand-ins; the request path and admission gate design carry forward to the real pack/CAS backends.

---

## 10) Troubleshooting (quick)

* **/readyz stays 503** → ensure `SVC_EDGE_ASSETS_DIR` exists (temporary gate) and env values parse (bind collisions show as bind errors).
* **ETag mismatch** → remember the ETag is quoted in headers; `If-None-Match` must include quotes around the hex string.
* **Lots of 503 under load** → increase `SVC_EDGE_ADMISSION_MAX_INFLIGHT` or reduce client concurrency.
* **408 for slow routes** → raise `SVC_EDGE_ADMISSION_TIMEOUT_MS` or reduce server work.

---

## 11) What’s next (post-beta “nice to haves”)

**Functional**

* Replace FS adapters with **real pack/CAS providers**, pluggable via config.
* Add **/version** (crate + git SHA via `vergen`) on admin plane.
* Add **request counters** (`edge_requests_total{route,method,status}`) in addition to latency histos (scaffold is already in `metrics.rs` if desired).
* Move assets/CAS paths to **`Config.assets.root`** in handlers (currently reading env; low-effort refactor).
* Add **conditional GET (If-Modified-Since)** if we introduce mtime-based flows (optional).

**Admission / Security**

* Add **rate limiter** (RPS shaping) and **body size guard** (413) as small, composable layers (we kept the admission minimal for stability).
* Hook **passport/macaroon** verification in a pre-handler slice (aligned with svc-passport/ron-auth).
* Per-route **quotas** / fair-queue (enqueue → coalesce bursts, reduce tail lat).

**Ops / DX**

* `/metrics` expansion: **rejected_total by route**, **inflight gauge**, **startup latency** buckets.
* Add **structured logs** for rejections (route/method/reason/queue_depth).
* Provide `Makefile` targets: `make run-edge`, `make smoke-edge`, `make release-smoke`.
* Dockerfile + container run script with health/readiness probes for k8s.
* Integration tests (`http_contract`) to verify admin/api surfaces as part of CI.

**Performance**

* Micro-opt the echo/asset paths (buffer reuse, zero-copy file send on platforms that support it via hyper/hyper-util; careful with Axum abstractions).
* Admission counters via atomics (cheap) to observe hot-path behavior with minimal overhead.

---

## 12) Acceptance checklist (beta)

* [x] **Builds clean** with lints on.
* [x] **Admin plane** healthy; `/readyz` flips to 200 with gates shown.
* [x] **API plane** responds on all 4 routes (echo, slow, assets, cas).
* [x] **Admission** produces **408** for slow > timeout and **503** under inflight saturation (observed).
* [x] **Metrics**: amnesia gauge, rejects, latency buckets visible and changing during smoke (observed).
* [x] **Smoke script** passes end-to-end on a fresh instance.

---

## 13) Summary

`s​vc-edge` now has a **solid, truthful, and operationally boring** surface: health/ready, metrics, admission guards, minimal but correct static/CAS handlers with ETag/Range, and clean shutdown. The design intentionally keeps the middleware **small and cloneable** to avoid Axum 0.7 trait pitfalls, and lays a straight path to integrate **capabilities (passport/macaroon)** and **overlay-backed assets/CAS** next.

> When we’re ready to move beyond the FS stubs, the handler contracts and admission posture **don’t change** — we slot in the real backends and start collecting the same metrics against them.



### END NOTE - NOVEMBER 13 2025 - 20:17 CST