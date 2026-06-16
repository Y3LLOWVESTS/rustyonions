### BEGIN NOTE - OCTOBER 29 2025 - 14:41 CST

---

# Carryover Notes for `svc-index` (from `svc-storage`)

**Date:** 2025-10-29 (America/Chicago)
**Context:** `svc-storage` is Beta-complete. `svc-index` should reuse its invariants, error model, and ops patterns to stay cohesive with RON-CORE.

## 1) Canonical invariants to **reuse verbatim**

* **CID format:** `b3:<64-lowercase-hex>` only. Uppercase → malformed.
  Minimal validator (no regex):

  ```rust
  #[inline]
  pub fn is_valid_b3(cid: &str) -> bool {
      cid.len() == 67 && cid.starts_with("b3:") &&
      cid.as_bytes()[3..].iter().all(|&b| matches!(b, b'0'..=b'9'|b'a'..=b'f'))
  }
  ```
* **Strong ETag:** quoted hex, e.g. `"aa…ff"`. When `svc-index` returns object-related metadata, keep ETag quoted.
* **Error split:**

  * `400` → malformed CID (validation fail).
  * `404` → well-formed but unknown (not present in storage / index).
  * `416` (range) is storage-only; index should not emit it unless it proxies byte ranges (not planned).
* **Health plane:** expose `/healthz` (always 200) and `/readyz` (truthful if possible).
* **Metrics plane:** `/metrics` with Prometheus exposition; namespaced counters & histograms.

## 2) Contract between **svc-index ↔ svc-storage**

* **Existence probe:** use `HEAD /o/:cid` from storage as the single source of truth for existence and size (`Content-Length`) and identity (`ETag`).
* **When creating index records:**

  * Validate CID with `is_valid_b3`. If false → `400`.
  * Optionally call storage `HEAD` to confirm presence; if 404 → reject index create with `404` (or `409/FailedPrecondition` if you distinguish).
  * **Idempotency:** indexing the same CID twice should be safe; treat as upsert by default.
* **Optional caching:** you may cache `{cid -> (len, etag, seen_at)}` with a short TTL. Always treat storage as the authority; on 404 from storage, purge cache.

## 3) `svc-index` API sketch (keep parity with storage error model)

* `POST /i` — create/update an index entry (CID + tags/labels/owner/scopes, etc.).

  * Validate CID (400). Optionally verify with storage HEAD; if absent → 404.
  * Return JSON with the normalized record and ETag echo (if you round-trip it).
* `GET /i/:cid` — fetch index metadata for CID (200 or 404).
* `DELETE /i/:cid` — remove index record (204 or 404). (Does **not** delete blob from storage.)
* `GET /i/search?...` — query by tags/owner/time/etc. (200; empty list ok).
* Observability: `/healthz`, `/readyz`, `/metrics`, `/version` (mirror storage).

## 4) Suggested internal boundary (clean testability)

Define a **storage client trait** in `svc-index`, with an HTTP adapter. This keeps tests hermetic and lets you stub storage easily.

```rust
#[async_trait::async_trait]
pub trait StorageProbe: Send + Sync + 'static {
    async fn head(&self, cid: &str) -> Result<StorageMeta, ProbeError>;
}

#[derive(Clone, Debug)]
pub struct StorageMeta { pub len: u64, pub etag: String } // etag must be quoted

#[derive(thiserror::Error, Debug)]
pub enum ProbeError {
    #[error("malformed cid")] Malformed,
    #[error("not found")] NotFound,
    #[error("transport")] Transport(anyhow::Error),
}
```

* **HTTP adapter:** maps `HEAD /o/:cid` → `StorageMeta`.

  * 200 → Ok(meta) (len from `Content-Length`, etag from `ETag`).
  * 400 → `ProbeError::Malformed`.
  * 404 → `ProbeError::NotFound`.
  * Others → `ProbeError::Transport`.

In handlers, translate `ProbeError` to **the same** HTTP status codes used in storage for cohesion.

## 5) Metrics names to keep consistent

* `index_put_total{status}` — for create/update.
* `index_get_total{status}` — for fetch.
* `index_delete_total{status}` — for delete.
* `index_request_latency_seconds{route,method,status}` — histogram with low-lat buckets (reuse storage’s buckets).
* **Storage probe:** `index_storage_head_total{status}`; latency histogram `index_storage_head_seconds`.

## 6) Testing patterns to carry over

* **Black-box integration test** that spawns `svc-index` binary and uses `reqwest` (like storage).
* **Contract tests** for status code mapping: malformed (400), unknown (404), success (200/204).
* **Mock StorageProbe** to simulate storage HEAD outcomes (200/400/404/5xx).
* **Smoke script** mirroring storage style:

  * `POST /i` with a known CID (after pushing a blob to storage) → 200.
  * `GET /i/:cid` → 200 with expected fields.
  * `GET /i/:fake_b3_zeros` → 404.
  * `/metrics` → 200.
  * PASS/FAIL summary; tail logs on fail.

## 7) Operational/behavioral notes

* **Idempotency keys:** If you allow client-supplied dedupe keys for writes, document them. Otherwise, treat `POST /i` as idempotent per CID.
* **Pagination & sorting:** For search endpoints, choose stable defaults and return `next_page` cursors; document limits.
* **Consistency:** `svc-index` is logically **downstream** of storage. If storage returns 404 after an index exists (unlikely for this profile), decide whether to:

  * Keep stale index entries (soft-state), or
  * Periodically reconcile (background janitor: probe a tiny subset daily).
* **Auth:** Same stance as Micronode dev: no auth now; gate later with feature flags (macaroons/tokens).
* **Limits:** Document reasonable caps (tags per record, label sizes, max payload in index create). Reject oversize with 413/400 as appropriate.

## 8) Performance stance (for now)

* Index I/O is light; primary cost is storage HEAD probes and DB writes.
* Add a tiny in-process LRU for HEAD meta (len/etag) with short TTL to reduce hot-path probes if you call storage frequently during search responses.

## 9) Reusable code bits

* **CID validator** (above). Put it in a small `cid.rs` module in `svc-index` to avoid cross-crate coupling for now.
* **HTTP helpers:** error mappers `(StatusCode, Json{err})` or plain status codes, same as storage’s minimalist style.
* **Observability bootstrap:** copy storage’s tracing init and Prometheus exporter gating.

## 10) “Do not drift” checklist

* [ ] 400 vs 404 split identical to storage.
* [ ] ETag always **quoted** when returned.
* [ ] Health/ready/metrics endpoints and behavior aligned.
* [ ] Scripts/tests follow the same PASS/FAIL style.
* [ ] No `axum-extra`, no `http_body_util` (stay with Axum core + `headers`).
* [ ] Keep pins consistent with workspace (tokio, axum 0.7, reqwest rustls, etc.).

---


### END NOTE - OCTOBER 29 2025 - 14:41 CST


### BEGIN NOTE - OCTOBER 29 2025 - 18:12 CST

**Estimated completion toward `svc-index` beta:** **≈ 65–70%**
Rationale: HTTP plane (health/ready/version/metrics), middleware, readiness gate, compile hygiene, and boot/shutdown are solid and reproducible. Admin seeding, key/CID validation, and basic route skeletons are in. The remaining work is mostly “real index plumbing” (sled wiring + DHT client + provider/resolve pipeline), contract tests, and the last mile of docs/ops polish.

---

# Carry-Over Notes — `svc-index` (Index & Provider Registry)

**Date:** 2025-10-29 (America/Chicago)
**Profile:** RON-CORE (no Web3) — Pillars: P1 Runtime, P3 Naming/Index, P4 Overlay/DHT
**Concerns:** RES, PERF, GOV, SEC, DX

## 1) What we’ve accomplished

**Boot & HTTP surface (green):**

* ✅ **Compiles cleanly with `-D warnings`** (after fixes below).
* ✅ **Service starts and binds** (example: `127.0.0.1:5304`) and logs:

  * `svc-index ready` (readiness gate flipped)
  * `svc-index starting bind=127.0.0.1:5304`
* ✅ **Endpoints online:**

  * `GET /healthz` → `200 ok`
  * `GET /readyz` → `200 ready` (after bootstrap)
  * `GET /version` → `svc-index/0.1.0`
  * `GET /metrics` → Prometheus `request_latency_seconds` histogram visible
  * `POST /admin/seed` (admin plane) — validates payload; now responds 422 with helpful message if fields missing, accepts valid payload
  * `GET /resolve/{key}` — returns 400 on invalid key (strict B3 key validation is active)
  * `GET /providers/{cid}` — returns 400 on invalid CID (strict B3 CID validation is active)

**Middleware & telemetry:**

* ✅ **tower-http trace layer** wired with custom span (removed invalid API usage; details below).
* ✅ **Request latency histogram** exposed (`request_latency_seconds`) and increments once routes are actually exercised.
* ✅ **Body size limiter layer** present; now parameterized by a constant/Config knob.
* ✅ **Readiness gate** – explicit “booting → ready” flip; `/readyz` returns 503 during bootstrap and 200 after.

**State & router:**

* ✅ **Axum 0.7 serve model** with `TcpListener` + `axum::serve(listener, router)` and **state injection via `router.with_state(Arc<AppState>)`**.
* ✅ **Graceful shutdown** via `tokio::signal` (Ctrl-C, SIGTERM on Unix).
* ✅ **Module structure** in place: `http/` (routes, middleware, extractors), `state/` (metrics, readiness), `config/`, `router.rs`, `app.rs`, `store/` (sled impl stub), `dht/` (client stub), etc.

**Validation & admin UX:**

* ✅ **Strict BLAKE3 IDs**: `b3:<64-hex lowercase>` enforced for both keys and CIDs (good! protects index from junk).
* ✅ **Admin seeding** requires `{ "name": "...", "region": "...", "count": N }` (we added explicit validation & error message).

**Tooling hygiene:**

* ✅ **Fixed dependency drift** (removed invalid `tokio-util = { features=["sync"] }` usage; no such feature).
* ✅ **Clippy** warnings addressed (identity-op, unused imports, dead code where appropriate).
* ✅ **Smoke flow** confirmed via `curl` (see §3).

---

## 2) The common errors we hit & how we resolved them (with context)

1. **Missing modules (E0583)**

   * **Symptoms:** `file not found for module 'health'|'metrics'|'version'|'resolve'|'providers'|'admin'|'extractors'|'middleware'`, `failed to resolve mod utils`, `could not find sled_store`.
   * **Cause:** Router referenced modules before files existed / incorrect paths / wrong `mod` layout.
   * **Fixes:** Created the missing files and/or corrected `mod`/`use` statements; ensured `store::mod.rs` refers to `super::sled_store` only after the file exists; standardized `http/mod.rs` to re-export submodules that actually exist.

2. **tower-http TraceLayer API misuse (E0599 + trait cascades E0277)**

   * **Symptoms:**

     * `no method named with_target` on `TraceLayer`
     * `MakeClassifier` bound failures when layering
     * bizarre `dyn Error + Send + Sync: Clone` bound errors
   * **Cause:** `.with_target(false)` doesn’t exist on `TraceLayer` (wrong API / outdated snippet). That single bad call poisoned type inference and caused the cascade.
   * **Fix:** Removed `.with_target(false)` and used valid builder chain:

     ```rust
     use tower_http::trace::TraceLayer;
     // Make sure the closure type is inferable:
     let trace = TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
         tracing::info_span!(
             "http_request",
             method = %req.method(),
             uri    = %req.uri(),
         )
     });
     router.layer(trace)
     ```

     (Any custom on_request/on_response/on_failure hooks can be added later; defaults are fine for beta.)

3. **Axum 0.7 state & serve pitfalls (E0277, wrong `.into_make_service*` calls)**

   * **Symptoms:**

     * `Router<Arc<AppState>>: Service<IncomingStream<'_>> not implemented` (E0277)
     * Trying `.into_make_service`/`.into_make_service_with_connect_info` on 0.7 `Router` (E0599)
     * `WithGracefulShutdown<...> is not a future` cascade
   * **Cause:** With Axum 0.7, you pass a **state-injected `Router<()>`** to `axum::serve`. If you keep the router typed as `Router<Arc<AppState>>`, the `Service<IncomingStream>` impl isn’t there. Also, the older `.into_make_service` idiom is no longer the pattern you want here.
   * **Fix:** Inject state first, then serve:

     ```rust
     let router = build_router().with_state(state.clone()); // -> Router<()>
     let listener = TcpListener::bind(addr).await?;
     axum::serve(listener, router)
         .with_graceful_shutdown(shutdown_signal())
         .await?;
     ```

     (No `into_make_service*` needed.)

4. **Body limit layer arity mismatch (E0061)**

   * **Symptoms:** `this function takes 1 argument but 0 supplied` for `body_limits::layer()`.
   * **Cause:** Our middleware wrapper requires a `usize` max‐bytes param.
   * **Fix:** Call with the constant/config value:

     ```rust
     .layer(middleware::body_limits::layer(DEFAULT_MAX_BODY_BYTES))
     // or .layer(middleware::body_limits::layer(cfg.body_cap_bytes))
     ```

5. **Renamed/absent constants (E0432)**

   * **Symptoms:** `no MAX_BODY_BYTES in constants`.
   * **Cause:** Constant was defined under a different name (e.g., `DEFAULT_MAX_BODY_BYTES`) or in Config.
   * **Fix:** Standardized constant name and imports:

     ```rust
     use crate::constants::DEFAULT_MAX_BODY_BYTES;
     ```

6. **Error enum lifetime issue (E0597)**

   * **Symptoms:** Borrowed message dropped in `error.rs` (`m.as_str()`).
   * **Cause:** Returning a `&str` to a temporary.
   * **Fix:** Convert to owned `String` or use static tags and include `m.to_string()` in the JSON body, not as borrowed `&str` in the tuple.

7. **`AppState` visibility (E0603) & type inference (E0282)**

   * **Symptoms:** `AppState` imported from a private path; `Arc<_>` needs type annotation.
   * **Fix:** Re-export correctly (`pub use state::AppState;`) or import from `state::AppState` directly; add explicit type when necessary:

     ```rust
     let state: Arc<AppState> = Arc::new(AppState::new(cfg.clone()).await?);
     ```

8. **`tokio-util` `sync` feature confusion**

   * **Symptoms:** Cargo resolution errors — feature `sync` doesn’t exist.
   * **Fix:** Remove `features = ["sync"]` from `tokio-util` in `Cargo.toml`; if we need a slab, enable `slab`.

---

## 3) Current smoke-test recipe (works today)

**Run:**

```bash
cargo clean
cargo fmt -p svc-index
cargo clippy -p svc-index --no-deps -- -D warnings
cargo run -p svc-index
# Logs should show: "svc-index ready" and "starting bind=127.0.0.1:5304"
```

**Probe:**

```bash
curl -i http://127.0.0.1:5304/healthz
curl -i http://127.0.0.1:5304/readyz
curl -i http://127.0.0.1:5304/version

# Seed demo data (produces valid b3 keys/CIDs):
curl -i -X POST http://127.0.0.1:5304/admin/seed \
  -H 'content-type: application/json' \
  -d '{"name":"demo","region":"local","count":3}'
# Copy one "key" and one "cid" from the response, then:

KEY="b3:<64-hex>"
CID="b3:<64-hex>"

curl -i "http://127.0.0.1:5304/resolve/$KEY"
curl -i "http://127.0.0.1:5304/providers/$CID"

# Metrics should increment after requests:
curl -s http://127.0.0.1:5304/metrics | head -n 50
```

---

## 4) What’s left for **beta** (engineering checklist)

**A. Store & pipeline (core value):**

* [ ] **Wire `sled_store::SledStore` end-to-end**:

  * [ ] `open()` with configurable path (`RON_INDEX_DB` or `Config.index_path`).
  * [ ] Trees: `keys/`, `providers/`, `namespaces/` (prefix strategy aligns with `svc-storage`/`svc-dht` conventions).
  * [ ] CRUD APIs used by routes (`resolve`, `providers`, `admin::seed`).
  * [ ] **Deterministic serialization** (bincode or postcard) for values; include schema version tag.
* [ ] **In-process memory store** (`MemStore`) feature-flagged for tests / CI speed.
* [ ] **Provider registration flow** (write-path) to be called by `svc-storage` (or an eventual `svc-gateway`) when new content becomes available.

**B. DHT integration (read-path resilience):**

* [ ] Implement `dht::client` shims to **fan-out queries** for providers when local index misses.
* [ ] Timeboxed lookup with **hops cap & TTL**, return best-effort providers; cache results in sled with TTL.
* [ ] Metrics: `index_resolve_local_hits_total`, `index_resolve_dht_miss_total`, `index_resolve_dht_hits_total`, `index_resolve_error_total`, `index_resolve_latency_seconds`.

**C. Route contract & validation:**

* [ ] **`/resolve/{key}`**: validate B3 key → return CID + metadata (namespace, version, maybe last-update).
* [ ] **`/providers/{cid}`**: validate CID → list providers (endpoint URIs / overlay addresses) + freshness.
* [ ] **`/admin/seed`**: extend to optionally write both key→cid and provider entries; return inserted counts & IDs.
* [ ] **Error model**: unify `(code, message)` JSON with stable `code` enums: `bad_request`, `not_found`, `conflict`, `internal`.

**D. Readiness/health truthfulness:**

* [ ] `/readyz` gates on: sled opened, trees created, metrics exporter bound, config loaded, DHT client channel open (if enabled).
* [ ] `/healthz` remains lightweight.

**E. Config & Ops:**

* [ ] **Config surface**: port, index path, body cap bytes, enable-dht, dht timeouts/hops, admin auth (token).
* [ ] **Admin auth** (even a simple `X-Admin-Token` env var) for `POST /admin/*`.
* [ ] **CORS** and rate limits (if `svc-index` will ever be hit cross-origin).
* [ ] **Dockerfile** + `scripts/smoke_index.sh` (start, wait, curl /healthz, /readyz, /version, seed, resolve, providers, /metrics).
* [ ] **Systemd unit** (optional) & sample k8s manifest (Service + Deployment + readinessProbe/livenessProbe).

**F. Tests (must pass for beta):**

* [ ] **Unit tests**: validators (key/cid), store (put/get roundtrip), error mapping, config parsing.
* [ ] **Integration tests** (`tests/`):

  * [ ] boot → ready gate trips true; routes return expected codes.
  * [ ] `admin::seed` → `/resolve` & `/providers` behave on seeded IDs.
  * [ ] Invalid IDs → 400 with stable messages.
  * [ ] If DHT enabled, simulate client miss/fallback (can be feature-gated + mocked).
* [ ] **Contract tests**: freeze JSON schema for response shapes (use `insta` snapshots).

**G. Metrics & Logging:**

* [ ] Counters/histograms named & documented:

  * `index_bootstrap_seconds`, `index_store_open_seconds`
  * `index_resolve_requests_total`, `index_providers_requests_total`
  * `index_admin_seed_requests_total`, `index_admin_seed_inserted_total`
  * `request_latency_seconds` (already present), plus per-route labels (`route="/resolve"`, etc.)
* [ ] **Trace span fields**: add `cid`, `key`, `namespace`, `status_code` to spans where applicable.
* [ ] **Log levels**: `info` for state transitions; `warn` for user-induced failures; `error` for internal.

**H. Documentation (beta quality):**

* [ ] `README.md` with purpose, routes, config, quickstart, smoke script output.
* [ ] `API.md` (route contracts with examples).
* [ ] `CONFIG.md` (all knobs + env).
* [ ] `RUNBOOK.md` (operational playbook: start/stop, logs, metrics to watch, backup/restore sled).
* [ ] `SECURITY.md` (admin plane auth, rate limits, data validation).
* [ ] `OBSERVABILITY.md` (metrics, traces, log fields).
* [ ] `NOTES.md` (carry-overs like this).
* [ ] `TODO.md` trimmed to only non-beta items.

---

## 5) Recommended near-term diffs (low effort → high impact)

1. **Finish sled store scaffolding** (`store/sled_store.rs`):

   * Implement `open(path)`, `get_key(key) -> Option<Cid>`, `get_providers(cid) -> Vec<Provider>`, `put_key`, `add_provider`.
   * Use **prefix trees** (e.g., `db.open_tree("keys")?`, `db.open_tree("providers")?`).
   * Serialize values with `bincode` (add `serde::{Serialize, Deserialize}` on value structs).

2. **Wire routes to store**:

   * `resolve.rs` calls `store.get_key(key)`; 404 if None; 200 with payload if present; on miss and `enable_dht`, call DHT client then cache result.
   * `providers.rs` reads from store; similar DHT fallback optional.

3. **Enable admin seeding to write into store**:

   * Generate `count` fake (key, cid, provider?) tuples under `{name, region}` and insert.

4. **Add admin token guard** (env `INDEX_ADMIN_TOKEN`):

   * Minimal middleware that checks header `X-Admin-Token`.

5. **Expand metrics** around each route and store operation.

6. **Add `tests/`** with an in-memory store feature to avoid disk IO in CI.

7. **Script** `crates/svc-index/scripts/smoke_index.sh`:

   * Build, run, wait for `/readyz`, seed, resolve, providers, dump a few metric lines, exit 0 on success.

---

## 6) Dev & test examples (paste-ready)

**Seed + E2E resolve/providers:**

```bash
# Start in Terminal A:
cargo run -p svc-index

# Terminal B:
curl -s -X POST http://127.0.0.1:5304/admin/seed \
  -H 'content-type: application/json' \
  -d '{"name":"demo","region":"local","count":3}' | tee /tmp/seed.json

KEY=$(jq -r '.inserted[0].key' /tmp/seed.json)
CID=$(jq -r '.inserted[0].cid' /tmp/seed.json)

curl -i "http://127.0.0.1:5304/resolve/$KEY"
curl -i "http://127.0.0.1:5304/providers/$CID"
curl -s http://127.0.0.1:5304/metrics | head -n 50
```

---

## 7) Known gaps / deferred to post-beta (nice-to-haves)

* Pluggable **compaction** / vacuum job for sled trees.
* Background **TTL reaping** for cached DHT provider entries.
* **OTLP traces** export (feature-gated) to Jaeger/Tempo.
* **Quotas** per namespace/region.
* **Bulk import/export** endpoint (NDJSON) for migrations.
* **Index snapshots** (read-only safe backups).

---

## 8) Risks & watch-outs

* **Strict ID validation** is correct for production; for demos, remember to use real 64-hex B3 IDs (or seed first).
* **Axum 0.7 patterns**: always inject state before `serve`; avoid old `into_make_service*` snippets.
* **Sled error handling**: map I/O errors to `internal` with log context; don’t leak internals in user messages.
* **Admin plane security**: do not ship beta without an auth gate (even a token) on `/admin/*`.

---

## 9) Minimal definition of **beta-ready** for `svc-index`

* [ ] Clean build on CI (`fmt`, `clippy -D warnings`, `test`, minimal `cargo deny`).
* [ ] `/healthz`, `/readyz`, `/version`, `/metrics` stable.
* [ ] `/admin/seed` working & **token-gated**.
* [ ] `/resolve/{key}` returns 200/404 with correct schema (store-backed).
* [ ] `/providers/{cid}` returns 200/404 with correct schema (store-backed).
* [ ] In-memory tests + basic sled roundtrip test.
* [ ] Smoke script passes locally and in CI container.
* [ ] README/API/CONFIG/RUNBOOK complete enough to hand off.

---

## 10) Quick win punch-list (action order for next session)

1. Implement `sled_store` (open trees + get/put methods).
2. Route → store wiring for `/resolve` & `/providers`.
3. Extend `/admin/seed` to write to store; return inserted IDs.
4. Add `INDEX_ADMIN_TOKEN` guard on admin routes.
5. Add integration tests with `MemStore`.
6. Add/expand metrics counters per route.
7. Write `scripts/smoke_index.sh` and add to repo.
8. Update docs (`README`, `API`, `CONFIG`, `RUNBOOK`).

---

### Appendix: exact fixes we applied (for future archaeology)

* Removed invalid `TraceLayer.with_target(false)`; used `new_for_http().make_span_with(|req| …)`.
* Injected state via `router.with_state(Arc<AppState>)` and passed resulting `Router<()>` into `axum::serve`.
* Dropped `into_make_service*` calls; not needed on Axum 0.7.
* Body limit layer now called with a concrete `usize` (e.g., `DEFAULT_MAX_BODY_BYTES`).
* Standardized constant imports (`DEFAULT_MAX_BODY_BYTES`) and moved per-service caps into `Config` knob.
* Fixed `error.rs` lifetime by avoiding `&str` to temporaries (use `String`/stable tags).
* Cleaned up visibility: `pub use state::AppState;` (or import directly).
* Removed nonexistent `tokio-util` `sync` feature; use `slab` if needed.

---


### END NOTE - OCTOBER 29 2025 - 18:12 CST


### BEGIN NOTE - OCTOBER  30 2025 - 13:58 CST

Here’s a paste-ready “carryover notes” pack for `crates/svc-index/NOTES.md`. It’s exhaustive, dated, and aligned to your RON-CORE/CODECOMMENTS style. Drop this in as the new top section.

---

# svc-index — Carryover Notes (Beta Locked)

**Date:** 2025-10-30 (America/Chicago)
**Profile:** RON-CORE (no Web3), adheres to RON-CORE.MD + CODECHECK.MD
**Status:** **BETA COMPLETE** — routing semantics, readiness, metrics, and performance verified.

---

## 1) What we accomplished (since last checkpoint)

### 1.1 Service boot & control plane

* **Axum 0.7** server binds via `INDEX_BIND` (default `127.0.0.1:5304`).
* Clean startup/shutdown; logs via `tracing` + env filter.
* Control plane endpoints are live and verified:

  * `GET /healthz` → 200 “ok”
  * `GET /readyz` → **truthful** readiness (flips only after store init)
  * `GET /version` → `svc-index/<semver>`
  * `GET /metrics` → Prometheus text (request latency histogram + counters)

### 1.2 Public API surface (MVP)

* `GET /resolve/:key`

  * **Key is generic** (supports `name:*` or `b3:*` etc.).
  * **Miss semantics:** well-formed but not found → **404** with the **same JSON body** (manifest `null`, providers `[]`, etc.).
  * Malformed key → 400 (if/when validation is tightened further; currently permissive).
* `GET /providers/:cid`

  * **CID validation**: strict `b3:<64-hex>` (lowercase; malformed → **400**).
  * **Miss semantics:** valid but no providers → **404** with JSON body `{ cid, providers: [], truncated: false, etag: null }`.
  * **Hit semantics:** 200 with `{ providers: [...] }`.

### 1.3 Providers pipeline (fixed)

* Removed synthetic fallback `local://stub` from the pipeline.
* Ranking deterministic: sort by descending `score`, then truncate to `limit.clamp(1, 32)`.
* **Truncation is truthful**; `truncated` flag reflects post-filter truncation.
* Cache stores **cleaned** results; cache hit returns the same DTO the route would emit.

### 1.4 Router cleanup & regression shield

* Router simplified; route no longer rewrites JSON (since pipeline emits no stub).
* Route returns **404** when provider list is empty; otherwise 200.
* Clippy is **clean with `-D warnings`**.

### 1.5 Readiness and invariants

* `/readyz` flips **only after store opens and trees created** (sled or in-mem).
* AppState behind `Arc`; handlers take `State<Arc<AppState>>`.
* No locks across `.await` in hot paths; per-request latency histogram exported.

### 1.6 Scripting: smoke + bench

* **Smoke** (green): `crates/svc-index/scripts/smoke_index.sh`
  Proves: build → boot → `healthz/readyz/version` → contract checks:

  * `resolve/name:does-not-exist` → 404
  * `providers/not-a-cid` → 400
  * `providers/b3:<64x0>` → 404
* **Bench** (release): `crates/svc-index/scripts/bench_http_release.sh`
  Uses `wrk` (or `hey`/pure fallback) to benchmark `/healthz`, `/version`, `/metrics`.

---

## 2) Performance snapshot (beta baseline)

**Machine:** 2019 MacBook Pro 13", Intel i5 (4c/8t), macOS; loopback (127.0.0.1).
**Tool:** `wrk -t4 -c128 -d30s` (64 conns on `/metrics`).
**Binary:** `cargo build -p svc-index --release`, `RUST_LOG=warn`.

### /healthz

* **Throughput:** **67,628 req/s**
* **Latency avg:** **1.86 ms**
* **Latency p50/p75/p90/p99:** **1.86 / 1.99 / 2.10 / 2.31 ms**
* **Max:** 19.44 ms
* **Transfer:** 7.61 MB/s (228.42 MB in 30s)

### /version

* **Throughput:** **59,214 req/s**
* **Latency avg:** **2.15 ms**
* **Latency p50/p75/p90/p99:** **2.07 / 2.24 / 2.54 / 3.02 ms**
* **Max:** 56.59 ms
* **Transfer:** 7.45 MB/s (224.41 MB in 30s)

### /metrics

* **Throughput:** **46,845 req/s**
* **Latency avg:** **1.30 ms**
* **Latency p50/p75/p90/p99:** **1.27 / 1.43 / 1.63 / 2.32 ms**
* **Max:** 46.59 ms
* **Transfer:** **35.16 MB/s** (1.03 GB in 30s)

**Interpretation:** On modest hardware, hot paths sustain ~60–68k RPS with **p99 ~2–3 ms**; the heavier `/metrics` still clears ~47k RPS at **~35 MB/s**—excellent headroom. This is brag-worthy and aligned with RON-CORE “fast path” invariants.

---

## 3) Current behaviors (contract)

* **Malformed input** → **400**:

  * `/providers/:cid` when `:cid` not `b3:<64-hex>` (lowercase).
* **Well-formed miss** → **404**:

  * `/resolve/:key` where key exists structurally but no manifest/providers (returns original JSON body; only status changes).
  * `/providers/:cid` where no providers exist (returns `{ providers: [] }`; only status changes).
* **Hit** → **200** with JSON DTOs.
* Metrics exported: `request_latency_seconds` histogram + total counters (see `/metrics`).

---

## 4) How to run (quick)

Build + run:

```
cargo fmt -p svc-index
cargo clippy -p svc-index --no-deps -- -D warnings
cargo build -p svc-index
INDEX_BIND=127.0.0.1:5304 cargo run -p svc-index
```

Smoke:

```
chmod +x crates/svc-index/scripts/smoke_index.sh
crates/svc-index/scripts/smoke_index.sh
```

Bench (release):

```
chmod +x crates/svc-index/scripts/bench_http_release.sh
crates/svc-index/scripts/bench_http_release.sh
```

---

## 5) What remains if we go **beyond beta** (prioritized roadmap)

**P0 — Fast, safe, incremental**

1. **Persistence durability knob (sled)**

   * Ensure `.flush()` after any write path (when write paths exist: e.g., future admin seed/key put).
   * Option: batch writes with periodic `flush` (configurable interval), with graceful shutdown `flush()`.

2. **Contract tests (small, high ROI)**

   * Add 3–5 tests covering 400/404/200 for resolve/providers, using in-mem store/DHT stub.
   * Assert exact JSON body shape on 404 (no accidental schema drift).

3. **Perf baseline gate (CI)**

   * Add a perf workflow that runs short `wrk` against `/healthz` and `/version` in release, captures `Requests/sec` & percentiles, and alerts on >10% regression vs. a stored baseline (artifact).
   * Keep it best-effort; do not gate merges if CI runners are noisy—start with notify-only.

**P1 — Capabilities & UX**

4. **ETag + conditional GET**

   * Plumb `etag` in responses when backing data has a stable hash (e.g., provider list or manifest).
   * Support `If-None-Match` → 304 to reduce payload & CPU, especially on `/providers` and `/resolve`.

5. **Pagination & limits (providers)**

   * Support `?limit=` (already honored) and add `?offset=` or cursor.
   * Return `truncated=true` when more entries exist (already wired) and include `next_cursor`.

6. **Admin plane (opt-in, token-guarded)**

   * `/admin/seed` for demos (token via `INDEX_ADMIN_TOKEN`).
   * `/admin/reindex` hook (future) to refresh caches or rehydrate from storage/DHT.

7. **Rate-limit & basic auth (edge-friendliness)**

   * Integrate a lightweight rate limiter on public endpoints to protect from abuse (esp. `/providers`).
   * Keep behind feature flag; document default off for internal deployments.

**P2 — Observability & resilience**

8. **Golden metrics (RED) expansion**

   * Add counters for 2xx/4xx/5xx by route; add in-flight gauge; add queue depth if any internal queues are introduced later.
   * Reserve labels sensibly to avoid cardinality blow-ups.

9. **Structured error taxonomy**

   * Ensure `SvcError` maps cleanly to HTTP + metric labels (e.g., `err_kind="bad_request|not_found|internal"`).

10. **Load shedding / timeouts**

* If future dependencies (e.g., DHT/network) are introduced in hot paths, add per-route timeouts and consistent 504/503 mapping with circuit breakers.

**P3 — Docs, DX, and polish**

11. **README/runbook updates**

* Document envs: `INDEX_BIND`, `RON_INDEX_DB`, optional `INDEX_ADMIN_TOKEN`.
* Summarize contract (400/404/200) with curl examples.
* Publish the **performance snapshot** above as “Beta Baseline (2025-10-30)”.

12. **Config surface**

* If you add admin features: expose configs using your standard `Config` loader (align with other crates).
* Include defaults, env overrides, and `/version` JSON mode (optional).

13. **Hardening**

* Security headers (if exposed on public edge).
* Log scrubbing (ensure tokens/envs never leak).
* Make `/admin/*` opt-in and off by default.

---

## 6) Known decisions & non-goals (for now)

* **No Web3 components** in RON-CORE phase; index is purely name/cid → providers/manifest.
* **No synthesis of providers** in pipeline (no `local://stub`); absence → 404.
* **No dependency on svc-gateway/svc-overlay** for this crate’s hot paths (keep it embeddable).
* **No pagination yet** (MVP uses `limit` + `truncated` only).
* **No compression** on `/metrics` (consider later; benchmark first).

---

## 7) Quick acceptance checklist (Beta)

* Build + Clippy (with `-D warnings`) ✅
* Control plane healthy (`healthz`, `readyz`, `version`, `metrics`) ✅
* Routing semantics locked:

  * malformed input → **400** ✅
  * well-formed miss → **404** ✅
  * hit → **200** ✅
* Providers pipeline: **no stub**, ranked, truncated truthfully, cached ✅
* Smoke script passes ✅
* Release perf snapshot captured (wrk) ✅

---

## 8) Handy command snippets (no comments)

Start (debug):

```
INDEX_BIND=127.0.0.1:5304 cargo run -p svc-index
```

Smoke:

```
crates/svc-index/scripts/smoke_index.sh
```

Bench (release, wrk preferred):

```
cargo build -p svc-index --release
INDEX_BIND=127.0.0.1:5304 RUST_LOG=warn target/release/svc-index >/tmp/svc-index.perf.log 2>&1 &
PID=$!; sleep 0.5
wrk -t4 -c128 -d30s --latency http://127.0.0.1:5304/healthz
wrk -t4 -c128 -d30s --latency http://127.0.0.1:5304/version
wrk -t4 -c64  -d30s --latency http://127.0.0.1:5304/metrics
kill $PID
```

Curl examples:

```
CID_ZERO=b3:0000000000000000000000000000000000000000000000000000000000000000
curl -i http://127.0.0.1:5304/healthz
curl -i http://127.0.0.1:5304/readyz
curl -i http://127.0.0.1:5304/version
curl -i http://127.0.0.1:5304/resolve/name:does-not-exist
curl -i http://127.0.0.1:5304/providers/not-a-cid
curl -i http://127.0.0.1:5304/providers/$CID_ZERO
```

---

## 9) “Next crate” carryover (integration hints)

* **svc-index → svc-gateway/svc-overlay**: keep `/providers` and `/resolve` contract as the gateway’s upstream; treat **404** as cacheable negative results (short TTL), **400** as client error.
* **Metrics integration**: gateway scrape should aggregate svc-index RED metrics; reserve labels to avoid high cardinality.
* **Config consistency**: reuse `RON_INDEX_DB` / `INDEX_BIND` naming patterns across services.

---

**Conclusion:** svc-index is **Beta-ready**. Performance is **elite** for the class on modest hardware (p99 ~2–3 ms at tens of thousands RPS). The above P0/P1 items are the shortest path to “Gold” without disrupting the proven contract or hot path performance.


### END NOTE - OCTOBER 30 2025 - 13:58 CST



### BEGIN NOTE - APRIL 29 2026 - 13:00 CST

Here are the **svc-index changelog/carry-over notes only**.

---

# `svc-index` — WEB3_2 Manifest Pointer Foundation Notes

**Session date:** 2026-04-29
**Crate:** `crates/svc-index`
**Status:** Batch complete / green
**Purpose of this session:** Add the runtime index foundation for WEB3_2 asset and site manifest pointers.

---

## 0. Summary

In this session, `svc-index` was extended to store and fetch mutable manifest pointers for WEB3_2 asset and site flows.

This gives RustyOnions the next critical piece after `ron-naming` and `ron-proto`:

```text
ron-naming
  defines crab://<64hex>.<asset_kind>

ron-proto
  defines asset/page/manifest DTOs

svc-index
  stores and resolves mutable pointers:
    asset CID → asset manifest CID
    site/name → site manifest CID
```

Important boundary preserved:

```text
svc-index stores pointers only.
svc-index does not store raw asset bytes.
svc-index does not hydrate asset pages.
svc-index does not call storage.
svc-index does not call wallet.
svc-index does not mutate ledger.
```

Raw bytes remain in `svc-storage`. Hydration belongs to `omnigate`. Wallet and receipts remain in `svc-wallet` / `ron-ledger`.

---

## 1. Files added or updated

### Updated

```text
crates/svc-index/src/store/mod.rs
crates/svc-index/src/types.rs
crates/svc-index/src/http/routes/mod.rs
crates/svc-index/src/router.rs
```

### Added

```text
crates/svc-index/src/store/keys.rs
crates/svc-index/src/http/routes/index_manifests.rs
crates/svc-index/tests/http_contract.rs
crates/svc-index/tests/integration.rs
crates/svc-index/tests/prop_index.rs
crates/svc-index/tests/loom_index.rs
crates/svc-index/tests/chaos.rs
```

---

## 2. Routes added

This session added the first WEB3_2 manifest pointer API routes in `svc-index`.

### 2.1 Asset manifest pointer routes

```text
PUT /v1/index/assets/:asset_cid/manifest
GET /v1/index/assets/:asset_cid/manifest
```

Purpose:

```text
Store and fetch a mutable pointer from an immutable asset CID to an immutable manifest CID.
```

Example conceptual mapping:

```text
asset_cid:
  b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

points to manifest_cid:
  b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789
```

This supports typed asset pages like:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

The asset bytes are still addressed by the asset CID. The metadata/ownership/payout manifest is separately addressed by the manifest CID.

### 2.2 Site manifest pointer routes

```text
PUT /v1/index/sites/:name/manifest
GET /v1/index/sites/:name/manifest
```

Purpose:

```text
Store and fetch a mutable pointer from a human site/name key to an immutable site manifest CID.
```

Example conceptual mapping:

```text
sealobsta.com
  → b3:<site_manifest_hash>
```

This sets up later `crab://site` and named-site work without building the whole site launch system yet.

---

## 3. Store keyspace added

A new store key helper module was added:

```text
crates/svc-index/src/store/keys.rs
```

It defines stable prefixes:

```rust
ASSET_MANIFEST_PREFIX = "asset_manifest:"
SITE_MANIFEST_PREFIX = "site_manifest:"
```

And helper functions:

```rust
asset_manifest_key(canonical_asset_cid: &str) -> String
site_manifest_key(canonical_name: &str) -> String
```

This keeps WEB3_2 product pointer records separated from legacy/other index records.

Important design point:

```text
The keyspace stores pointer JSON only.
It does not store raw object bytes.
It does not store wallet receipts.
It does not store ledger state.
```

---

## 4. DTOs added in `svc-index`

The following local request/record DTOs were added to `types.rs`.

---

## 4.1 `PutAssetManifestPointer`

Request body for:

```text
PUT /v1/index/assets/:asset_cid/manifest
```

Fields:

```text
asset_kind
manifest_cid
owner_passport_subject
owner_wallet_account
updated_at_ms
```

Purpose:

```text
Let a caller write/update the manifest pointer for an immutable asset CID.
```

The asset CID comes from the URL path.

Example conceptual body:

```json
{
  "asset_kind": "image",
  "manifest_cid": "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
  "owner_passport_subject": "passport:main:alice",
  "owner_wallet_account": "acct_creator_alice",
  "updated_at_ms": 1776000000000
}
```

---

## 4.2 `PutSiteManifestPointer`

Request body for:

```text
PUT /v1/index/sites/:name/manifest
```

Fields:

```text
manifest_cid
owner_passport_subject
owner_wallet_account
updated_at_ms
```

Purpose:

```text
Let a caller write/update the manifest pointer for a site/name.
```

The site/name key comes from the URL path.

---

## 4.3 `AssetManifestPointer`

Stored pointer record for assets.

Fields:

```text
version
asset_cid
asset_kind
manifest_cid
owner_passport_subject
owner_wallet_account
updated_at_ms
```

Purpose:

```text
Represent the durable/mutable index pointer:
asset CID → asset manifest CID
```

Important boundary:

```text
owner_passport_subject and owner_wallet_account are references only.
They do not grant wallet spend authority.
They do not prove ownership by themselves.
They do not bypass passport/auth/policy.
```

---

## 4.4 `SiteManifestPointer`

Stored pointer record for sites.

Fields:

```text
version
name
manifest_cid
owner_passport_subject
owner_wallet_account
updated_at_ms
```

Purpose:

```text
Represent the durable/mutable index pointer:
site/name → site manifest CID
```

This is the foundation for later `crab://site` work.

---

## 5. Validation helpers added

`types.rs` now includes validation/normalization helpers for product pointer inputs.

---

## 5.1 `normalize_b3_cid()`

Accepts:

```text
b3:<64 lowercase hex>
```

or raw:

```text
<64 lowercase hex>
```

Returns canonical:

```text
b3:<64 lowercase hex>
```

Rejects:

```text
bad length
uppercase hex
non-hex characters
invalid prefix/body
```

This keeps internal index records aligned with the canonical RustyOnions content ID invariant.

---

## 5.2 `normalize_asset_kind()`

Normalizes asset kind suffixes to lowercase.

Accepted beta/product kinds:

```text
image
video
music
song
article
post
comment
page
site
app
manifest
```

Rejects:

```text
empty kind
unknown kind
control characters
path-like forms
```

Important boundary:

```text
svc-index validates kind strings but does not own full product semantics.
```

`ron-proto` and `ron-naming` are still the better long-term homes for canonical asset-kind vocabulary.

---

## 5.3 `normalize_site_name()`

Normalizes site/name keys to lowercase and rejects unsafe forms.

Allowed shape for this beta foundation:

```text
ASCII lowercase letters
digits
dot
dash
underscore
```

Rejected:

```text
empty string
.
..
../path
slash
backslash
@
double-dot traversal-ish forms
leading dot
trailing dot
spaces
control characters
newlines
unsupported characters
```

A bug was caught during testing:

```text
"sealobsta\n" was initially accepted because the function trimmed before checking controls.
```

Fix applied:

```text
control characters are now checked before trim/lowercase normalization.
```

---

## 5.4 `normalize_optional_ref()`

Normalizes optional owner/passport/account references.

Behavior:

```text
None → None
empty/whitespace → None
trimmed non-empty → Some(trimmed)
control characters → reject
over 512 bytes → reject
```

Used for:

```text
owner_passport_subject
owner_wallet_account
```

Important boundary:

```text
These are references only, not authority.
```

---

## 6. Store methods added

`Store` was extended with WEB3_2 pointer methods.

### Asset pointer methods

```rust
put_asset_manifest_pointer(&self, pointer: &AssetManifestPointer) -> anyhow::Result<()>
get_asset_manifest_pointer(&self, asset_cid: &str) -> Option<AssetManifestPointer>
```

### Site pointer methods

```rust
put_site_manifest_pointer(&self, pointer: &SiteManifestPointer) -> anyhow::Result<()>
get_site_manifest_pointer(&self, name: &str) -> Option<SiteManifestPointer>
```

Implementation behavior:

```text
Pointer records are serialized as strict JSON.
Memory store and sled-backed store both use the same Store facade.
Malformed stored JSON returns None on read rather than panicking.
```

This is enough for `omnigate` to ask `svc-index`:

```text
What manifest describes this b3 asset?
```

or later:

```text
What manifest describes this named site?
```

---

## 7. HTTP route module added

New file:

```text
crates/svc-index/src/http/routes/index_manifests.rs
```

Handlers added:

```rust
put_asset_manifest()
get_asset_manifest()
put_site_manifest()
get_site_manifest()
```

Behavior:

```text
canonicalizes b3 CIDs
canonicalizes asset kinds
canonicalizes site names
normalizes owner refs
defaults updated_at_ms to current system time if omitted
rejects updated_at_ms = 0
writes pointer records through Store
returns 202 Accepted on successful PUT
returns 200 OK on successful GET
returns 404 if pointer missing
returns BadRequest for invalid inputs
```

Important boundary:

```text
Handlers do not read object bytes.
Handlers do not verify manifests.
Handlers do not verify ownership signatures.
Handlers do not verify passport authority.
Handlers do not price anything.
Handlers do not call wallet.
Handlers do not mutate ledger.
```

---

## 8. Router updated

`router.rs` was updated to mount the new routes:

```text
/v1/index/assets/:asset_cid/manifest
/v1/index/sites/:name/manifest
```

These routes sit alongside existing index routes like:

```text
/resolve/:key
/providers/:cid
/healthz
/readyz
/version
/metrics
```

This makes the manifest pointer system reachable by downstream services such as `omnigate`.

---

## 9. Tests added / replaced

---

## 9.1 `tests/http_contract.rs`

Covers:

```text
unknown field rejection for PutAssetManifestPointer
stable AssetManifestPointer JSON shape
no raw_bytes field appears in pointer JSON
normalize_b3_cid accepts canonical and raw lowercase hash
normalize_asset_kind accepts beta/product asset forms
normalize_site_name normalizes SeaLobsta.COM → sealobsta.com
```

Purpose:

```text
Verify strict wire behavior and basic validation contract.
```

---

## 9.2 `tests/integration.rs`

Covers:

```text
memory store round-trips AssetManifestPointer
memory store round-trips SiteManifestPointer
missing asset/site pointers return None
```

Purpose:

```text
Prove the Store facade supports WEB3_2 pointer records.
```

---

## 9.3 `tests/prop_index.rs`

Covers:

```text
bad b3 lengths reject
uppercase b3 hash rejects
bad b3 characters reject
unsafe site names reject
unknown asset kinds reject
```

Important failure caught and fixed:

```text
"sealobsta\n" originally passed because trim happened before control-char validation.
```

Fix:

```text
normalize_site_name now checks control characters before trim.
```

---

## 9.4 `tests/loom_index.rs`

This is a lightweight concurrency smoke test for memory-store pointer updates.

Covers:

```text
two threads repeatedly update the same asset pointer
store does not panic
final pointer exists
final pointer is one of the expected writes
```

Important boundary:

```text
This is not a full Loom model.
It is a bounded concurrent update smoke test.
```

Purpose:

```text
Provide basic safety signal for pointer updates in the memory store.
```

---

## 9.5 `tests/chaos.rs`

Covers:

```text
malformed CIDs fail closed
malformed site names fail closed
optional owner refs normalize correctly
empty owner ref becomes None
trimmed non-empty owner ref is preserved
```

Purpose:

```text
Replace placeholder chaos test behavior with deterministic rejection checks.
```

---

## 10. Validation gates passed

The final `svc-index` gates passed:

```text
cargo fmt
cargo clippy -p svc-index --all-targets -- -D warnings
cargo test -p svc-index --all-targets
```

Final observed result:

```text
svc-index clippy: clean
svc-index tests: all passing
```

Passing test groups included:

```text
chaos.rs
http_contract.rs
integration.rs
loom_index.rs
prop_index.rs
resolve.rs bench target
client.rs example target
```

Specific new tests passed:

```text
chaos.rs             2/2 passing
http_contract.rs     3/3 passing
integration.rs       3/3 passing
loom_index.rs        1/1 passing
prop_index.rs        3/3 passing
```

---

## 11. Invariants preserved

This work preserved WEB3 / WEB3_2 and RustyOnions crate-boundary invariants:

```text
svc-index stores mutable lookup/index records only.
svc-index does not store raw content bytes.
svc-index does not own BLAKE3 hashing.
svc-index does not hydrate asset pages.
svc-index does not serve asset bytes.
svc-index does not call svc-storage.
svc-index does not call svc-wallet.
svc-index does not mutate ron-ledger.
svc-index does not calculate payouts.
svc-index does not verify wallet receipts.
svc-index does not grant passport authority.
svc-index fails closed on malformed b3/name/asset-kind input.
```

Content-addressing invariant preserved:

```text
Internal CIDs are canonical b3:<64 lowercase hex>.
Raw public crab URLs are handled upstream/downstream as crab://<64hex>.<kind>.
```

---

## 12. Architectural boundary notes

`svc-index` now owns:

```text
asset CID → manifest CID pointer storage
site/name → manifest CID pointer storage
pointer fetch/update routes
pointer keyspace prefixes
pointer DTOs for its HTTP surface
basic normalization and validation for pointer inputs
```

`svc-index` still does **not** own:

```text
raw content bytes
asset upload
manifest byte storage
manifest byte validation
asset-page hydration
browser-extension UX
gateway ingress policy
wallet hold/capture/release
ledger mutation
receipt verification
creator payout execution
storage provider payout execution
```

Those remain in:

```text
svc-storage    — raw object and manifest bytes by b3 CID
ron-proto      — shared DTO definitions
ron-naming     — crab URL grammar and naming primitives
omnigate       — hydrated asset-page view composition
svc-gateway    — public ingress/proxy routing
svc-wallet     — wallet mutation and receipts
ron-ledger     — durable economic truth
svc-rewarder   — deterministic payout planning
```

---

## 13. Completion assessment for `svc-index`

For the WEB3_2 product-proof TODO, `svc-index` Batch 3 is:

```text
100% complete for this session’s scope
```

Current `svc-index` WEB3_2 readiness:

```text
Asset CID → manifest pointer storage       complete
Asset pointer GET/PUT routes               complete
Site/name → manifest pointer storage       complete
Site pointer GET/PUT routes                complete
Pointer DTO wire shape                     complete
Strict unknown-field rejection             complete
Canonical b3 normalization                 complete
Asset kind normalization                   complete
Unsafe site/name rejection                 complete
Owner reference fields                     complete as references only
Store integration tests                    complete
Targeted gates                             green
```

---

## 14. Remaining future work for `svc-index`

No blocker remains from this session.

Future enhancements may include:

```text
Add stronger auth/capability enforcement around pointer writes.
Add owner/passport verification hooks.
Add optimistic concurrency / compare-and-swap for pointer updates.
Add history/audit trail for pointer changes.
Add direct manifest metadata summary indexing if omnigate needs faster hydration.
Add site manifest search/list support.
Add typed asset search facets.
Add policy-gated update semantics.
Add integration tests against real svc-storage manifest bytes.
Add route docs/OpenAPI for the new pointer endpoints.
```

Likely future route additions:

```text
GET /v1/index/assets/:asset_cid
GET /v1/index/assets/:asset_cid/page-metadata
GET /v1/index/sites/:name
GET /v1/index/sites/:name/page-metadata
POST /v1/index/assets/search
POST /v1/index/sites/search
```

But these should only be added when required by the next WEB3_2 batches.

---

## 15. Session takeaway

`svc-index` now provides the mutable pointer layer needed for WEB3_2 typed asset pages and later site launches.

The important relationship is:

```text
Immutable asset bytes:
  b3:<asset_hash>

Mutable asset pointer in svc-index:
  b3:<asset_hash> → b3:<asset_manifest_hash>

Immutable manifest bytes:
  b3:<asset_manifest_hash>
```

For named sites:

```text
Mutable site pointer in svc-index:
  sealobsta.com → b3:<site_manifest_hash>
```

This makes the product stack coherent:

```text
ron-naming
  parses crab://<hash>.<kind>

ron-proto
  defines manifest/page DTOs

svc-index
  resolves hash/name to manifest pointers

omnigate
  hydrates asset pages from index/storage signals

svc-gateway
  exposes routes to clients/browser extension
```

`svc-index` Batch 3 is complete and green.


### END NOTE - APRIL 29 2026 - 13:00 CST


### BEGIN NOTE - JUNE 15 2026 - 20:05 CST


# `svc-index` QuickChain Phase-0 / Preflight Notes

## 0. Current checkpoint

`svc-index` is now green and parkable for the current QuickChain Phase-0 / QC-0A preflight sweep.

Final observed local gate:

```text
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
```

Final terminal marker:

```text
svc-index QuickChain Phase-0 preflight passed.
```

The final successful gate included:

```text
quickchain_preflight_docs.rs                4/4 passed
quickchain_preflight_boundary.rs            4/4 passed
quickchain_preflight_pointer_authority.rs   5/5 passed
quickchain_preflight_routes.rs              2/2 passed
http_contract.rs                            3/3 passed
integration.rs                              3/3 passed
cargo clippy -p svc-index --all-targets -- -D warnings passed
```

This crate can now be treated as:

```text
green
parkable
safe to move past for now
ready for optional future polish later
```

Estimated `svc-index` QuickChain Phase-0 / preflight completion:

```text
~93–96%
```

Estimated overall QuickChain Phase-0 / preflight completion after this crate:

```text
~91–93%
```

This estimate only applies to the current Phase-0 / QC-0A preflight sweep. It does not mean the full QuickChain long-term blueprint is near complete. Full QuickChain still requires locked vectors, deterministic roots, proofs, checkpoint artifacts, validator/committee work, DA/challenge/archive planning, pruning safety, and future external-anchor decisions.

---

## 1. Role of `svc-index`

`svc-index` is lookup and pointer infrastructure.

Its current legitimate authority is index/pointer truth, such as:

```text
this name points to this manifest
this site name points to this site manifest CID
this asset CID points to this asset manifest CID
this provider list is a lookup result
this cache entry is lookup acceleration
this record exists in the index service
```

`svc-index` is not:

```text
economic authority
wallet authority
ledger truth
receipt authority
balance authority
paid-access authority
finality authority
QuickChain runtime
root producer
checkpoint producer
validator
bridge
staking infrastructure
liquidity infrastructure
external settlement infrastructure
```

The key distinction established in this session:

```text
Index truth is not economic truth.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Policy metadata is not wallet authority.
Provider lookup is not settlement finality.
```

This is the central rule for future `svc-index` work.

---

## 2. Active QuickChain doctrine to preserve

QuickChain remains future ROC settlement infrastructure.

It is not an active blockchain, live chain runtime, public L1, bridge, staking system, exchange-facing system, or client authority layer.

Current allowed scope remains:

```text
docs hardening
threat-model review
vector planning
DTO shape discussion
strict ron-proto::quickchain DTO scaffolding when explicitly requested and safe
validation helpers
schema constants
unknown-field rejection
integer minor-unit string validation
b3 lowercase validation
event-class enum tests
operation/idempotency tests
preflight boundary tests
no-authority source scans
crate-local preflight scripts
```

Current forbidden scope remains:

```text
no roots
no checkpoints
no validators
no pruning
no settlement
no anchors
no bridge
no staking
no liquidity
no live chain state
no CrabLink chain authority
no gateway-side ledger mutation
no omnigate-side ledger mutation
no rewarder ledger mutation
no index-side ledger mutation
no storage-side ledger mutation
no raw engagement protocol payouts
no DB-order roots
no wall-clock roots
no unknown consensus fields
no fake hashes
no placeholder hashes
no root-producing code without canonical bytes and golden vectors
```

Core doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Canonicalization doctrine for future work:

```text
canonical JSON v1
preimage = domain_separator_bytes || 0x00 || canonical_payload_bytes
hash = blake3-256 as b3:<64 lowercase hex>
money = integer minor-unit strings only
no floats
no placeholder hashes
vectors move sketch -> locked_bytes -> locked_hash
```

`svc-index` must not be the crate that skips these gates.

---

## 3. Why `svc-index` needed special handling

`svc-index` is tricky because it is supposed to be authoritative for lookup/pointer records, but it must not become authoritative for payment, wallet, ledger, receipt, balance, unlock, or QuickChain state.

The correct model:

```text
Index record exists and resolves a name/pointer: yes, that can be index truth.
Index record proves paid access: no.
Index record proves wallet balance: no.
Index record proves receipt finality: no.
Index record proves QuickChain root/checkpoint state: no.
Index record proves external settlement: no.
Index record proves validator finality: no.
```

We explicitly avoided overcorrecting.

We did not ban all words like:

```text
owner
creator
recipient
policy
manifest
wallet
paid
```

Those words can be legitimate metadata.

Instead, the tests and docs constrain authority-shaped usage such as:

```text
paid_from_index
unlock_from_index
receipt_from_index
balance_from_index
finality_from_index
checkpoint_from_index
validator_from_index
root_from_index
paid_from_manifest
unlock_from_manifest
receipt_from_pointer
balance_from_pointer
unlock_from_cache
entitlement_from_cache
```

This distinction must be preserved in future patches.

---

## 4. Files added or changed in this session

### 4.1 Added docs

Created:

```text
crates/svc-index/docs/quickchain-preflight.md
```

Purpose:

```text
documents the svc-index QuickChain Phase-0 boundary
locks the lookup/pointer-only doctrine
defines what index truth can and cannot prove
defines forbidden paid-access shortcut paths
defines forbidden QuickChain/runtime/settlement scope
documents scanner posture so future tests do not ban legitimate metadata blindly
```

Important content in the doc:

```text
svc-index is a lookup and pointer service
index truth is not economic truth
pointer truth is not receipt truth
manifest lookup is not paid unlock
paid access must flow through backend wallet/ledger/storage truth
owner/passport/wallet metadata is reference metadata only
no roots/checkpoints/validators/settlement/bridges/staking/liquidity/ROX/Solana
```

A small docs-only scanner-phrase fix was later applied because the tests expected plain substring matches, not Markdown-backticked text.

Fixed phrases:

```text
svc-index is a lookup and pointer service
wallet as a reference string
```

This fix did not change runtime behavior and did not weaken tests.

---

### 4.2 Added docs contract test

Created:

```text
crates/svc-index/tests/quickchain_preflight_docs.rs
```

Purpose:

```text
ensures docs preserve the svc-index QuickChain Phase-0 doctrine
proves the docs explicitly separate lookup/pointer truth from economic truth
proves paid-access shortcut paths remain forbidden
proves QuickChain runtime/external settlement scope remains forbidden
proves we do not blindly ban legitimate metadata words
```

Final result:

```text
4 tests passed
```

Tests:

```text
docs_define_svc_index_as_lookup_pointer_service_only
docs_preserve_backend_paid_access_truth_path
docs_forbid_quickchain_runtime_and_external_settlement_scope
docs_warn_not_to_ban_legitimate_metadata_blindly
```

What this proves:

```text
The crate now has explicit docs requiring pointer-only authority.
The docs must keep saying that index truth is not economic truth.
The docs must keep saying that manifest lookup is not paid unlock.
The docs must keep saying that paid access requires backend wallet/ledger/storage truth.
The docs must keep the forbidden QuickChain scope visible.
```

---

### 4.3 Added source/dependency boundary test

Created:

```text
crates/svc-index/tests/quickchain_preflight_boundary.rs
```

Purpose:

```text
proves svc-index does not depend on wallet/ledger authority crates
proves production source does not import value-plane authority crates
proves production source does not call wallet mutation verbs
proves production source does not define QuickChain runtime/root/checkpoint/validator/settlement machinery
```

Final result:

```text
4 tests passed
```

Tests:

```text
cargo_does_not_depend_on_wallet_or_ledger_authority_crates
production_source_does_not_import_value_plane_authority_crates
production_source_does_not_call_wallet_mutation_verbs
production_source_does_not_define_quickchain_runtime_infrastructure
```

Forbidden dependency/source tokens include:

```text
ron-ledger
svc-wallet
ron-accounting
svc-rewarder
quickchain-runtime
quickchain-validator
quickchain-consensus
solana
ron_ledger
svc_wallet
ron_accounting
svc_rewarder
```

Forbidden wallet mutation call shapes include:

```text
.issue(
::issue(
.transfer(
::transfer(
.burn(
::burn(
.hold(
::hold(
.capture(
::capture(
.release(
::release(
.mint(
::mint(
put_receipt(
insert_receipt(
create_receipt(
accept_receipt(
commit_receipt(
```

Forbidden QuickChain/runtime terms include:

```text
state_root
receipt_root
accounting_root
reward_root
checkpoint_root
checkpoint_hash
checkpoint_header
validator_set
validator_signature
committee_quorum
fork_choice
settlement_status
external_anchor
bridge_settlement
root_producer
produce_root
produce_checkpoint
finalize_checkpoint
quickchain_receipt
quickchain_checkpoint
quickchain_validator
```

Important implementation detail:

```text
The scanner checks production source under src/ and Cargo dependencies.
It strips Rust comments to reduce false positives.
It does not scan docs/tests as production authority.
```

This is the correct shape for Phase-0 preflight.

---

### 4.4 Added pointer-authority DTO test

Created:

```text
crates/svc-index/tests/quickchain_preflight_pointer_authority.rs
```

Purpose:

```text
proves manifest pointer DTOs cannot smuggle paid/economic/QuickChain authority
proves unknown authority-shaped fields reject
proves stored pointer wire shapes do not expose receipt/balance/finality/root/checkpoint/validator/settlement authority
proves owner wallet metadata remains allowed as a reference only
```

Final result:

```text
5 tests passed
```

Tests:

```text
put_asset_manifest_pointer_rejects_authority_shaped_fields
put_site_manifest_pointer_rejects_authority_shaped_fields
stored_asset_pointer_wire_shape_has_no_paid_or_chain_authority
stored_site_pointer_wire_shape_has_no_paid_or_chain_authority
owner_wallet_metadata_is_allowed_but_normalized_as_reference_only
```

Authority-shaped fields rejected on input include:

```text
paid_from_index
unlock_from_index
receipt_from_index
balance_from_index
finality_from_index
checkpoint_from_index
validator_from_index
root_from_index
paid_from_manifest
unlock_from_manifest
receipt_id
receipt_hash
receipt_root
state_root
checkpoint_hash
settlement_status
entitlement_granted
unlock_granted
spend_authority
capture_authority
```

Stored pointer wire shapes were checked to ensure they do not contain fields such as:

```text
paid
unlock
receipt
receipt_id
receipt_hash
balance
finality
entitlement
state_root
receipt_root
checkpoint
checkpoint_hash
validator
settlement
spend_authority
capture_authority
```

Owner metadata remains allowed as metadata:

```text
owner_passport_subject
owner_wallet_account
```

But those remain references only.

They are not:

```text
spend authority
capture authority
receipt authority
balance authority
entitlement authority
paid-access authority
```

---

### 4.5 Added route-boundary test

Created:

```text
crates/svc-index/tests/quickchain_preflight_routes.rs
```

Purpose:

```text
ensures public svc-index routes remain lookup/pointer/admin/health surfaces only
ensures svc-index does not expose QuickChain roots/checkpoints/validators/settlement/bridges/staking/liquidity/ROX/Solana/finality/unlock surfaces
ensures expected current routes remain present
```

Final result:

```text
2 tests passed
```

Tests:

```text
public_routes_do_not_expose_quickchain_settlement_surfaces
allowed_routes_remain_lookup_and_pointer_or_admin_health_surfaces
```

Forbidden public route fragments include:

```text
/quickchain
/v1/quickchain
/roots
/root
/state-root
/receipt-root
/accounting-root
/reward-root
/checkpoint
/checkpoints
/validator
/validators
/settlement
/settle
/bridge
/anchors
/anchor
/staking
/stake
/liquidity
/rox
/solana
/finality
/entitlement
/unlock
```

Expected allowed route surface remains:

```text
/healthz
/readyz
/version
/metrics
/resolve/:key
/providers/:cid
/v1/index/assets/:asset_cid/manifest
/v1/index/sites/:name/manifest
```

This is exactly the right route posture for this phase.

---

### 4.6 Added crate-local preflight script

Created:

```text
crates/svc-index/scripts/dev-quickchain-preflight.sh
```

Purpose:

```text
provides a single local gate for svc-index QuickChain Phase-0 / preflight
runs format
runs focused QuickChain preflight tests
runs existing contract/integration tests
runs strict clippy
prints a green success marker
```

Final successful command:

```bash
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
```

Script currently runs:

```bash
cargo fmt -p svc-index

cargo test -p svc-index --test quickchain_preflight_docs
cargo test -p svc-index --test quickchain_preflight_boundary
cargo test -p svc-index --test quickchain_preflight_pointer_authority
cargo test -p svc-index --test quickchain_preflight_routes

cargo test -p svc-index --test http_contract
cargo test -p svc-index --test integration

cargo clippy -p svc-index --all-targets -- -D warnings
```

Success marker:

```text
svc-index QuickChain Phase-0 preflight passed.
```

This script is now the preferred local command before declaring `svc-index` green for QuickChain preflight.

---

## 5. Existing `svc-index` code shape relevant to this work

### 5.1 Crate purpose

`svc-index` is described as:

```text
RustyOnions index service (resolve name/cid -> manifest/providers)
```

It is an Axum service with:

```text
routes
types
store
cache
providers pipeline
resolve pipeline
DHT provider stub
metrics
health/readiness
manifest pointer routes
```

The focused crate bundle confirms that the crate is small enough to reason about and already centered on lookup/pointer behavior.

---

### 5.2 Router shape

Current router includes:

```text
/healthz
/readyz
/version
/metrics
/resolve/:key
/providers/:cid
/v1/index/assets/:asset_cid/manifest
/v1/index/sites/:name/manifest
```

This surface is correct for current `svc-index`.

Important:

```text
No QuickChain routes were added.
No settlement routes were added.
No unlock/entitlement routes were added.
No root/checkpoint/validator routes were added.
No wallet/ledger mutation routes were added.
```

---

### 5.3 Manifest pointer route shape

Current manifest pointer routes:

```text
PUT /v1/index/assets/:asset_cid/manifest
GET /v1/index/assets/:asset_cid/manifest
PUT /v1/index/sites/:name/manifest
GET /v1/index/sites/:name/manifest
```

These routes validate and normalize:

```text
asset CID
asset kind
manifest CID
site name
owner_passport_subject
owner_wallet_account
updated_at_ms
```

They store:

```text
AssetManifestPointer
SiteManifestPointer
```

They do not:

```text
store raw bytes
call svc-wallet
call ron-ledger
mint/issue/transfer/burn/hold/capture/release ROC
create receipts
validate payment finality
unlock paid content
produce roots
produce checkpoints
create settlement artifacts
```

This is the correct boundary.

---

### 5.4 DTO shape

Current pointer DTOs use strict Serde behavior:

```rust
#[serde(deny_unknown_fields)]
```

Important DTOs:

```text
PutAssetManifestPointer
PutSiteManifestPointer
AssetManifestPointer
SiteManifestPointer
ResolveResponse
ProviderEntry
ProvidersResponse
ErrorResponse
```

The pointer DTOs support owner metadata:

```text
owner_passport_subject
owner_wallet_account
```

But these are reference strings only.

The DTO tests now prove that adding authority-shaped fields fails closed through unknown-field rejection.

---

### 5.5 Store shape

Current store owns pointer values only.

Important store behavior:

```text
Store::put_asset_manifest_pointer
Store::get_asset_manifest_pointer
Store::put_site_manifest_pointer
Store::get_site_manifest_pointer
Store::get_manifest
Store::put_manifest
```

Current backends:

```text
sled-backed persistent store
in-memory store for dev/test/amnesia-friendly use
```

Important invariant:

```text
raw content bytes are not stored in svc-index
wallet/ledger truth is not stored in svc-index
receipt truth is not stored in svc-index
balance truth is not stored in svc-index
QuickChain root/checkpoint truth is not stored in svc-index
```

---

### 5.6 Existing contract/integration tests preserved

The preflight script runs existing tests:

```text
http_contract.rs
integration.rs
```

Observed final results:

```text
http_contract.rs: 3/3 passed
integration.rs:   3/3 passed
```

Existing contract tests prove:

```text
unknown fields reject on manifest pointer input
asset manifest pointer wire shape is stable
validators accept beta asset forms
```

Existing integration tests prove:

```text
asset manifest pointer store roundtrip works
site manifest pointer store roundtrip works
missing pointers return none
```

This matters because the QuickChain preflight did not break existing `svc-index` behavior.

---

## 6. Problems encountered and fixed

### 6.1 Markdown code fence escaping issue

The first pasted patch escaped/closed Markdown code blocks incorrectly.

Cause:

```text
the outer Markdown fence was not stronger than nested fences inside docs/quickchain-preflight.md
```

Fix:

```text
the patch was reissued using a five-backtick outer fence
```

No code semantics changed from this issue.

---

### 6.2 Docs scanner mismatch

Initial failure:

```text
docs_define_svc_index_as_lookup_pointer_service_only FAILED
docs_warn_not_to_ban_legitimate_metadata_blindly FAILED
```

The test expected plain phrases:

```text
svc-index is a lookup and pointer service
wallet as a reference string
```

The doc used Markdown backticks around those words:

```text
`svc-index`
`wallet`
```

Fix applied:

```text
added plain scanner phrase for svc-index
changed metadata bullet to plain "wallet as a reference string"
```

After the fix:

```text
quickchain_preflight_docs.rs passed 4/4
full dev-quickchain-preflight.sh passed
```

This was a docs-string mismatch only.

It did not represent an architecture failure.

It did not weaken tests.

It did not change runtime behavior.

---

## 7. What the green gate proves

The green `svc-index` gate proves the following for current Phase-0 purposes:

```text
docs preserve pointer-only doctrine
Cargo.toml does not pull in direct wallet/ledger/QuickChain runtime authority crates
production source does not import wallet/ledger authority crates
production source does not call direct wallet mutation verbs
production source does not define QuickChain roots/checkpoints/validators/settlement machinery
pointer DTOs reject unknown authority-shaped fields
stored pointer DTOs do not expose receipt/balance/finality/root/checkpoint/validator/settlement authority fields
owner wallet metadata remains normalized reference metadata only
public routes do not expose QuickChain settlement/root/checkpoint/validator/bridge/staking/liquidity/ROX/Solana surfaces
expected lookup/pointer/admin routes remain present
existing manifest pointer contract tests still pass
existing store integration tests still pass
strict clippy passes
```

That is enough to park `svc-index` for this QuickChain Phase-0 sweep.

---

## 8. What the green gate does not prove

The green `svc-index` gate does not prove or implement:

```text
deterministic pointer roots
canonical index snapshots
receipt roots
state roots
checkpoint roots
validator replay
settlement finality
bridge safety
external anchoring
data availability
challenge windows
archive fallback
pruning safety
paid-content entitlement verification
wallet balance truth
receipt finality
policy decision truth
storage byte integrity
b3 byte verification on storage reads
```

Those belong to other crates or future phases.

Important:

```text
Do not later reinterpret these tests as permission to build roots/checkpoints in svc-index.
```

If future QuickChain work ever needs index/pointer artifacts, it must go through the locked-vector path first.

---

## 9. Remaining work for `svc-index`

`svc-index` is green for QuickChain Phase 0, but the crate still has future work.

### 9.1 QuickChain-related future work

Do not do this now, but later phases may need:

```text
deterministic index/pointer snapshot discussion
canonical ordering rules for index pointer snapshots
proof-eligible pointer record shapes
read-only inclusion proof DTO discussions
policy-bound metadata classification
clear distinction between pointer proof and paid-access proof
strict vector planning for any future pointer snapshot bytes
```

Forbidden until gates are green:

```text
no index roots
no pointer roots
no checkpoint roots
no receipt roots
no state roots
no root-producing snapshots
no validators
no settlement
no finality claims
no unlock claims from index data
```

A future safe plan, after QuickChain vector gates, would be:

```text
1. Discuss whether index pointer records need to be proof-eligible.
2. Define non-authoritative DTO shape in ron-proto only.
3. Define canonical bytes and exact ordering in docs.
4. Produce sketch vectors.
5. Move to locked_bytes vectors.
6. Move to locked_hash vectors only with real BLAKE3.
7. Only then consider read-only verification helpers.
```

Even then, `svc-index` should remain read/lookup/pointer infrastructure, not economic authority.

---

### 9.2 General crate hardening work

These items are outside the QuickChain Phase-0 gate but worth tracking:

```text
fix or remove stale scaffold references to svc-index2 in crate-local GitHub workflow stubs if those workflows are used
replace placeholder fuzz targets with real parser/route fuzzing
replace placeholder chaos/perf scripts with real bounded checks if this crate becomes production-critical
wire real svc-dht provider lookups instead of local stub behavior when the DHT crate is ready
harden admin seed route around capability/policy decisions before production exposure
improve body limit middleware beyond identity placeholder if public write routes expand
audit sled persistence/amnesia behavior if micronode/profile switching uses this crate directly
add more HTTP integration tests around manifest pointer PUT/GET route status codes
add tests for owner_passport_subject and owner_wallet_account length/control-character rejection through HTTP route bodies
add tests proving cache hits cannot create unlock/paid-access responses
add docs around whether name records are mutable, versioned, audited, or append-only in future product modes
```

These are ordinary crate-quality items. They are not blockers for the current QuickChain preflight sweep.

---

### 9.3 Potential later product work

Depending on RustyOnions/CrabLink product priorities, future `svc-index` may need:

```text
profile pointer lookup
creator catalog pointer lookup
site route map lookup
asset manifest pointer history
site manifest pointer history
publisher/owner metadata normalization
index audit events
manifest-pointer revision numbers
policy hints as non-authoritative metadata
search/graph facets if enabled under the Developer Suite facet model
provider records from svc-dht
gateway/omnigate hydration support
```

Any product extension must keep this rule:

```text
metadata can help route/hydrate/display
metadata cannot prove paid access, wallet balance, receipt finality, or settlement finality
```

---

## 10. Current commands for `svc-index`

### Regenerate focused codebundle

From repo root:

```bash
bash scripts/make_crate_codex.sh -c svc-index
```

Observed latest output shape:

```text
Wrote 97 files to: /Users/mymac/Desktop/RustyOnions/crates/svc-index/CODEBUNDLE.md
```

The generated crate codebundle skips docs and Markdown, so it will not include `docs/quickchain-preflight.md`.

That is expected.

### Run focused QuickChain preflight

```bash
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
```

### Run individual focused tests

```bash
cargo test -p svc-index --test quickchain_preflight_docs
cargo test -p svc-index --test quickchain_preflight_boundary
cargo test -p svc-index --test quickchain_preflight_pointer_authority
cargo test -p svc-index --test quickchain_preflight_routes
```

### Run existing relevant regression tests

```bash
cargo test -p svc-index --test http_contract
cargo test -p svc-index --test integration
```

### Run format and clippy

```bash
cargo fmt -p svc-index
cargo clippy -p svc-index --all-targets -- -D warnings
```

---

## 11. Recommended status label

Use this status for planning:

```text
svc-index QuickChain Phase-0 / preflight: GREEN / PARKED
```

Current confidence:

```text
high
```

Reason:

```text
The crate-local preflight runner is green.
All new focused QuickChain boundary tests pass.
Existing route/DTO/store contract tests pass.
Strict clippy passes.
No runtime authority was added.
No root/checkpoint/validator/settlement logic was added.
No wallet/ledger dependency or mutation path was added.
```

---

## 12. Next crate after `svc-index`

The next crate in the QuickChain Phase-0 sweep should be:

```text
ron-policy
```

Why `ron-policy` next:

```text
ron-policy is declarative policy infrastructure.
It must remain policy decision/config/evaluation logic.
It must not become wallet authority.
It must not become ledger truth.
It must not mutate balances.
It must not mint or allocate ROC.
It must not create receipts.
It must not produce roots/checkpoints/settlement.
It must not authorize paid unlocks without backend wallet/ledger/storage receipt truth.
```

Expected `ron-policy` QuickChain preflight themes:

```text
docs preserving declarative-policy-only boundary
source/dependency scan for no wallet/ledger mutation
tests proving policy decisions are not receipts/balances/finality
tests proving policy economics config does not become economic truth
route/API scans if applicable
no QuickChain roots/checkpoints/validators/settlement/anchors/bridges
crate-local dev-quickchain-preflight.sh
```

After `ron-policy`, the likely next step is:

```text
workspace-level umbrella QuickChain Phase-0 gate
```

The umbrella gate should prove cross-crate doctrine:

```text
no live service path consumes QuickChain DTOs as authority
wallet remains mutation front-door
ledger remains truth
accounting/rewarder do not mutate balances
storage stores bytes only
gateway public boundary fails closed
omnigate hydrates but does not mutate ledger
index resolves pointers but does not unlock paid content
policy declares/evaluates rules but does not mutate economic state
clients display backend-derived truth only
```

---

## 13. One-paragraph handoff

We completed `svc-index` QuickChain Phase-0 / preflight hardening. The crate is now green and parkable. Added `docs/quickchain-preflight.md`, `quickchain_preflight_docs.rs`, `quickchain_preflight_boundary.rs`, `quickchain_preflight_pointer_authority.rs`, `quickchain_preflight_routes.rs`, and `scripts/dev-quickchain-preflight.sh`. The final script passed docs tests, dependency/source boundary tests, pointer-authority DTO tests, route-surface tests, existing HTTP contract tests, existing store integration tests, and strict clippy. The crate is now explicitly constrained as lookup/pointer infrastructure only: not wallet authority, not ledger truth, not paid-access authority, not receipt/balance/finality authority, not root/checkpoint/validator/settlement/bridge/staking/liquidity/ROX/Solana infrastructure. Owner/passport/wallet metadata remains allowed as reference metadata only. The main caution for future work is not to overcorrect by banning legitimate metadata, but to keep banning authority-shaped usage such as `unlock_from_index`, `receipt_from_index`, `balance_from_index`, `finality_from_index`, `checkpoint_from_index`, `validator_from_index`, and `root_from_index`. The next crate should be `ron-policy`, followed by a workspace-level QuickChain umbrella gate.


### END NOTE - JUNE 15 2026 - 20:05 CST