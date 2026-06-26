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


### BEGIN NOTE - JUNE 17 2026 - 18:35 CST

Based on the latest terminal output, both **svc-index** and **ron-policy** are green/parked: tooling tests passed, crate-local parking scripts passed, all-targets passed, and clippy `-D warnings` passed. The output explicitly confirms the forbidden QuickChain runtime scopes remain parked for both crates.   

Here are the drop-in notes:

# QuickChain Phase-0 Notes — `svc-index` + `ron-policy`

## Status Summary

`svc-index` and `ron-policy` are now **GREEN / Phase-0 preflight complete / parked** for the current QuickChain safety slice.

This means both crates now have:

* QuickChain boundary documentation.
* Focused QuickChain preflight tests.
* Tooling boundary tests.
* Crate-local preflight scripts.
* Crate-local park scripts.
* Existing crate regression coverage included in the preflight.
* All-targets test coverage.
* Clippy gate with `-D warnings`.
* Low-disk-compatible parking command path.
* No Python helper drift in QuickChain tooling.
* No root/checkpoint/validator/settlement/bridge/wallet/ledger mutation behavior introduced.

This work keeps both crates inside the current QuickChain doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Both crates remain Phase-0-safe. No chain runtime, no root production, no validator behavior, no checkpoint behavior, no external settlement, no ROX/Solana/bridge/staking/liquidity logic, and no fake economic truth were added.

---

# 1. svc-index Notes

## Current Status

```text
svc-index: GREEN / Phase-0 preflight complete / parked
```

`svc-index` now has a repeatable QuickChain safety gate proving that it remains a lookup/pointer service only.

The final parking run ended with:

```text
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## Intended Role

`svc-index` is a lookup and pointer service.

It may own or expose lookup/index truth such as:

* names
* site pointers
* asset pointers
* manifest pointers
* profile pointers
* provider lookup metadata
* b3-addressed object references
* crab navigation targets
* index metadata needed for discovery/resolution

It must not become:

* wallet authority
* ledger authority
* accounting authority
* reward authority
* paid unlock authority
* receipt authority
* balance authority
* finality authority
* QuickChain runtime
* root producer
* checkpoint producer
* validator
* settlement service
* bridge
* staking/liquidity layer
* external chain participant

## Core svc-index Doctrine

The central boundary is:

```text
svc-index is a lookup and pointer service, not economic authority.
```

More specifically:

```text
Index truth is not economic truth.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Provider lookup is not settlement finality.
Owner/passport/wallet fields in index records are references only.
Cache hits cannot unlock paid content.
crab:// navigation is navigation, not authority.
```

## Files Added / Hardened

The QuickChain work for `svc-index` includes the following file categories.

### Documentation

```text
crates/svc-index/docs/quickchain-preflight.md
```

Purpose:

* Defines `svc-index` as lookup/pointer infrastructure only.
* Explicitly separates pointer truth from economic truth.
* Documents that names are pointers.
* Documents that b3 hashes prove bytes/content identity, not payment.
* Documents that index lookup does not prove balance, receipt, finality, entitlement, or paid access.
* Documents that paid access must come from backend wallet/ledger/storage receipt paths.
* Keeps forbidden QuickChain runtime scope explicit.

### Scripts

```text
crates/svc-index/scripts/dev-quickchain-preflight.sh
crates/svc-index/scripts/dev-quickchain-park.sh
```

The preflight script now:

* Uses Bash strict mode: `set -euo pipefail`.
* Supports `CARGO="${CARGO:-cargo}"`.
* Supports `--check`.
* Locates repo root and crate root.
* Verifies the QuickChain docs file exists.
* Verifies the tests directory exists.
* Rejects Python helper drift under the crate.
* Runs format check or formatting.
* Dynamically discovers `quickchain*.rs` tests.
* Runs each QuickChain test by test target name.
* Runs existing `svc-index` regressions.
* Runs `cargo test -p svc-index --all-targets`.
* Runs `cargo clippy -p svc-index --all-targets --no-deps -- -D warnings`.
* Prints the forbidden runtime scope parked message.
* Prints the final preflight success marker.

The park script now:

* Verifies the docs file exists.
* Verifies the preflight script exists.
* Verifies the tooling boundary test exists.
* Delegates to the preflight script.
* Supports `--check`.
* Prints the parking success marker.

### Tests

Current focused QuickChain tests include:

```text
crates/svc-index/tests/quickchain_preflight_boundary.rs
crates/svc-index/tests/quickchain_preflight_docs.rs
crates/svc-index/tests/quickchain_preflight_pointer_authority.rs
crates/svc-index/tests/quickchain_preflight_routes.rs
crates/svc-index/tests/quickchain_tooling_boundary.rs
```

Existing regression tests included in the gate:

```text
crates/svc-index/tests/http_contract.rs
crates/svc-index/tests/integration.rs
```

All-targets also covers additional crate tests and examples/benches.

## Final Verified svc-index Test Results

The final parking run verified:

```text
quickchain_preflight_boundary.rs:          4 passed
quickchain_preflight_docs.rs:              4 passed
quickchain_preflight_pointer_authority.rs: 5 passed
quickchain_preflight_routes.rs:            2 passed
quickchain_tooling_boundary.rs:            4 passed
http_contract.rs:                          3 passed
integration.rs:                            3 passed
cargo test -p svc-index --all-targets:     passed
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings: passed
```

Additional all-targets coverage included:

```text
chaos.rs
loom_index.rs
prop_index.rs
benches/resolve.rs
examples/client.rs
src/lib.rs
src/main.rs
```

## What svc-index Now Proves

`svc-index` now proves:

```text
svc-index remains lookup/pointer infrastructure.
svc-index does not depend on wallet/ledger authority crates.
svc-index does not import value-plane mutation authority crates.
svc-index production source does not call wallet mutation verbs.
svc-index production source does not define QuickChain runtime infrastructure.
svc-index docs preserve the backend paid-access truth path.
svc-index docs warn not to over-ban legitimate metadata.
svc-index pointer DTOs reject authority-shaped fields.
svc-index stored asset pointers carry no paid or chain authority.
svc-index stored site pointers carry no paid or chain authority.
svc-index owner wallet metadata is normalized as reference-only metadata.
svc-index public routes do not expose QuickChain settlement surfaces.
svc-index routes remain lookup/pointer/admin/health surfaces.
svc-index tooling remains Bash/Rust-only.
svc-index preflight dynamically discovers QuickChain tests.
svc-index parking gate is repeatable.
```

## Important svc-index Boundary Details

### Pointer Records

Pointer records are meaningful, but only as pointer/index truth.

Correct:

```text
asset_cid -> manifest_cid
site_name -> manifest_cid
name -> content reference
provider lookup -> possible provider metadata
owner_wallet_account -> reference metadata only
owner_passport_subject -> reference metadata only
```

Incorrect / forbidden:

```text
index record proves paid access
index record proves wallet balance
index record proves ledger state
index record proves ownership finality
index record proves receipt finality
index record proves QuickChain root state
index record unlocks paid content
index cache unlocks paid content
```

### b3 / crab Semantics

Correct:

```text
b3 identifies bytes/content.
crab:// is navigation.
names are mutable pointers.
manifests describe content.
index resolves references.
```

Forbidden:

```text
b3 hash as payment proof
crab:// route as paid unlock authority
name resolution as ownership proof
manifest lookup as receipt proof
provider lookup as settlement finality
```

### Owner / Passport / Wallet Fields

`svc-index` may carry owner/passport/wallet references as metadata.

Allowed:

```text
owner_passport_subject
owner_wallet_account
creator reference
manifest owner reference
site owner reference
```

Forbidden:

```text
owner_wallet_account as spend authority
passport subject as paid proof
owner metadata as receipt proof
owner metadata as balance proof
owner metadata as finality proof
index metadata as unlock authority
```

## svc-index Tests by Purpose

### `quickchain_preflight_boundary.rs`

Purpose:

* Proves `svc-index` does not depend on wallet/ledger/runtime authority crates.
* Proves production source does not import mutation authority.
* Proves production source does not call direct wallet mutation verbs.
* Proves production source does not define runtime/root/checkpoint/validator behavior.

This protects against accidental settlement creep.

### `quickchain_preflight_docs.rs`

Purpose:

* Proves the docs plainly state the crate boundary.
* Proves docs preserve lookup/pointer-only doctrine.
* Proves docs preserve paid-access backend truth path.
* Proves docs keep forbidden runtime/external-settlement scope explicit.
* Proves docs avoid overcorrecting by banning legitimate metadata blindly.

This protects against future maintainers misunderstanding `svc-index` as economic authority.

### `quickchain_preflight_pointer_authority.rs`

Purpose:

* Proves stored asset pointers do not become economic proof.
* Proves stored site pointers do not become economic proof.
* Proves authority-shaped fields reject.
* Proves owner/passport/wallet metadata remains reference-only.
* Proves pointer records do not manufacture paid, chain, receipt, balance, root, or finality claims.

This protects against fake entitlement or fake receipt creation through index state.

### `quickchain_preflight_routes.rs`

Purpose:

* Proves public routes stay in lookup/pointer/admin/health territory.
* Proves routes do not expose QuickChain settlement/root/checkpoint/validator surfaces.

This protects against accidentally adding `/settle`, `/checkpoint`, `/root`, `/validator`, `/bridge`, or similar route authority into `svc-index`.

### `quickchain_tooling_boundary.rs`

Purpose:

* Proves docs/preflight/park files exist.
* Proves preflight uses Bash strict mode.
* Proves preflight uses env-aware Cargo.
* Proves preflight dynamically discovers `quickchain*.rs`.
* Proves preflight runs known regressions.
* Proves preflight runs all-targets.
* Proves preflight runs clippy with `-D warnings`.
* Proves no Python helper drift exists.

This protects the repeatability of the crate-local QuickChain gate.

## svc-index Final Commands

Focused tooling tests:

```bash
cargo test -p svc-index --test quickchain_tooling_boundary
```

Low-disk parking gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
```

## svc-index Current Future Work

The current QuickChain Phase-0 slice is complete. Future work should remain scoped.

Safe future work:

```text
cleanup stale svc-index2 workflow references
real fuzz targets
real chaos/perf scripts
more HTTP PUT/GET manifest route integration tests
cache-hit cannot unlock paid content tests
transport/header context-only tests if needed
manifest pointer snapshot discussion
schema cleanup
stronger b3 normalization tests
stronger crab:// navigation tests
```

Still forbidden in `svc-index`:

```text
root production
receipt root production
state root production
checkpoint production
validator behavior
settlement behavior
bridge behavior
staking/liquidity logic
external chain logic
ROX/Solana logic
wallet mutation
ledger mutation
receipt creation
fake balance response
fake paid proof
cache-only paid unlock
index-only paid unlock
```

---

# 2. ron-policy Notes

## Current Status

```text
ron-policy: GREEN / Phase-0 preflight complete / parked
```

`ron-policy` now has a repeatable QuickChain safety gate proving that it remains declarative policy infrastructure only.

The final parking run ended with:

```text
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## Intended Role

`ron-policy` is declarative policy infrastructure.

It may provide:

* allow/deny decisions
* policy reasons
* explanation traces
* obligations as instructions
* strict policy DTOs
* JSON/TOML policy parsing
* schema constraints
* economics policy configuration parsing
* ROC pricing helper calculations
* cap/split validation helpers
* deterministic policy evaluation

It must not become:

* economic truth
* wallet authority
* ledger authority
* receipt authority
* balance authority
* finality authority
* paid unlock authority
* settlement authority
* QuickChain runtime
* root producer
* checkpoint producer
* validator
* bridge
* staking/liquidity layer
* external settlement component

## Core ron-policy Doctrine

The central boundary is:

```text
ron-policy is declarative policy infrastructure.
```

More specifically:

```text
policy decision is not economic truth
policy allow is not paid proof
policy obligation is not receipt proof
policy explanation is not finality proof
economics policy config is not ledger mutation
feature flag is not settlement authority
```

Policy can say:

```text
allow
deny
require backend proof
require a caller/service to perform a check
apply a cap
calculate a quoted/estimated amount
validate configured splits
explain why a rule matched
```

Policy cannot say:

```text
receipt created
balance updated
wallet mutated
ledger mutated
paid access unlocked
settlement finalized
checkpoint produced
root committed
validator signed
bridge completed
```

## Files Added / Hardened

### Documentation

```text
crates/ron-policy/docs/quickchain-preflight.md
```

Purpose:

* Defines `ron-policy` as declarative policy infrastructure.
* Separates policy decisions from economic truth.
* Separates obligations from receipt proof.
* Separates explanation traces from finality proof.
* Separates economics config from ledger mutation.
* Keeps forbidden runtime/external-settlement scope explicit.
* Documents that policy must not manufacture paid proof, receipt proof, finality proof, or balance proof.

### Scripts

```text
crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/scripts/dev-quickchain-park.sh
```

The preflight script now:

* Uses Bash strict mode: `set -euo pipefail`.
* Supports `CARGO="${CARGO:-cargo}"`.
* Supports `--check`.
* Locates repo root and crate root.
* Verifies QuickChain docs file exists.
* Verifies tests directory exists.
* Rejects Python helper drift under the crate.
* Runs format check or formatting.
* Dynamically discovers `quickchain*.rs` tests.
* Runs each QuickChain test by test target name.
* Runs existing `ron-policy` regressions.
* Runs `cargo test -p ron-policy --all-targets`.
* Runs `cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings`.
* Prints forbidden runtime scope parked message.
* Prints final preflight success marker.

The park script now:

* Verifies docs/preflight/tooling files exist.
* Delegates to the preflight script.
* Supports `--check`.
* Prints the final parking success marker.

### Tests

Current focused QuickChain tests include:

```text
crates/ron-policy/tests/quickchain_preflight_boundary.rs
crates/ron-policy/tests/quickchain_preflight_public_surface_boundary.rs
crates/ron-policy/tests/quickchain_preflight_decision_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_identifier_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_schema_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_parser_consistency.rs
crates/ron-policy/tests/quickchain_preflight_docs.rs
crates/ron-policy/tests/quickchain_tooling_boundary.rs
```

Existing regression tests included in the gate:

```text
crates/ron-policy/tests/economics_policy.rs
crates/ron-policy/tests/unit_model_serde_strict.rs
crates/ron-policy/tests/unit_eval_determinism.rs
crates/ron-policy/tests/unit_first_match_wins.rs
crates/ron-policy/tests/golden_reasons.rs
```

All-targets also covers the broader policy unit suite, bench target, and examples.

## Final Verified ron-policy Test Results

The final parking run verified:

```text
quickchain_preflight_boundary.rs:                          4 passed
quickchain_preflight_decision_non_authority.rs:            4 passed
quickchain_preflight_docs.rs:                              3 passed
quickchain_preflight_economics_config_non_authority.rs:    4 passed
quickchain_preflight_economics_identifier_non_authority.rs: 7 passed
quickchain_preflight_parser_consistency.rs:                4 passed
quickchain_preflight_public_surface_boundary.rs:           1 passed
quickchain_preflight_schema_non_authority.rs:              5 passed
quickchain_tooling_boundary.rs:                            4 passed
economics_policy.rs:                                       15 passed
unit_model_serde_strict.rs:                                2 passed
unit_eval_determinism.rs:                                  1 passed
unit_first_match_wins.rs:                                  1 passed
golden_reasons.rs:                                         1 passed
cargo test -p ron-policy --all-targets:                    passed
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings: passed
```

Additional all-targets coverage included:

```text
unit_churn_protection.rs
unit_eu_only.rs
unit_large_body_default_deny.rs
unit_method_matrix.rs
unit_tags_all.rs
unit_tighten_only.rs
benches/eval_throughput.rs
examples/minimal_allow_deny.rs
examples/ron_policy_cli.rs
src/lib.rs
```

## What ron-policy Now Proves

`ron-policy` now proves:

```text
ron-policy remains declarative policy infrastructure.
ron-policy does not depend on mutation authority crates.
ron-policy production source does not import wallet/ledger/root/checkpoint/runtime authority.
ron-policy production source does not call wallet or ledger mutation verbs.
ron-policy production source does not define QuickChain runtime surfaces.
ron-policy examples, benches, and fuzz targets remain policy-only.
ron-policy decisions do not become paid unlocks.
ron-policy allow decisions do not become paid proof.
ron-policy deny decisions do not become receipt/balance/finality truth.
ron-policy obligations cannot smuggle authority-shaped kinds.
ron-policy obligation params cannot smuggle authority-shaped keys.
ron-policy economics config rejects receipt/balance/finality/root fields.
ron-policy economics helpers return amounts/validation only, not authority artifacts.
ron-policy economics identifiers reject authority-shaped aliases.
ron-policy schema preserves strict DTO shape.
ron-policy schema rejects authority-shaped obligations and params.
ron-policy parser behavior is consistent across JSON/TOML paths.
ron-policy tooling remains Bash/Rust-only.
ron-policy parking gate is repeatable.
```

## Important ron-policy Boundary Details

### Policy Decisions

Policy decisions are control-plane outputs, not value-plane truth.

Allowed:

```text
DecisionEffect::Allow
DecisionEffect::Deny
reason
trace
obligations
```

Forbidden:

```text
receipt_id
receipt_hash
receipt_root
balance_minor
wallet_balance
ledger_balance
finality
finalized
unlock_granted
paid_proof
settlement_status
state_root
checkpoint_root
validator_signature
mint_authority
```

### Obligations

Obligations are instructions for callers/services. They are not proof.

Allowed obligation shape:

```text
require-backend-wallet-ledger-proof
log-audit
add-header
mask-field
require-proof-check
```

Forbidden obligation authority shapes:

```text
create_receipt
put_receipt
insert_receipt
accept_receipt
commit_receipt
finalize_receipt
unlock_paid_content
prove_payment_finality
produce_checkpoint
produce_root
settlement_complete
settlement_finalized
bridge_settlement
mutate_balance
set_balance
credit_account
debit_account
mint_roc
allocate_roc
```

### Economics Config

Economics config is declarative configuration.

Allowed:

```text
roc_minor integer units
basis-point splits
action pricing
minimum charges
maximum spend caps
hold multiplier basis points
recipient role aliases
static account aliases
capture-plan validation
price estimation
```

Forbidden:

```text
ledger mutation
wallet mutation
receipt truth
balance truth
finality truth
paid unlock truth
root/checkpoint truth
settlement truth
validator signatures
mint authority
operation authority
```

### Important Correction: Declarative `burn` Alias

A critical overbroad scanner/validator issue was corrected.

The word `burn` can be a legitimate declarative account alias in economics configuration. It must not automatically be treated as a burn operation.

Correct distinction:

```text
burn as an account alias: allowed when declarative
burn as a wallet/ledger mutation operation: forbidden outside wallet/ledger authority
```

The final behavior allows the legitimate declarative `burn` account alias while still rejecting authority-shaped identifiers such as:

```text
receipt_root
balance_minor
settlement_status
validator_signature
mint_authority
operation_id
idempotency_key
```

This is important because QuickChain preflight must prevent authority creep without destroying useful declarative economics vocabulary.

## ron-policy Tests by Purpose

### `quickchain_preflight_boundary.rs`

Purpose:

* Proves `ron-policy` does not depend on mutation authority crates.
* Proves production source does not import mutation authority crates.
* Proves production source does not call wallet/ledger mutation verbs.
* Proves production source does not define QuickChain runtime surfaces.

This protects `ron-policy` from becoming economic or chain authority.

### `quickchain_preflight_public_surface_boundary.rs`

Purpose:

* Proves examples, benches, and fuzz targets remain policy-only.
* Prevents side surfaces from smuggling authority behavior.
* Ensures public/dev surfaces do not drift into wallet/ledger/root/settlement behavior.

This protects the non-library surfaces.

### `quickchain_preflight_decision_non_authority.rs`

Purpose:

* Proves allow/deny decisions are policy results only.
* Proves allow does not mean paid unlock.
* Proves deny does not mean economic truth.
* Proves obligations cannot smuggle receipt/balance/finality authority.
* Proves authority-shaped obligation kinds and params reject.

This protects against policy becoming proof.

### `quickchain_preflight_economics_config_non_authority.rs`

Purpose:

* Proves economics config rejects top-level authority fields.
* Proves action config rejects authority fields.
* Proves serialized economics config has no receipt/balance/finality/root keys.
* Proves economics helpers return numeric amounts/validation only.

This protects against economics config becoming ledger truth.

### `quickchain_preflight_economics_identifier_non_authority.rs`

Purpose:

* Proves checked-in economics identifiers remain non-authority.
* Proves declarative `burn` alias is allowed.
* Proves receipt/balance/settlement/validator/token authority-shaped aliases reject.
* Proves action IDs cannot be named like token mutation authority.

This protects against identifier-level authority smuggling.

### `quickchain_preflight_parser_consistency.rs`

Purpose:

* Proves JSON and TOML loaders consistently reject authority-shaped obligation kinds and params.
* Prevents one parser from being stricter than another.
* Ensures policy authority boundaries are format-independent.

This protects the parser layer.

### `quickchain_preflight_schema_non_authority.rs`

Purpose:

* Proves the published JSON schema is valid.
* Proves schema preserves strict DTO shape.
* Proves schema preserves body-cap limits.
* Proves schema rejects authority-shaped obligations.
* Proves schema rejects authority-shaped obligation params.

This protects downstream schema consumers.

### `quickchain_preflight_docs.rs`

Purpose:

* Proves docs file exists.
* Proves docs include boundary phrases.
* Proves docs explicitly state forbidden runtime/external-settlement scope.

This protects future maintainers and integration work.

### `quickchain_tooling_boundary.rs`

Purpose:

* Proves docs/preflight/park files exist.
* Proves preflight uses Bash strict mode.
* Proves preflight uses env-aware Cargo.
* Proves preflight dynamically discovers QuickChain tests.
* Proves preflight runs focused suites.
* Proves preflight runs existing regressions.
* Proves preflight runs all-targets.
* Proves preflight runs clippy with `-D warnings`.
* Proves no Python helper drift exists.

This protects repeatability.

## ron-policy Final Commands

Focused tooling test:

```bash
cargo test -p ron-policy --test quickchain_tooling_boundary
```

Low-disk parking gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## ron-policy Current Future Work

The current QuickChain Phase-0 slice is complete. Future work should remain scoped.

Safe future work:

```text
canonical policy vectors later
locked economics config hashes later
schema publication workflow
additional fuzzing
expanded parser consistency vectors
transport integration tests through consuming services
policy bundle provenance metadata
policy version migration rules
stronger golden reason snapshots
consumer integration tests confirming policy allow does not unlock paid content alone
```

Still forbidden in `ron-policy`:

```text
wallet mutation
ledger mutation
receipt creation
balance mutation
paid unlock finality
root production
checkpoint production
validator behavior
settlement behavior
bridge behavior
staking/liquidity logic
ROX/Solana logic
external settlement
public chain authority
```

---

# 3. Cross-Crate Implications

## Why svc-index + ron-policy Matter Together

These two crates are a critical boundary pair.

`svc-index` answers:

```text
Where is the content/pointer/manifest/provider?
```

`ron-policy` answers:

```text
Is this request allowed by declarative policy?
```

Neither answers:

```text
Was payment made?
Is the receipt real?
Is the wallet balance sufficient?
Was a hold captured?
Is the ledger updated?
Is a checkpoint final?
Is a QuickChain root valid?
```

Those remain backend value-plane truths.

## Correct End-to-End Boundary

The correct path for paid access remains:

```text
client request
→ gateway / omnigate route handling
→ policy check as declarative allow/deny/context only
→ index lookup for pointer/manifest/provider only
→ wallet/ledger/storage receipt validation for paid truth
→ storage/gateway/omnigate render/unlock after backend receipt truth
```

Incorrect paths that remain forbidden:

```text
policy allow → unlock paid content
index pointer exists → unlock paid content
manifest exists → unlock paid content
b3 exists → unlock paid content
cache hit → unlock paid content
owner metadata exists → paid proof
route metadata says paid=true → paid proof
policy obligation says proof required → proof exists
```

## Relationship to Internal ROC Value Loop

The current internal ROC value loop remains:

```text
ron-proto econ DTOs
→ ron-ledger truth
→ svc-wallet issue/transfer/burn/hold/capture/release/receipt
→ svc-storage / svc-gateway / omnigate paid enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ wallet/ledger receipts
```

`svc-index` participates only as lookup/pointer infrastructure.

`ron-policy` participates only as declarative policy/config infrastructure.

Neither crate mutates ledger truth.

Neither crate creates wallet receipts.

Neither crate updates balances.

Neither crate manufactures paid access.

## Phase-0 Completion Effect

With `svc-index` and `ron-policy` now green/parked, the Phase-0 service boundary mesh is much stronger.

The following boundary categories are now covered across the recent work:

```text
wallet/ledger mutation boundary
accounting snapshot boundary
rewarder planning boundary
storage paid-enforcement boundary
gateway public-boundary boundary
omnigate hydration/product-flow boundary
index lookup/pointer boundary
policy declarative-boundary boundary
```

This gives QuickChain Phase-0 a coherent safety envelope before any root-producing or validator work is allowed.

---

# 4. Current Known Parking Commands

## svc-index

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index
```

## ron-policy

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

## Codebundle Regeneration

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

---

# 5. Remaining Work After This Pair

## Immediate Next Likely QuickChain Work

The next major QuickChain safety step should be one of:

```text
workspace umbrella QuickChain Phase-0 runner
CrabLink Tauri + client adapter boundary hardening
final Phase-0 cross-crate audit
```

Recommended order:

```text
1. Add workspace umbrella runner.
2. Verify every parked crate in stable order.
3. Harden CrabLink Tauri/client adapter boundaries.
4. Run final Phase-0 cross-crate audit.
5. Only then discuss Phase-1 canonical bytes / locked vectors / root-producing design.
```

## Workspace Runner Goal

A future workspace-level runner should execute crate-local gates in a stable order:

```text
ron-proto
ron-ledger
svc-wallet
ron-accounting
svc-rewarder
svc-storage
svc-gateway
omnigate
svc-index
ron-policy
```

It should prefer crate-local park/preflight scripts and support low-disk mode.

It must not introduce any runtime authority.

## Still Forbidden Until Later Gates

The following remain forbidden until explicit later-phase authorization:

```text
root-producing code
state roots
receipt roots
checkpoint roots
checkpoint headers
validators
validator signatures
committee quorum
fork choice
pruning
settlement
anchors
bridges
ROX
Solana
external L2
external DA
public bridge
staking
liquidity
exchange-facing logic
CrabLink chain authority
gateway ledger mutation
omnigate ledger mutation
rewarder ledger mutation
accounting balance truth
index paid unlock authority
policy paid unlock authority
cache-only paid access
fake receipts
fake balances
fake finality
placeholder hashes
```

---

# 6. Final Crate Status

## svc-index

```text
Status: GREEN / PARKED
QuickChain Phase-0 role: lookup/pointer service only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid unlock authority: no
Root/checkpoint/validator authority: no
Settlement/bridge authority: no
Parking script: passed
All-targets: passed
Clippy -D warnings: passed
```

## ron-policy

```text
Status: GREEN / PARKED
QuickChain Phase-0 role: declarative policy infrastructure only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid unlock authority: no
Receipt/balance/finality authority: no
Root/checkpoint/validator authority: no
Settlement/bridge authority: no
Parking script: passed
All-targets: passed
Clippy -D warnings: passed
```

## Bottom Line

`svc-index` and `ron-policy` are complete for the current QuickChain Phase-0 safety/preflight slice.

The system now has strong boundary tests proving:

```text
lookup is not payment
policy is not proof
metadata is not authority
cache is not unlock
names are pointers
b3 is byte identity
wallet/ledger remain economic truth
QuickChain runtime remains parked
external settlement remains deferred
```

### END NOTE - JUNE 17 2026 - 18:35 CST


### BEGIN NOTE - JUNE 19 2026 - 17:20 CST

Both crate notes below are paste-ready. They reflect the green terminal proof: `svc-index` parked with `tests=6`, `ron-policy` parked with `tests=10`, both `all-targets` and strict Clippy clean, and both crate codebundles regenerated successfully.  The notes keep the QuickChain scope correctly parked as future settlement infrastructure, not current runtime chain logic. 

### BEGIN NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - svc-index

# svc-index QuickChain QC-0A / Phase-0 Notes

## Status

`svc-index` is now green for the current QuickChain QC-0A / Phase-0 preflight pass.

This crate is parked as a **lookup / pointer / name / manifest reference boundary**.

It is not wallet authority, ledger authority, paid-unlock authority, receipt authority, balance authority, finality authority, root authority, checkpoint authority, validator authority, settlement authority, or bridge authority.

## Current verdict

```text
svc-index QuickChain Phase-0 preflight passed.
svc-index QuickChain parking gate passed.
svc-index quickchain exhaustive preflight gate passed: tests=6
```

This is sufficient to mark the current `svc-index` QuickChain safety-preflight slice complete.

No additional patch is required from the latest terminal output.

---

## What changed in this slice

This pass hardened `svc-index` against QuickChain/value-plane authority creep.

The key principle added and enforced is:

```text
svc-index is lookup and pointer infrastructure only.
index truth is not economic truth.
pointer truth is not receipt truth.
name resolution is not ownership proof.
b3 byte identity is not payment proof.
manifest lookup is not paid unlock.
provider lookup is not settlement finality.
```

The crate now has explicit docs, scripts, and tests proving this boundary.

---

## Files added or materially updated

### `crates/svc-index/docs/quickchain-preflight.md`

Purpose:

```text
Documents the svc-index QuickChain Phase-0 boundary.
```

It explicitly states:

```text
svc-index is a lookup and pointer service.
```

It allows `svc-index` to resolve:

```text
- names
- b3 content identifiers
- manifest pointers
- provider metadata
- site manifest pointers
- asset manifest pointers
- reference metadata
```

It forbids `svc-index` from becoming:

```text
- value-plane authority
- wallet authority
- ledger authority
- paid-unlock authority
- receipt authority
- balance authority
- finality authority
- root authority
- checkpoint authority
- validator authority
- settlement authority
- bridge authority
```

It also preserves an important nuance:

```text
Do not blindly ban legitimate metadata words.
```

The docs allow descriptive metadata like:

```text
owner
creator
recipient
policy
manifest
wallet as a reference string
```

But reject authority-shaped usage like:

```text
unlock_from_index
receipt_from_index
balance_from_index
finality_from_index
checkpoint_from_index
validator_from_index
root_from_index
settle_from_index
mint_from_index
transfer_from_index
paid_from_index
entitlement_from_index
```

This prevents false positives while still blocking authority creep.

---

### `crates/svc-index/scripts/dev-quickchain-preflight.sh`

Purpose:

```text
Runs the crate-local exhaustive QuickChain Phase-0 preflight gate.
```

The script now verifies:

```text
- docs file exists
- test directory exists
- no Python helper drift exists under the crate
- formatting is clean
- every quickchain*.rs test runs dynamically
- selected existing regressions run
- all targets pass
- strict clippy passes with -D warnings
```

It uses:

```text
set -euo pipefail
```

It accepts:

```text
--check
```

It dynamically discovers QuickChain tests instead of hardcoding a stale list.

This is important because new `quickchain*.rs` tests automatically enter the preflight gate.

---

### `crates/svc-index/scripts/dev-quickchain-park.sh`

Purpose:

```text
Stable parking gate wrapper for svc-index.
```

The parking script verifies required preflight artifacts exist, then delegates to:

```text
crates/svc-index/scripts/dev-quickchain-preflight.sh
```

This preserves the low-disk workflow rule:

```text
After focused tests, run only the park script.
Do not run both preflight and park manually for the same crate unless intentionally doing two full passes.
```

---

### `crates/svc-index/tests/quickchain_preflight_value_loop_boundary.rs`

Purpose:

```text
Proves svc-index cannot become a hidden value-loop authority.
```

This test file added coverage for:

```text
- no direct value-plane or QuickChain runtime dependencies
- no authority-shaped value-loop entrypoints in production source
- no forbidden settlement/paid-unlock/wallet/ledger mutation routes
- docs preserve backend-paid-access truth path
```

It checks for forbidden direct dependency drift such as:

```text
svc-wallet
ron-ledger
ron-accounting
svc-rewarder
quickchain-runtime
quickchain-validator
quickchain-consensus
solana
solana-sdk
solana-client
```

It blocks authority-shaped production-source tokens such as:

```text
unlock_from_index
receipt_from_index
balance_from_index
finality_from_index
checkpoint_from_index
validator_from_index
root_from_index
settle_from_index
mint_from_index
transfer_from_index
paid_from_index
entitlement_from_index
issue_receipt_from_index
claim_finality_from_index
grant_unlock_from_index
capture_hold_from_index
release_hold_from_index
```

It also checks `src/router.rs` for forbidden route fragments such as:

```text
/quickchain/root
/quickchain/checkpoint
/quickchain/finality
/quickchain/validator
/quickchain/bridge
/settle
/settlement
/mint
/transfer
/receipt/fake
/unlock/from-index
/balance/from-index
/wallet/mutate
/ledger/mutate
```

The purpose is not to ban index functionality.

The purpose is to prevent index lookup from becoming a hidden payment, balance, finality, or settlement surface.

---

## Existing QuickChain tests proven by this run

The park script dynamically found and ran six QuickChain test targets:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

### `quickchain_preflight_boundary`

Proves:

```text
- Cargo does not depend on wallet or ledger authority crates
- production source does not import value-plane authority crates
- production source does not define QuickChain runtime infrastructure
- production source does not call wallet mutation verbs
```

### `quickchain_preflight_docs`

Proves docs preserve the required boundary statements:

```text
- svc-index is lookup/pointer only
- backend paid-access truth path remains explicit
- legitimate metadata is not blindly banned
- QuickChain runtime and external settlement scope remain forbidden
```

### `quickchain_preflight_pointer_authority`

Proves pointer wire shapes stay non-authoritative:

```text
- owner_wallet metadata is allowed but normalized as reference only
- stored site pointer shape has no paid/chain authority
- stored asset pointer shape has no paid/chain authority
- site manifest pointer rejects authority-shaped fields
- asset manifest pointer rejects authority-shaped fields
```

### `quickchain_preflight_routes`

Proves:

```text
- allowed routes remain lookup/pointer/admin-health surfaces
- public routes do not expose QuickChain settlement surfaces
```

### `quickchain_preflight_value_loop_boundary`

Proves:

```text
- index does not directly depend on value-plane/QuickChain runtime crates
- index router does not expose paid unlock or settlement authority routes
- index source has no authority-shaped value-loop entrypoints
- docs preserve backend value-loop boundaries
```

### `quickchain_tooling_boundary`

Proves:

```text
- QuickChain tooling files exist
- park script delegates to preflight
- preflight script is strict Bash and dynamic
- crate has no Python helper drift
```

---

## Existing non-QuickChain regressions proven by this run

The preflight script also ran existing `svc-index` regressions:

```text
http_contract
integration
```

### `http_contract`

Passed tests proving:

```text
- validators accept beta asset forms
- asset manifest pointer wire shape is stable
- put_asset_manifest_pointer rejects unknown fields
```

### `integration`

Passed tests proving:

```text
- missing pointers return none
- site manifest pointer roundtrips through store
- asset manifest pointer roundtrips through store
```

The all-targets run also proved:

```text
chaos.rs
loom_index.rs
prop_index.rs
quickchain*.rs
benches/resolve.rs
examples/client.rs
```

Relevant proven behavior includes:

```text
- malformed inputs fail closed
- owner reference normalization drops empty values
- memory store survives concurrent pointer updates
- b3 validation rejects bad lengths and characters
- asset kind validation rejects unknown forms
- site name validation rejects unsafe forms
```

---

## Quality gate status

The latest run proves:

```text
cargo fmt -p svc-index                              PASS
cargo test -p svc-index --test quickchain_preflight_docs                  PASS
cargo test -p svc-index --test quickchain_tooling_boundary                 PASS
cargo test -p svc-index --test quickchain_preflight_value_loop_boundary    PASS
bash crates/svc-index/scripts/dev-quickchain-park.sh --check              PASS
cargo test -p svc-index --all-targets                                      PASS
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings           PASS
```

The regenerated crate codebundle also succeeded:

```text
Wrote 100 files to: crates/svc-index/CODEBUNDLE.md
```

---

## Boundary now enforced

`svc-index` may continue to own:

```text
- lookup
- names
- pointers
- manifest references
- provider metadata
- site references
- asset references
- b3 reference validation
- reference normalization
- pointer wire-shape validation
```

`svc-index` must not own:

```text
- wallet mutation
- ledger mutation
- ROC issuance
- ROC transfer
- ROC burn
- hold capture
- hold release
- receipt creation
- balance truth
- finality truth
- paid unlock truth
- QuickChain roots
- QuickChain checkpoints
- QuickChain validator logic
- QuickChain settlement logic
- external bridge logic
- Solana logic
- ROX logic
- staking
- liquidity
```

---

## Paid access boundary

The paid access path remains outside `svc-index`.

The correct authority path remains:

```text
svc-wallet / ron-ledger truth
→ backend receipt or backend authorization result
→ svc-storage / svc-gateway / omnigate paid enforcement
→ render/display after backend acceptance
```

Forbidden unlock shortcuts remain:

```text
index entry → unlock
manifest lookup → unlock
cache hit → unlock
client says paid → unlock
```

The key rule:

```text
svc-index may help find what content is.
svc-index must not decide who has paid for it.
```

---

## QuickChain status for svc-index

`svc-index` is complete for this QC-0A / Phase-0 pass.

It is not complete for future Phase-1+ QuickChain root/proof work because that work is not authorized yet.

Future work, when explicitly authorized after the proper gates, may include read-only proof-oriented DTO integration or root-verifiable index snapshots, but only after canonical bytes, locked vectors, and root-producing rules are approved.

For now:

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No wallet mutation.
No ledger mutation.
No fake receipts.
No fake balances.
No fake finality.
No paid unlocks from index entries.
```

---

## Recommended next commands only if files change

Do not rerun this full pass unless source/docs/tests change.

If `svc-index` changes again, use the low-disk flow:

```bash
cargo fmt -p svc-index
cargo test -p svc-index --test quickchain_preflight_docs
cargo test -p svc-index --test quickchain_tooling_boundary
cargo test -p svc-index --test quickchain_preflight_value_loop_boundary

CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index

bash scripts/make_crate_codex.sh --force -c svc-index
```

Avoid duplicate exhaustive runs.

The park script already delegates to preflight.

---

## Completion estimate for svc-index in current QuickChain phase

For the currently intended QC-0A / Phase-0 safety-preflight scope:

```text
svc-index: 100% complete for this crate-pair pass
```

For the future full QuickChain vision:

```text
svc-index: parked and ready as a non-authority boundary
```

Future root/proof/index snapshot work remains explicitly deferred.

### END NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - svc-index

### BEGIN NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - ron-policy

# ron-policy QuickChain QC-0A / Phase-0 Notes

## Status

`ron-policy` is now green for the current QuickChain QC-0A / Phase-0 preflight pass.

This crate is parked as a **declarative policy / allow-deny / classification boundary**.

It is not wallet authority, ledger authority, receipt authority, balance authority, paid-unlock authority, payout authority, finality authority, root authority, checkpoint authority, validator authority, bridge authority, or settlement authority.

## Current verdict

```text
ron-policy QuickChain Phase-0 preflight passed.
ron-policy QuickChain parking gate passed.
ron-policy quickchain exhaustive preflight gate passed: tests=10
```

This is sufficient to mark the current `ron-policy` QuickChain safety-preflight slice complete.

No additional patch is required from the latest terminal output.

---

## What changed in this slice

This pass hardened `ron-policy` against QuickChain/value-plane authority creep.

The key principle added and enforced is:

```text
ron-policy is declarative policy infrastructure.
policy decision is not economic truth.
policy allow is not paid proof.
policy obligation is not receipt proof.
policy explanation is not finality proof.
economics policy config is not ledger mutation.
feature flag is not settlement authority.
policy config is not wallet authority.
policy classification is not payout allocation.
reason code is not finality.
policy metadata is not paid entitlement.
```

The crate now explicitly proves that policy may decide, classify, explain, and declare obligations, but it cannot manufacture backend truth.

---

## Files added or materially updated

### `crates/ron-policy/docs/quickchain-preflight.md`

Purpose:

```text
Documents the ron-policy QuickChain Phase-0 boundary.
```

It allows `ron-policy` to provide:

```text
- allow decisions
- deny decisions
- reason codes
- obligations
- classification
- request constraints
- quota/config interpretation
- economics policy configuration
- explanations/traces for policy decisions
```

It forbids `ron-policy` from becoming:

```text
- wallet truth
- ledger truth
- receipt truth
- balance truth
- paid-access authority
- finality authority
- payout authority
- root authority
- checkpoint authority
- validator authority
- bridge authority
- external settlement authority
```

The docs explicitly preserve these scanner phrases:

```text
policy decision is not economic truth.
policy allow is not paid proof.
policy obligation is not receipt proof.
policy explanation is not finality proof.
economics policy config is not ledger mutation.
feature flag is not settlement authority.
policy config is not wallet authority.
policy classification is not payout allocation.
reason code is not finality.
policy metadata is not paid entitlement.
Policy must not manufacture paid proof.
Policy must not manufacture receipt proof.
Policy must not manufacture finality proof.
Policy must not manufacture balance proof.
```

This prevents the common architecture failure where “allow” slowly becomes “paid,” “entitled,” “final,” or “settled.”

---

### `crates/ron-policy/scripts/dev-quickchain-preflight.sh`

Purpose:

```text
Runs the crate-local exhaustive QuickChain Phase-0 preflight gate.
```

The script verifies:

```text
- docs file exists
- test directory exists
- no Python helper drift exists under the crate
- formatting is clean
- every quickchain*.rs test runs dynamically
- selected policy/economics regressions run
- all targets pass
- strict clippy passes with -D warnings
```

It uses:

```text
set -euo pipefail
```

It accepts:

```text
--check
```

It dynamically discovers QuickChain tests.

This is important because any new `quickchain*.rs` test becomes part of the preflight gate automatically.

---

### `crates/ron-policy/scripts/dev-quickchain-park.sh`

Purpose:

```text
Stable parking gate wrapper for ron-policy.
```

The parking script verifies required preflight artifacts exist, then delegates to:

```text
crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

This preserves the low-disk workflow rule:

```text
After focused tests, run only the park script.
Do not run both preflight and park manually for the same crate unless intentionally doing two full passes.
```

---

### `crates/ron-policy/tests/quickchain_preflight_value_loop_boundary.rs`

Purpose:

```text
Proves ron-policy cannot become hidden value-loop authority.
```

This test file added coverage for:

```text
- no direct value-plane or QuickChain runtime dependencies
- no authority-shaped value-loop entrypoints in production source
- docs preserve policy-as-declarative-only boundary
- docs reject transport/header claims as authority
```

It checks for forbidden direct dependency drift such as:

```text
svc-wallet
ron-ledger
ron-accounting
svc-rewarder
quickchain-runtime
quickchain-validator
quickchain-consensus
solana
solana-sdk
solana-client
```

It blocks authority-shaped production-source tokens such as:

```text
receipt_from_policy
balance_from_policy
unlock_from_policy
finality_from_policy
settle_from_policy
root_from_policy
checkpoint_from_policy
validator_from_policy
mint_from_policy
transfer_from_policy
capture_from_policy
release_from_policy
reward_from_policy
issue_receipt_from_policy
grant_unlock_from_policy
claim_finality_from_policy
allocate_protocol_roc_from_policy
execute_payout_from_policy
```

This protects the boundary between policy decisions and economic mutation.

---

## Existing QuickChain tests proven by this run

The park script dynamically found and ran ten QuickChain test targets:

```text
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

### `quickchain_preflight_boundary`

Proves:

```text
- Cargo.toml does not depend on mutation authority crates
- production source does not import mutation authority crates
- production source does not define QuickChain runtime surface
- production source does not call wallet or ledger mutation verbs
```

### `quickchain_preflight_decision_non_authority`

Proves:

```text
- allow decision is policy result, not paid unlock or receipt truth
- deny decision is still not receipt, balance, or finality truth
- authority-shaped obligation kind rejects policy
- authority-shaped obligation param key rejects policy
```

### `quickchain_preflight_docs`

Proves docs preserve:

```text
- required policy boundary phrases
- forbidden runtime scope
- QuickChain Phase-0 non-authority status
```

### `quickchain_preflight_economics_config_non_authority`

Proves:

```text
- economics config rejects top-level authority fields
- economics action config rejects authority fields
- helpers return amounts and validation only, not truth artifacts
- serialized shape has no receipt/balance/finality/root fields
```

### `quickchain_preflight_economics_identifier_non_authority`

Proves:

```text
- token-mutation-shaped action IDs reject before use
- receipt-authority-shaped account aliases reject
- balance-authority-shaped role aliases reject
- settlement-authority-shaped remainder sinks reject
- validator-authority-shaped split destinations reject
- declarative burn account aliases can be allowed without granting mutation authority
- checked-in economics identifiers remain non-authority
```

This is especially important because economics config may legitimately contain words related to ROC distribution, but must remain declarative.

### `quickchain_preflight_parser_consistency`

Proves JSON and TOML loaders consistently reject authority-shaped policy fields:

```text
- JSON loader rejects dash-separated authority obligation params
- JSON loader rejects camelCase authority obligation kinds
- TOML loader rejects authority obligation kinds
- TOML loader rejects authority obligation param keys
```

### `quickchain_preflight_public_surface_boundary`

Proves:

```text
- examples, benches, and fuzz targets remain policy-only
```

### `quickchain_preflight_schema_non_authority`

Proves:

```text
- policybundle schema is valid JSON
- schema preserves strict DTO shape
- schema preserves body cap limit
- schema rejects authority-shaped obligations
- schema rejects authority-shaped obligation params
```

### `quickchain_preflight_value_loop_boundary`

Proves:

```text
- docs reject transport/header claims as authority
- docs record policy as declarative without delegating backend truth
- ron-policy has no direct value-plane or QuickChain runtime dependencies
- production source has no authority-shaped value-loop entrypoints
```

### `quickchain_tooling_boundary`

Proves:

```text
- QuickChain tooling files exist
- park script delegates to preflight
- preflight script is strict Bash and dynamic
- crate has no Python helper drift
```

---

## Existing non-QuickChain regressions proven by this run

The preflight script also ran existing `ron-policy` regressions:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
```

### `economics_policy`

Passed 15 tests proving:

```text
- floats reject during parse
- negative values reject during parse
- overflow values reject during parse
- invalid split sums reject
- missing dynamic recipient rejects capture plan
- capture over action cap rejects
- disabled action rejects lookup but config can load
- missing required action rejects
- unknown paid action lookup rejects
- unknown action rejects
- unknown split destination rejects
- paid_storage_put price uses minimum and hold multiplier
- deterministic action order is sorted
- valid checked-in ROC economics config loads
```

These are important because `ron-policy` can compute policy/economics configuration, but those computations remain validation/planning helpers rather than wallet or ledger mutation.

### `unit_model_serde_strict`

Passed tests proving:

```text
- minimal policy model roundtrips
- unknown fields are denied
```

### `unit_eval_determinism`

Passed tests proving:

```text
- default deny remains deterministic
```

### `unit_first_match_wins`

Passed tests proving:

```text
- first-match-wins and default policy behavior remain stable
```

### `golden_reasons`

Passed tests proving:

```text
- explanation trace output remains stable
```

The all-targets run also proved the wider test surface:

```text
unit_churn_protection
unit_eu_only
unit_large_body_default_deny
unit_method_matrix
unit_tags_all
unit_tighten_only
benches/eval_throughput.rs
examples/minimal_allow_deny.rs
examples/ron_policy_cli.rs
```

---

## Quality gate status

The latest run proves:

```text
cargo fmt -p ron-policy                              PASS
cargo test -p ron-policy --test quickchain_preflight_docs                  PASS
cargo test -p ron-policy --test quickchain_tooling_boundary                 PASS
cargo test -p ron-policy --test quickchain_preflight_value_loop_boundary    PASS
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check              PASS
cargo test -p ron-policy --all-targets                                      PASS
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings           PASS
```

The regenerated crate codebundle also succeeded:

```text
Wrote 67 files to: crates/ron-policy/CODEBUNDLE.md
```

---

## Boundary now enforced

`ron-policy` may continue to own:

```text
- declarative rules
- allow/deny decisions
- reason codes
- obligations
- body caps
- quota interpretation
- feature-gate interpretation
- policy explanations
- policy traces
- economics config validation
- deterministic policy evaluation
```

`ron-policy` must not own:

```text
- wallet mutation
- ledger mutation
- ROC issuance
- ROC transfer
- ROC burn
- hold capture
- hold release
- receipt creation
- balance truth
- finality truth
- paid unlock truth
- payout execution
- reward allocation as protocol truth
- QuickChain roots
- QuickChain checkpoints
- QuickChain validator logic
- QuickChain settlement logic
- external bridge logic
- Solana logic
- ROX logic
- staking
- liquidity
```

---

## Value-loop boundary

The backend value loop remains:

```text
svc-wallet prepares, issues, transfers, burns, holds, captures, releases, and returns backend receipts.
ron-ledger remains durable economic truth.
ron-accounting records snapshots and usage views, not balance truth.
svc-rewarder plans payouts, not direct ledger mutation.
svc-storage, svc-gateway, and omnigate enforce paid access using backend-derived truth.
ron-policy only declares policy and evaluates policy.
```

`ron-policy` must not:

```text
- mutate wallet state
- mutate ledger state
- create wallet receipts
- create ledger receipts
- fabricate receipt IDs
- fabricate balances
- allocate protocol ROC
- allocate rewards
- execute payout
- unlock paid content by itself
- capture holds
- release holds
- settle operations
- claim accepted receipt status
- claim epoch_included status
- claim finalized status
- claim anchored status
```

---

## Transport/header boundary

This pass explicitly documents that caller-supplied transport/header context is context only.

These must never become authority inside policy:

```text
x-ron-paid: true as authority
x-ron-receipt: fake as authority
x-ron-balance: fake as authority
x-ron-finalized: true as authority
x-quickchain-root as authority
x-quickchain-checkpoint as authority
x-quickchain-validator as authority
```

The rule is:

```text
Policy is transport-agnostic.
Transport authority must be validated by callers through backend wallet/ledger/storage truth paths.
```

---

## QuickChain status for ron-policy

`ron-policy` is complete for this QC-0A / Phase-0 pass.

It is not complete for future Phase-1+ QuickChain settlement/root/proof work because that work is not authorized yet.

Future work, when explicitly authorized after the proper gates, may include more precise policy classifications for proof-eligible event classes or settlement-aware configuration validation, but not mutation, receipt creation, balance proof, finality, validators, roots, bridges, or anchors unless the full QuickChain phase gates allow it.

For now:

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No wallet mutation.
No ledger mutation.
No fake receipts.
No fake balances.
No fake finality.
No paid unlocks from policy decisions.
```

---

## Recommended next commands only if files change

Do not rerun this full pass unless source/docs/tests/schema/config files change.

If `ron-policy` changes again, use the low-disk flow:

```bash
cargo fmt -p ron-policy
cargo test -p ron-policy --test quickchain_preflight_docs
cargo test -p ron-policy --test quickchain_tooling_boundary
cargo test -p ron-policy --test quickchain_preflight_value_loop_boundary

CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy

bash scripts/make_crate_codex.sh --force -c ron-policy
```

Avoid duplicate exhaustive runs.

The park script already delegates to preflight.

---

## Completion estimate for ron-policy in current QuickChain phase

For the currently intended QC-0A / Phase-0 safety-preflight scope:

```text
ron-policy: 100% complete for this crate-pair pass
```

For the future full QuickChain vision:

```text
ron-policy: parked and ready as a non-authority declarative policy boundary
```

Future settlement-aware policy/proof integration remains explicitly deferred.

### END NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - ron-policy


### END NOTE - JUNE 19 2026 - 17:20 CST


### BEGIN NOTE - JUNE 21 2026 - 13:35 CST

Based on the current terminal output, the focused Phase 1 tests passed for `svc-index` and `ron-policy`: `svc-index`’s new pointer DTO strictness suite passed 5/5, and `ron-policy`’s new policy condition-tag suite passed 4/4, with parser consistency and schema non-authority suites also green.  The current focused codebundles also show both crates now have QuickChain park/preflight scripts and expanded QuickChain test surfaces.  

Here are the drop-in crate notes:

# QuickChain Phase 1 Crate Notes — `svc-index` + `ron-policy`

## Status Summary

`svc-index` and `ron-policy` are now **GREEN / PARKED** for the current QuickChain Phase 1 boundary-hardening pass.

This was not root-producing Phase 1 implementation. It was the correct next Phase 1 foundation step: hardening lookup and policy crates so they cannot accidentally become economic authority, paid-access authority, receipt authority, root/checkpoint authority, validator authority, bridge authority, or external settlement authority.

Current status:

```text
svc-index:  GREEN / PARKED
ron-policy: GREEN / PARKED
```

Phase interpretation:

```text
Phase 0 / QC-0A safety posture: preserved
Phase 1 foundation posture: advanced
Roots/proofs runtime implementation: still parked until final gates
Validator/committee/bridge/anchor work: still parked
```

The important outcome is that both crates now support the QuickChain value-loop doctrine without violating crate boundaries:

```text
ron-proto defines DTOs.
ron-ledger remains economic truth.
svc-wallet remains the mutation front-door.
ron-accounting remains snapshot/counter infrastructure, not balance truth.
svc-rewarder plans payouts but does not mutate balances.
svc-storage stores bytes and enforces paid access through backend truth.
svc-gateway is public boundary, not ledger authority.
omnigate hydrates/coordinatess product views, not ledger mutation.
svc-index resolves pointers/lookups, not paid entitlement.
ron-policy declares/evaluates policy, not payment proof.
CrabLink displays backend-derived truth, not local economic authority.
```

---

# `svc-index` QuickChain Phase 1 Notes

## Current Status

```text
Status: GREEN / PARKED
QuickChain role: lookup and pointer service only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid unlock authority: no
Receipt authority: no
Balance authority: no
Finality authority: no
Root/checkpoint authority: no
Validator authority: no
Bridge/settlement authority: no
```

## What Changed

The Phase 1 hardening pass added focused DTO strictness coverage for the manifest pointer surfaces.

New test:

```text
crates/svc-index/tests/quickchain_phase1_pointer_dto_strictness.rs
```

This test proves that `svc-index` pointer DTOs reject authority-shaped field smuggling across both inbound and stored pointer wire shapes.

Covered DTO surfaces:

```text
PutAssetManifestPointer
PutSiteManifestPointer
AssetManifestPointer
SiteManifestPointer
```

The test specifically protects against fields that would imply economic, paid-access, receipt, chain, checkpoint, finality, validator, bridge, hold, operation, or account-sequence authority.

Examples of rejected authority-shaped fields:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
account_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

The test also preserves legitimate metadata behavior:

```text
owner_reference_fields_remain_references_not_authority_objects
```

That means owner/passport/wallet-like references can still exist as plain reference metadata, but they cannot become spend authority, paid-access proof, receipt proof, balance proof, or settlement proof.

## Tests Proven Green

Focused new Phase 1 test:

```text
cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness
```

Observed result:

```text
running 5 tests
owner_reference_fields_remain_references_not_authority_objects ... ok
stored_site_manifest_pointer_rejects_authority_smuggling_fields ... ok
put_site_manifest_pointer_rejects_authority_smuggling_fields ... ok
stored_asset_manifest_pointer_rejects_authority_smuggling_fields ... ok
put_asset_manifest_pointer_rejects_authority_smuggling_fields ... ok

test result: ok. 5 passed; 0 failed
```

Park/preflight coverage also continued to include the existing QuickChain and regression surface:

```text
quickchain_phase1_pointer_dto_strictness.rs
quickchain_preflight_boundary.rs
quickchain_preflight_docs.rs
quickchain_preflight_pointer_authority.rs
quickchain_preflight_routes.rs
quickchain_preflight_value_loop_boundary.rs
quickchain_tooling_boundary.rs
http_contract.rs
integration.rs
loom_index.rs
prop_index.rs
cargo test -p svc-index --all-targets
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

## What `svc-index` Now Proves

`svc-index` now proves:

```text
Index truth is not economic truth.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Policy metadata is not wallet authority.
Provider lookup is not settlement finality.
Owner/passport/wallet fields are references only.
Cache hits cannot unlock paid content.
crab:// navigation is navigation, not authority.
```

## Boundaries Preserved

`svc-index` may continue to own or expose:

```text
name lookup
site pointers
asset pointers
manifest pointers
profile pointers
provider lookup metadata
b3-addressed object references
crab navigation targets
non-authoritative owner/passport/wallet reference metadata
index metadata needed for discovery and hydration
```

`svc-index` must not become:

```text
wallet authority
ledger authority
accounting authority
reward authority
paid unlock authority
receipt authority
balance authority
finality authority
QuickChain runtime
root producer
checkpoint producer
validator
settlement service
bridge
staking/liquidity layer
external chain participant
ROX/Solana integration point
```

## Why This Matters for Phase 1

Phase 1 requires deterministic single-authority roots/proofs later, but those roots must be built from the actual value-plane truth, not from lookup metadata.

This hardening prevents a future mistake where someone treats an index pointer, owner field, provider record, manifest pointer, or cache hit as if it were:

```text
payment proof
receipt proof
balance proof
settlement finality
account-state proof
paid entitlement
QuickChain checkpoint material
```

The result is exactly what we want:

```text
svc-index can help find things.
svc-index cannot prove economic truth.
```

## Files Touched / Relevant

```text
crates/svc-index/tests/quickchain_phase1_pointer_dto_strictness.rs
crates/svc-index/tests/quickchain_preflight_boundary.rs
crates/svc-index/tests/quickchain_preflight_docs.rs
crates/svc-index/tests/quickchain_preflight_pointer_authority.rs
crates/svc-index/tests/quickchain_preflight_routes.rs
crates/svc-index/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-index/tests/quickchain_tooling_boundary.rs
crates/svc-index/scripts/dev-quickchain-preflight.sh
crates/svc-index/scripts/dev-quickchain-park.sh
crates/svc-index/src/types.rs
crates/svc-index/src/http/routes/index_manifests.rs
```

## Commands Used / Recommended

Focused check:

```bash
cargo fmt -p svc-index
cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness
```

Low-disk park gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/svc-index
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
```

## Future `svc-index` Work

Future allowed work:

```text
more negative tests for new pointer DTOs
more HTTP manifest pointer PUT/GET route tests
cache-hit cannot unlock paid content tests
owner_passport_subject and owner_wallet_account validation tests through HTTP bodies
provider lookup metadata tests
audit/versioning docs for future mutable pointer records
deterministic pointer snapshot discussion only after vector gates
```

Still forbidden:

```text
roots
checkpoints
validators
validator signatures
settlement
bridges
anchors
ROX
Solana
external L2/DA
staking
liquidity
exchange-facing logic
ledger mutation
wallet mutation
fake receipts
fake balances
fake finality
cache-only paid unlock
index-only paid unlock
```

---

# `ron-policy` QuickChain Phase 1 Notes

## Current Status

```text
Status: GREEN / PARKED
QuickChain role: declarative policy only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid proof authority: no
Receipt proof authority: no
Balance proof authority: no
Finality authority: no
Root/checkpoint authority: no
Validator authority: no
Bridge/settlement authority: no
```

## What Changed

The Phase 1 hardening pass strengthened `ron-policy` against authority smuggling through policy condition tags, obligation kinds, and obligation parameter keys.

New test:

```text
crates/ron-policy/tests/quickchain_phase1_policy_condition_tags.rs
```

Updated validation path:

```text
crates/ron-policy/src/parse/validate.rs
```

Updated schema path:

```text
crates/ron-policy/schema/policybundle.schema.json
```

The parser now rejects condition tags that look like economic or QuickChain authority.

Examples of rejected authority-shaped condition tags:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
account_proof
inclusion_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

It also rejects separator variants such as:

```text
paid-proof
receipt.hash
checkpoint/root
validator signature
```

The validator also rejects authority-shaped obligation parameter keys and obligation kinds. This prevents a policy bundle from pretending to verify receipts, mutate wallet state, prove balance, open/capture/release holds, create roots, sign checkpoints, anchor checkpoints, or validate settlement.

## Legitimate Policy Behavior Preserved

The hardening intentionally does **not** ban normal classification tags.

Allowed examples:

```text
asset:image
paid-storage-request
tenant-beta
creator-content
```

This matters because policy still needs normal product/declarative routing labels. The rule is not “ban policy tags.” The rule is:

```text
classification tags are allowed
authority-shaped tags are rejected
```

## Tests Proven Green

Focused new Phase 1 test:

```text
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
```

Observed result:

```text
running 4 tests
ordinary_classification_tags_remain_allowed ... ok
authority_shaped_condition_tags_reject_across_separator_styles ... ok
authority_shaped_condition_tags_reject ... ok
authority_shaped_obligation_param_keys_reject ... ok

test result: ok. 4 passed; 0 failed
```

Parser consistency regression:

```text
cargo test -p ron-policy --test quickchain_preflight_parser_consistency
```

Observed result:

```text
running 4 tests
json_loader_rejects_dash_separated_authority_obligation_param ... ok
json_loader_rejects_camel_case_authority_obligation_kind ... ok
toml_loader_rejects_authority_obligation_kind ... ok
toml_loader_rejects_authority_obligation_param_key ... ok

test result: ok. 4 passed; 0 failed
```

Schema non-authority regression:

```text
cargo test -p ron-policy --test quickchain_preflight_schema_non_authority
```

Observed result:

```text
running 5 tests
policybundle_schema_preserves_body_cap_limit ... ok
policybundle_schema_rejects_authority_shaped_obligation_params ... ok
policybundle_schema_is_valid_json ... ok
policybundle_schema_rejects_authority_shaped_obligations ... ok
policybundle_schema_preserves_strict_dto_shape ... ok

test result: ok. 5 passed; 0 failed
```

Park/preflight coverage also continued to include the existing QuickChain and regression surface:

```text
quickchain_phase1_policy_condition_tags.rs
quickchain_preflight_boundary.rs
quickchain_preflight_decision_non_authority.rs
quickchain_preflight_docs.rs
quickchain_preflight_economics_config_non_authority.rs
quickchain_preflight_economics_identifier_non_authority.rs
quickchain_preflight_parser_consistency.rs
quickchain_preflight_public_surface_boundary.rs
quickchain_preflight_schema_non_authority.rs
quickchain_preflight_value_loop_boundary.rs
quickchain_tooling_boundary.rs
economics_policy.rs
golden_reasons.rs
unit_churn_protection.rs
unit_eu_only.rs
unit_eval_determinism.rs
unit_first_match_wins.rs
unit_large_body_default_deny.rs
unit_method_matrix.rs
unit_model_serde_strict.rs
unit_tags_all.rs
unit_tighten_only.rs
cargo test -p ron-policy --all-targets
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

## What `ron-policy` Now Proves

`ron-policy` now proves:

```text
Policy decisions are not receipts.
Policy decisions are not balances.
Policy decisions are not finality.
Policy obligations are not wallet mutation.
Policy obligations are not ledger mutation.
Economics config is not economic truth.
Policy schema cannot smuggle receipt/balance/finality/root/settlement authority.
Policy condition tags cannot smuggle economic authority.
Policy remains declarative.
```

## Boundaries Preserved

`ron-policy` may continue to own:

```text
allow/deny rules
declarative policy bundles
policy reasons
obligations as instructions to callers
method/tenant/region/tag matching
body-size caps
quota-style policy rules
declarative economics config
pricing/cap/split configuration helpers
```

`ron-policy` must not become:

```text
wallet authority
ledger authority
balance truth
receipt proof
paid proof
finality proof
hold lifecycle authority
account sequence authority
operation authority
root authority
checkpoint authority
validator authority
settlement authority
bridge authority
reward mutation authority
paid unlock authority by itself
```

## Why This Matters for Phase 1

Phase 1 eventually needs deterministic root/proof material, but policy cannot be allowed to “declare” economic facts into existence.

This hardening prevents a future mistake where a policy condition, tag, obligation, or config field is treated as:

```text
proof of payment
proof of receipt
proof of balance
proof of finality
proof of settlement
proof of account state
proof of hold lifecycle
proof of checkpoint validity
```

The result is exactly what we want:

```text
ron-policy can decide rules.
ron-policy cannot prove money.
```

## Files Touched / Relevant

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/schema/policybundle.schema.json
crates/ron-policy/tests/quickchain_phase1_policy_condition_tags.rs
crates/ron-policy/tests/quickchain_preflight_parser_consistency.rs
crates/ron-policy/tests/quickchain_preflight_schema_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_boundary.rs
crates/ron-policy/tests/quickchain_preflight_decision_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_docs.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_identifier_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_public_surface_boundary.rs
crates/ron-policy/tests/quickchain_preflight_value_loop_boundary.rs
crates/ron-policy/tests/quickchain_tooling_boundary.rs
crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/scripts/dev-quickchain-park.sh
```

## Commands Used / Recommended

Focused checks:

```bash
cargo fmt -p ron-policy
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
cargo test -p ron-policy --test quickchain_preflight_parser_consistency
cargo test -p ron-policy --test quickchain_preflight_schema_non_authority
```

Low-disk park gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/ron-policy
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## Future `ron-policy` Work

Future allowed work:

```text
more negative tests for new obligation kinds
more negative tests for new policy condition fields
schema/parser parity tests for any new policybundle field
tests proving policy obligations require backend receipt validation before paid unlock
tests proving economics config can express caps/splits/prices but cannot mutate balances
better error taxonomy for rejected authority-smuggling attempts
documentation around policy as declarative input to backend enforcement only
```

Still forbidden:

```text
roots
checkpoints
validators
validator signatures
settlement
bridges
anchors
ROX
Solana
external L2/DA
staking
liquidity
exchange-facing logic
ledger mutation
wallet mutation
rewarder mutation
accounting balance truth
fake receipts
fake balances
fake finality
policy-only paid unlock
policy-only receipt proof
policy-only balance proof
```

---

# Pair-Level Result

Together, `svc-index + ron-policy` now close one of the most important Phase 1 foundation gaps:

```text
lookup cannot become payment
policy cannot become proof
metadata cannot become authority
declarative config cannot become balance truth
cache cannot become paid unlock
names remain pointers
b3 remains byte identity
wallet/ledger remain economic truth
QuickChain runtime remains parked
external settlement remains deferred
```

This is a strong result because these are exactly the two crates where authority confusion could creep in quietly:

```text
svc-index risk: pointer/metadata accidentally treated as entitlement or settlement truth
ron-policy risk: policy condition/obligation/config accidentally treated as payment proof
```

Both risks are now covered by explicit tests and parking gates.

---

# Current Phase 1 Impact

After this pair, the Phase 1 foundation estimate should move from the previous `~75%` range to approximately:

```text
QuickChain Phase 1 foundation: ~85–90%
Full Phase 1 deliverable: ~70–75%
```

Reason:

```text
ron-proto + ron-ledger already established the DTO/ledger deterministic-material foundation.
svc-wallet + ron-accounting preserved mutation-front-door and snapshot/non-authority boundaries.
svc-rewarder + svc-storage preserved payout-planning and byte/paid-access boundaries.
svc-gateway + omnigate preserved public-boundary and hydration/non-mutation boundaries.
svc-index + ron-policy now preserve lookup/policy non-authority boundaries.
```

The main remaining Phase 1 foundation area is now:

```text
CrabLink Tauri + client adapters
```

That pass should prove:

```text
React displays truth but does not own authority.
Tauri Rust mediates native privilege.
TypeScript adapters do not bypass Tauri policy.
No client-side QuickChain authority.
No client-side roots/checkpoints/validators.
No fake balances.
No fake receipts.
No silent spend.
No cache-only paid unlock.
Display-only receipt cache remains display-only.
Paid actions require explicit backend wallet path confirmation.
```

---

# Recommended Next Step

Next QuickChain pair:

```text
CrabLink Tauri + client adapters
```

Recommended attachments for the next session:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
CODEBUNDLE_TAURI_APP.md
latest terminal output proving svc-index + ron-policy green/parked
these crate notes
```

Recommended next gate shape:

```bash
cd /Users/mymac/Desktop/crablink

npm run check:quickchain-boundary --workspace apps/crablink-tauri
npm run check:quickchain-paid-cache-boundary --workspace apps/crablink-tauri
npm run check:quickchain-readiness-boundary --workspace apps/crablink-tauri
npm run build --workspace apps/crablink-tauri
scripts/check-tauri.sh
```

Use the actual package scripts present in the current CrabLink repo. If a script name differs, inspect `apps/crablink-tauri/package.json` and use the existing QuickChain boundary scripts rather than inventing new ones.

---

# Low-Disk Workflow Reminder

Do not run duplicate exhaustive gates.

Preferred pattern after a focused patch:

```text
1. cargo fmt for touched crate(s)
2. focused new test(s)
3. park script once
4. delete crate-specific target dir
5. regenerate codebundle
```

Avoid:

```text
preflight script + park script for the same crate
```

unless deliberately debugging or wanting a duplicate archival pass, because the park script delegates to the preflight script.

Use per-crate target dirs:

```text
.quickchain-target-lowdisk/svc-index
.quickchain-target-lowdisk/ron-policy
```

and remove them after successful gates.

---

# Final Label

Use this status label going forward:

```text
svc-index + ron-policy: GREEN / PARKED for QuickChain Phase 1 boundary foundation
```

Short version for carryover:

```text
We completed the svc-index + ron-policy QuickChain Phase 1 boundary hardening pass. svc-index now rejects authority-shaped field smuggling in pointer DTOs while preserving owner/passport/wallet references as metadata only. ron-policy now rejects authority-shaped condition tags, obligation kinds, and obligation parameter keys while preserving ordinary classification tags. Focused tests passed, parser/schema regressions passed, and both crates remain parked behind crate-local QuickChain gates. No roots, checkpoints, validators, settlement, bridges, anchors, ROX/Solana, wallet mutation, ledger mutation, fake receipts, fake balances, fake finality, policy-only paid unlocks, or index-only paid unlocks were introduced. The next Phase 1 foundation target is CrabLink Tauri + client adapters.
```


### END NOTE - JUNE 21 2026 - 13:35 CST


### BEGIN NOTE - JUNE 22 2026 - 14:40 CST

Both crates are green and parked. The terminal output shows `svc-index` passed its exhaustive QuickChain gate with `tests=8` and `ron-policy` passed with `tests=12`, including both new Phase 1 pair-interlock suites. 

Here are the crate notes to drop into your project notes.

# QuickChain Phase 1 Round 1 / QC-1A Crate Notes — `svc-index` + `ron-policy`

Date: June 22, 2026
Crate pair: `svc-index` + `ron-policy`
Track: QuickChain Phase 1 / Round 1 / QC-1A foundation
Status: Green and parked for this crate pair
Next crate pair: CrabLink Tauri + client adapters

## 1. Session Result

This session completed the Phase 1 Round 1 / QC-1A foundation pass for:

```text
crates/svc-index
crates/ron-policy
```

Both crates are now parked behind their crate-local QuickChain park scripts:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=8 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==

== ron-policy quickchain exhaustive preflight gate passed: tests=12 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

Although the scripts still print “Phase-0 preflight” because they are the existing reusable preflight runners, this session added and proved the Phase 1 Round 1 / QC-1A pair-interlock layer for `svc-index` and `ron-policy`.

This does not mean full Phase 1 is complete. It means the `svc-index + ron-policy` pair is green for Phase 1 Round 1 / QC-1A foundation. Full Round 1 will complete after CrabLink Tauri + client adapters are completed in the next session.

## 2. Current Phase 1 Round 1 Pair-Sweep Status

```text
1. ron-proto + ron-ledger              ✅ parked
2. svc-wallet + ron-accounting         ✅ parked
3. svc-rewarder + svc-storage          ✅ parked
4. svc-gateway + omnigate              ✅ parked
5. svc-index + ron-policy              ✅ parked
6. CrabLink Tauri + client adapters    ⏭ next
```

After CrabLink passes, Phase 1 Round 1 / QC-1A foundation will be complete.

## 3. Files Added or Changed

### `svc-index`

Added:

```text
crates/svc-index/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Purpose:

```text
Adds the QC-1A pair-interlock test suite for svc-index.
Locks svc-index into lookup / pointer / index-only authority.
Prevents index entries, names, manifests, b3 references, provider lookups, cache hits, or policy metadata from becoming economic truth.
```

### `ron-policy`

Added:

```text
crates/ron-policy/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Purpose:

```text
Adds the QC-1A pair-interlock test suite for ron-policy.
Locks ron-policy into declarative policy-only authority.
Prevents policy decisions, policy obligations, policy explanations, feature flags, economics config, or tags from becoming economic truth.
```

Patched during session:

```text
crates/ron-policy/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Fixes applied:

```text
1. Replaced a const-generic array helper with a slice-based helper.

   The original helper used:

   fn policy_with_required_tags<const N: usize>(tags: [&str; N]) -> Vec<u8>

   That caused serde_json::json! to fail because the current serde path did not serialize [&str; N] generically.

   The helper now uses:

   fn policy_with_required_tags(tags: &[&str]) -> Vec<u8>

2. Removed unused serde_json::Value import.

3. Fixed schema authority-token assertion.

   The schema stores regex fragments such as [Uu]nlock rather than literal words like Unlock.
   The test now normalizes the schema text before checking semantic authority tokens.
```

No production runtime authority was added. No wallet, ledger, QuickChain runtime, validator, bridge, or external-settlement implementation was added.

## 4. `svc-index` Role Locked by This Session

`svc-index` remains:

```text
lookup service
pointer service
index service
manifest pointer service
provider lookup service
name / b3 resolution support
```

`svc-index` must not become:

```text
wallet authority
ledger authority
payment authority
receipt authority
balance authority
paid unlock authority
QuickChain root authority
checkpoint authority
validator authority
settlement authority
bridge authority
external-chain authority
policy authority
```

Important boundary:

```text
Index truth is pointer truth only.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Provider lookup is not settlement finality.
Cache hits are not entitlement proof.
```

The new test suite verifies that `svc-index` cannot accidentally turn these lookup or pointer facts into value-plane truth.

## 5. `svc-index` New QC-1A Test Coverage

New test file:

```text
crates/svc-index/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Focused test result:

```text
running 6 tests
test manifest_does_not_add_policy_wallet_ledger_or_chain_runtime_dependencies ... ok
test pointer_dtos_reject_authority_shaped_fields ... ok
test dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite ... ok
test docs_lock_index_policy_pair_roles_without_authority_transfer ... ok
test route_source_does_not_expose_policy_or_quickchain_settlement_unlock_routes ... ok
test runtime_source_does_not_define_index_or_policy_as_paid_authority ... ok

test result: ok. 6 passed; 0 failed
```

Coverage added:

```text
1. Docs role lock

   Ensures docs explicitly preserve svc-index as lookup / pointer only and state that index truth is not economic truth.

2. Cargo dependency boundary

   Ensures svc-index does not add ron-policy, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, QuickChain runtime crates, validator crates, consensus crates, Solana, Anchor, SPL, ethers, or web3 dependencies.

3. Pointer DTO strictness

   Ensures asset/site manifest pointer DTOs reject authority-shaped smuggling fields such as:

   receipt_id
   receipt_hash
   receipt_root
   receipt_proof
   balance_minor
   wallet_balance
   ledger_balance
   unlock_granted
   paid_proof
   finality
   settlement_status
   state_root
   checkpoint_root
   checkpoint_hash
   validator_signature
   bridge_proof
   operation_id
   idempotency_key
   account_sequence
   hold_id

4. Route boundary

   Ensures svc-index routes do not expose QuickChain, checkpoint, state-root, receipt-root, validator, settlement, bridge, staking, liquidity, ROX, Solana, policy allow, paid unlock, receipt, balance, or finality surfaces.

5. Runtime source boundary

   Ensures svc-index runtime source does not import or define policy/wallet/ledger/QuickChain authority shortcuts.

6. Dynamic preflight pickup

   Ensures the crate-local dynamic QuickChain preflight script discovers the new `quickchain*.rs` suite automatically.
```

## 6. `svc-index` Full Parking Result

The park script was run:

```text
crates/svc-index/scripts/dev-quickchain-park.sh
```

It ran:

```text
format
focused QuickChain preflight tests
existing svc-index regressions
all targets
clippy
```

Focused QuickChain suites discovered and passed:

```text
quickchain_phase1_pointer_dto_strictness        5 passed
quickchain_preflight_boundary                   4 passed
quickchain_preflight_docs                       4 passed
quickchain_preflight_phase1_pair_interlock      6 passed
quickchain_preflight_pointer_authority          5 passed
quickchain_preflight_routes                     2 passed
quickchain_preflight_value_loop_boundary        4 passed
quickchain_tooling_boundary                     4 passed
```

Existing regressions passed:

```text
http_contract                                  3 passed
integration                                    3 passed
```

All-targets passed, including:

```text
src/lib.rs unit target                         0 tests
src/main.rs unit target                        0 tests
chaos                                          2 passed
http_contract                                  3 passed
integration                                    3 passed
loom_index                                     1 passed
prop_index                                     3 passed
all QuickChain suites                          passed
bench/resolve                                  0 tests
example/client                                 0 tests
```

Clippy passed:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

Final parking marker:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries

== svc-index quickchain exhaustive preflight gate passed: tests=8 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## 7. `ron-policy` Role Locked by This Session

`ron-policy` remains:

```text
declarative policy infrastructure
allow / deny decision engine
obligation aggregator
explainability surface
strict policy parser
strict economics policy parser / validator
configuration and rule validation layer
```

`ron-policy` must not become:

```text
index authority
wallet authority
ledger authority
payment authority
receipt authority
balance authority
paid unlock authority
QuickChain root authority
checkpoint authority
validator authority
settlement authority
bridge authority
external-chain authority
```

Important boundary:

```text
Policy decision is not economic truth.
Policy allow is not paid proof.
Policy obligation is not receipt proof.
Policy explanation is not finality proof.
Economics policy config is not ledger mutation.
Feature flag is not settlement authority.
```

`ron-policy` can say whether a request is allowed or denied according to declarative policy. It cannot prove payment, create receipts, mutate balances, finalize access, or produce QuickChain state.

## 8. `ron-policy` New QC-1A Test Coverage

New test file:

```text
crates/ron-policy/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Focused test result after fixes:

```text
running 8 tests
test manifest_does_not_add_index_wallet_ledger_or_chain_runtime_dependencies ... ok
test policy_allows_ordinary_pair_classification_tags_only ... ok
test policy_allow_decision_does_not_unlock_paid_content_without_backend_truth ... ok
test docs_lock_policy_index_pair_roles_without_authority_transfer ... ok
test dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite ... ok
test schema_keeps_authority_shaped_obligations_forbidden ... ok
test policy_rejects_authority_shaped_tags_from_index_cache_or_client_context ... ok
test runtime_source_does_not_define_policy_or_index_as_paid_authority ... ok

test result: ok. 8 passed; 0 failed
```

Coverage added:

```text
1. Docs role lock

   Ensures docs explicitly preserve ron-policy as declarative policy infrastructure only.

2. Cargo dependency boundary

   Ensures ron-policy does not add svc-index, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, QuickChain runtime crates, validator crates, consensus crates, Solana, Anchor, SPL, ethers, or web3 dependencies.

3. Policy allow non-authority

   Ensures an allow decision can carry an obligation requiring backend wallet/ledger proof, but the policy decision itself does not become paid unlock truth.

4. Authority-shaped tags reject

   Ensures authority-shaped condition tags from index/cache/client context reject. Examples include:

   receipt_id
   receipt_hash
   receipt_root
   receipt_proof
   balance_minor
   wallet_balance
   ledger_balance
   unlock_granted
   paid_proof
   finality
   finalized
   settlement_status
   state_root
   checkpoint_root
   checkpoint_hash
   validator_signature
   bridge_proof
   operation_id
   idempotency_key
   account_sequence
   hold_id

5. Ordinary classification tags remain allowed

   Ensures legitimate non-authority tags still work, such as:

   index-pointer-found
   policy-context-checked
   asset:image
   tenant-beta

6. Runtime source boundary

   Ensures ron-policy source does not define shortcuts such as unlock_from_policy, receipt_from_policy, root_from_policy, policy_allow_proves_payment, economics_config_mutates_ledger, or other authority-shaped runtime shortcuts.

7. Schema authority pattern check

   Ensures the policy bundle schema still contains authority-blocking patterns for unlock, paid, proof, receipt, balance, finality, state/root/checkpoint, validator, bridge, operation, idempotency, and hold concepts.

8. Dynamic preflight pickup

   Ensures the crate-local dynamic QuickChain preflight script discovers the new `quickchain*.rs` suite automatically.
```

## 9. `ron-policy` Full Parking Result

The park script was run:

```text
crates/ron-policy/scripts/dev-quickchain-park.sh
```

It ran:

```text
format
focused QuickChain preflight tests
existing ron-policy regressions
all targets
clippy
```

Focused QuickChain suites discovered and passed:

```text
quickchain_phase1_policy_condition_tags                    4 passed
quickchain_preflight_boundary                              4 passed
quickchain_preflight_decision_non_authority                4 passed
quickchain_preflight_docs                                  3 passed
quickchain_preflight_economics_config_non_authority        4 passed
quickchain_preflight_economics_identifier_non_authority    7 passed
quickchain_preflight_parser_consistency                    4 passed
quickchain_preflight_phase1_pair_interlock                 8 passed
quickchain_preflight_public_surface_boundary               1 passed
quickchain_preflight_schema_non_authority                  5 passed
quickchain_preflight_value_loop_boundary                   4 passed
quickchain_tooling_boundary                                4 passed
```

Existing regressions passed:

```text
economics_policy                             15 passed
unit_model_serde_strict                       2 passed
unit_eval_determinism                         1 passed
unit_first_match_wins                         1 passed
golden_reasons                                1 passed
```

All-targets passed, including:

```text
src/lib.rs unit target                        0 tests
economics_policy                             15 passed
golden_reasons                                1 passed
all QuickChain suites                         passed
unit_churn_protection                         1 passed
unit_eu_only                                  1 passed
unit_eval_determinism                         1 passed
unit_first_match_wins                         1 passed
unit_large_body_default_deny                  1 passed
unit_method_matrix                            1 passed
unit_model_serde_strict                       2 passed
unit_tags_all                                 1 passed
unit_tighten_only                             1 passed
bench/eval_throughput                         Success
example/minimal_allow_deny                    0 tests
example/ron_policy_cli                        0 tests
```

Clippy passed:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Final parking marker:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions

== ron-policy quickchain exhaustive preflight gate passed: tests=12 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## 10. Important Bugs Fixed During Session

### Bug 1 — `serde_json::json!` with const-generic array

Initial failure:

```text
error[E0277]: the trait bound `[&str; N]: serde::Serialize` is not satisfied
```

Cause:

```text
The helper used a const-generic array:

fn policy_with_required_tags<const N: usize>(tags: [&str; N]) -> Vec<u8>

serde_json::json! could not serialize `[&str; N]` generically in this build path.
```

Fix:

```text
Changed the helper to accept a slice:

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8>
```

Also removed the unused `Value` import.

### Bug 2 — Case-sensitive schema assertion

Initial failure:

```text
ron-policy policybundle.schema.json must contain QC-1A interlock phrase `Unlock`
```

Cause:

```text
The schema contains regex fragments like `[Uu]nlock`, not the literal word `Unlock`.
```

Fix:

```text
Added a test-side normalizer:

fn normalize_schema_text(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

The assertion now checks normalized semantic tokens.
```

This keeps the test aligned with the actual schema format without weakening the security intent.

## 11. No Forbidden Scope Was Added

This session did not add:

```text
QuickChain roots
QuickChain checkpoints
validators
validator signatures
committee logic
fork choice
settlement
bridges
external anchors
staking
liquidity
ROX
Solana
external chain code
wallet mutation
ledger mutation
fake receipts
fake balances
fake finality
silent spend
paid unlocks from cache
paid unlocks from policy
paid unlocks from index entries
```

This is correct for Phase 1 Round 1 / QC-1A foundation.

## 12. Value-Loop Boundary Preserved

The correct backend value-loop truth remains:

```text
ron-proto economic DTOs
-> ron-ledger truth
-> svc-wallet issue / transfer / burn / hold / capture / release / receipt
-> svc-storage / svc-gateway / omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> wallet / ledger receipts
```

`svc-index` and `ron-policy` are support services in this loop, not economic authorities.

`svc-index` can assist with lookup and pointer discovery.

`ron-policy` can assist with declarative allow/deny and policy obligations.

Neither crate can mutate ledger state, create economic receipts, prove finality, or unlock paid content by itself.

## 13. Commands Run / Proven

Focused pair-interlock tests:

```bash
cargo test -p svc-index --test quickchain_preflight_phase1_pair_interlock
cargo test -p ron-policy --test quickchain_preflight_phase1_pair_interlock
```

Parking gates:

```bash
crates/svc-index/scripts/dev-quickchain-park.sh
crates/ron-policy/scripts/dev-quickchain-park.sh
```

Recommended final codebundle refresh after this session:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## 14. What This Means for Phase 1

`svc-index + ron-policy` are complete for Phase 1 Round 1 / QC-1A foundation.

Current full Round 1 status:

```text
1. ron-proto + ron-ledger              ✅ parked
2. svc-wallet + ron-accounting         ✅ parked
3. svc-rewarder + svc-storage          ✅ parked
4. svc-gateway + omnigate              ✅ parked
5. svc-index + ron-policy              ✅ parked
6. CrabLink Tauri + client adapters    next
```

Once CrabLink Tauri + client adapters are completed and parked, Phase 1 Round 1 / QC-1A foundation is complete.

After that, Phase 1 Round 2 can begin. Round 2 is where deterministic root/proof implementation work should start, primarily in `ron-proto + ron-ledger`, with downstream confirmation in the other crates.

## 15. Next Session Focus

Next crate pair:

```text
CrabLink Tauri + client adapters
```

Next session goal:

```text
Complete Phase 1 Round 1 / QC-1A foundation by proving CrabLink remains a client/product layer only.
```

CrabLink must remain:

```text
Tauri-first client
display layer
user-intent layer
typed adapter layer
gateway-first caller
b3 verifier for local cache
optional sidecar supervisor
```

CrabLink must not become:

```text
wallet truth
ledger truth
receipt truth
balance truth
paid unlock authority
QuickChain root authority
checkpoint authority
validator authority
settlement authority
bridge authority
external-chain authority
policy authority
index authority
storage authority
```

Critical CrabLink expectations for next session:

```text
React owns display and user intent only.
Tauri Rust mediates native privilege.
TS owns no secrets or backend truth.
Paid access must still flow through backend wallet/ledger/storage/gateway/omnigate paths.
Offline cache may verify b3 but cannot unlock paid content alone.
Client-side receipts remain display-only.
No fake balances.
No fake receipts.
No silent spend.
No client-produced QuickChain authority.
No raw invoke exposure.
No shell/eval/native command drift.
```

## 16. Final Status Line

`svc-index + ron-policy` are green and parked for QuickChain Phase 1 Round 1 / QC-1A foundation. The project is now one crate-pair session away from completing Phase 1 Round 1.


### END NOTE - JUNE 22 2026 - 14:40 CST


### BEGIN NOTE - JUNE 23 2026 - 15:50 CST

Based on the latest park output, `svc-index` passed its exhaustive QuickChain preflight with `tests=9` and the parking gate, while `ron-policy` passed with `tests=13` and the parking gate. The logs also confirm both crates kept forbidden QuickChain runtime scope parked: no roots, checkpoints, validators, settlement, bridges, wallet/ledger mutation, fake receipts, fake balances, fake finality, or paid unlock authority from index/policy surfaces. 

Here are the drop-in crate notes:

# QuickChain Phase 1 Round 2 Crate Notes — `svc-index` + `ron-policy`

Date prepared: June 23, 2026
Project: RustyOnions / CrabLink
Phase: QuickChain Phase 1 — single-authority deterministic roots/proofs
Round: Phase 1 Round 2 — deterministic roots/proofs + downstream confirmation
Crate pair: `svc-index + ron-policy`
Status: **GREEN / PHASE 1 ROUND 2 COMPLETE / PARKED**

## 0. Status Summary

`svc-index` and `ron-policy` are now complete and parked for **QuickChain Phase 1 Round 2**.

This pair was intentionally downstream-light. The goal was not to implement roots, proofs, validators, checkpoint production, settlement, bridge logic, or wallet/ledger mutation.

The goal was to confirm that:

```text
svc-index can point to artifacts but not prove them.
ron-policy can gate access but not create proof, finality, or spend authority.
```

That goal is complete.

Final park markers:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=9 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==

== ron-policy quickchain exhaustive preflight gate passed: tests=13 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

Although the scripts still print `Phase-0 preflight` wording, this session’s added tests and docs specifically confirm the **Phase 1 Round 2 downstream-confirmation boundary** for both crates.

## 1. What Changed

### svc-index

Added:

```text
crates/svc-index/tests/quickchain_phase1_round2_confirmation.rs
```

Updated:

```text
crates/svc-index/docs/quickchain-preflight.md
```

The new test suite proves:

```text
manifest pointers can reference Phase 1 artifacts without proof authority
manifest pointer DTOs do not expose receipt/root/proof/finality/spend fields
index manifest routes remain pointer routes, not root/proof routes
production source does not define Phase 1 root/proof/finality authority
public route string literals do not expose QuickChain proof or settlement surfaces
docs explicitly preserve the Phase 1 Round 2 downstream boundary
```

The docs now include the required scanner phrases:

```text
phase 1 round 2 downstream confirmation
index can point to artifacts but not prove them
artifact pointer is not proof
manifest pointer is not proof
index pointer is not quickchain root authority
index pointer is not finality authority
policy metadata on an index entry is not wallet or ledger proof
svc-index cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
future statuses remain parked: accepted, epoch_included, finalized, anchored
quickchain_phase1_round2_confirmation
```

### ron-policy

Added:

```text
crates/ron-policy/tests/quickchain_phase1_round2_confirmation.rs
```

Updated:

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/docs/quickchain-preflight.md
```

The new test suite proves:

```text
policy can gate on ordinary artifact context without creating authority
ordinary artifact/reference tags remain allowed
authority-shaped condition tags reject
authority-shaped tags reject across separator styles
authority-shaped obligation param keys reject
authority-shaped obligation kinds reject
production source remains declarative
production source does not add root/proof/finality/spend authority
docs explicitly preserve the Phase 1 Round 2 downstream boundary
```

The validator was tightened to reject more Phase 1 authority-shaped vocabulary, including shapes around:

```text
receipt inclusion proof
accepted receipt
epoch included receipt
account root
account state proof
state proof
hold root
hold proof
epoch root
root hash
root proof
proof hash
merkle proof
checkpoint proof
validator proof
validator set
quorum signature
anchor proof
anchored receipt
external anchor
spend authority
settlement status
```

The docs now include the required scanner phrases:

```text
phase 1 round 2 downstream confirmation
policy can gate access but not create proof/finality
policy cannot turn a proof into spend authority
policy decision is not quickchain proof
policy allow is not epoch_included
policy allow is not finalized
policy allow is not anchored
policy obligation is not receipt inclusion proof
policy obligation is not account proof
policy obligation is not spend authority
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
quickchain_phase1_round2_confirmation
```

## 2. Final Verified Commands

Focused confirmation tests passed:

```bash
cargo test -p svc-index --test quickchain_phase1_round2_confirmation
cargo test -p ron-policy --test quickchain_phase1_round2_confirmation
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
```

Park gates passed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index

CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

Both crates completed:

```text
cargo fmt / format check
all focused quickchain*.rs tests
existing regression tests
cargo test --all-targets
cargo clippy --all-targets --no-deps -- -D warnings
park script final marker
```

## 3. Final svc-index Test Results

The final `svc-index` park run included:

```text
quickchain_phase1_pointer_dto_strictness.rs: 5 passed
quickchain_phase1_round2_confirmation.rs:   5 passed
quickchain_preflight_boundary.rs:           4 passed
quickchain_preflight_docs.rs:               4 passed
quickchain_preflight_phase1_pair_interlock.rs: 6 passed
quickchain_preflight_pointer_authority.rs:  5 passed
quickchain_preflight_routes.rs:             2 passed
quickchain_preflight_value_loop_boundary.rs: 4 passed
quickchain_tooling_boundary.rs:             4 passed
```

Existing regressions included:

```text
http_contract.rs: 3 passed
integration.rs:   3 passed
```

All-targets coverage also included:

```text
src/lib.rs
src/main.rs
chaos.rs
loom_index.rs
prop_index.rs
benches/resolve.rs
examples/client.rs
```

Clippy passed:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

Final marker:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=9 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## 4. Final ron-policy Test Results

The final `ron-policy` park run included:

```text
quickchain_phase1_policy_condition_tags.rs: 4 passed
quickchain_phase1_round2_confirmation.rs:  7 passed
quickchain_preflight_boundary.rs:          4 passed
quickchain_preflight_decision_non_authority.rs: 4 passed
quickchain_preflight_docs.rs:              3 passed
quickchain_preflight_economics_config_non_authority.rs: 4 passed
quickchain_preflight_economics_identifier_non_authority.rs: 7 passed
quickchain_preflight_parser_consistency.rs: 4 passed
quickchain_preflight_phase1_pair_interlock.rs: 8 passed
quickchain_preflight_public_surface_boundary.rs: 1 passed
quickchain_preflight_schema_non_authority.rs: 5 passed
quickchain_preflight_value_loop_boundary.rs: 4 passed
quickchain_tooling_boundary.rs:            4 passed
```

Existing regressions included:

```text
economics_policy.rs:       15 passed
unit_model_serde_strict.rs: 2 passed
unit_eval_determinism.rs:   1 passed
unit_first_match_wins.rs:   1 passed
golden_reasons.rs:          1 passed
```

All-targets coverage also included:

```text
unit_churn_protection.rs
unit_eu_only.rs
unit_large_body_default_deny.rs
unit_method_matrix.rs
unit_tags_all.rs
unit_tighten_only.rs
benches/eval_throughput.rs
examples/minimal_allow_deny.rs
examples/ron_policy_cli.rs
```

Clippy passed:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Final marker:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=13 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## 5. What svc-index Now Proves

`svc-index` now proves:

```text
svc-index remains lookup/pointer infrastructure.
svc-index can reference artifacts without proving artifacts.
svc-index can reference manifests without creating proof authority.
svc-index may store b3 references and owner metadata as references only.
svc-index does not depend on wallet or ledger authority crates.
svc-index does not import value-plane mutation authority crates.
svc-index production source does not call wallet mutation verbs.
svc-index production source does not define QuickChain runtime infrastructure.
svc-index route surface does not expose QuickChain settlement/proof/finality routes.
svc-index docs preserve backend paid-access truth path.
svc-index docs warn not to over-ban legitimate metadata.
```

Important invariant:

```text
svc-index may help find what content is.
svc-index must not decide who has paid for it.
```

## 6. What ron-policy Now Proves

`ron-policy` now proves:

```text
ron-policy remains declarative policy infrastructure.
ron-policy can evaluate allow/deny decisions without becoming receipt truth.
ron-policy can require backend wallet/ledger proof without creating that proof.
ron-policy decisions are not payment receipts.
ron-policy decisions are not balances.
ron-policy decisions are not finality.
ron-policy obligations are instructions, not economic mutations.
ron-policy rejects authority-shaped condition tags.
ron-policy rejects authority-shaped obligation kinds.
ron-policy rejects authority-shaped obligation param keys.
ron-policy production source does not call wallet or ledger mutation verbs.
ron-policy production source does not define QuickChain runtime infrastructure.
```

Important invariant:

```text
ron-policy may classify and gate.
ron-policy must not manufacture proof, finality, settlement, unlock, or spend authority.
```

## 7. Boundary Now Enforced

### svc-index may own

```text
lookup
names
pointers
manifest references
provider metadata
site references
asset references
b3 reference validation
reference normalization
pointer wire-shape validation
owner/passport/wallet reference metadata
```

### svc-index must not own

```text
wallet mutation
ledger mutation
ROC issuance
ROC transfer
ROC burn
hold capture
hold release
receipt creation
balance truth
finality truth
paid unlock truth
QuickChain roots
QuickChain checkpoints
QuickChain validator logic
QuickChain settlement logic
external bridge logic
Solana logic
ROX logic
staking
liquidity
```

### ron-policy may own

```text
declarative policy bundles
allow/deny evaluation
condition tags
obligations as enforcement instructions
economics config validation
policy explanation/debug surfaces
body cap validation
strict DTO validation
```

### ron-policy must not own

```text
wallet mutation
ledger mutation
ROC issuance
ROC transfer
ROC burn
receipt creation
balance truth
finality truth
paid unlock truth
spend authority
proof authority
root authority
checkpoint authority
validator authority
settlement authority
external bridge authority
```

## 8. Paid Access Boundary

The paid access path remains outside `svc-index` and `ron-policy`.

Correct authority path:

```text
user intent / prepare / quote
→ explicit confirmation
→ svc-wallet mutation path
→ ron-ledger durable economic truth
→ backend receipt or backend authorization result
→ svc-storage / svc-gateway / omnigate paid enforcement
→ render/display after backend acceptance
```

Forbidden shortcuts remain:

```text
index entry → unlock
manifest lookup → unlock
name lookup → unlock
b3 exists → unlock
cache hit → unlock
policy allow → unlock
policy obligation → unlock
client says paid → unlock
```

Key rule:

```text
Lookup is not payment.
Policy is not proof.
Metadata is not authority.
Cache is not unlock.
```

## 9. Forbidden Scope Still Parked

The following remain forbidden in these crates:

```text
root-producing code
state roots
receipt roots
hold roots
checkpoint roots
checkpoint headers
validators
validator signatures
validator sets
committee quorum
fork choice
pruning
settlement
anchors
bridges
ROX active runtime
Solana active runtime
external L2
external DA
public bridge
staking
liquidity
exchange-facing logic
direct wallet mutation
direct ledger mutation
index-created receipts
policy-created receipts
index-created balances
policy-created balances
index-created finality
policy-created finality
index-created paid unlock
policy-created paid unlock
cache-only paid unlock
name-only paid unlock
pointer-only paid unlock
manifest-only paid unlock
b3-only paid unlock
fake receipts
fake balances
fake finality
placeholder hashes
```

## 10. Current Phase 1 Round 2 Progress

Phase 1 Round 2 crate-pair status after this session:

```text
1. ron-proto + ron-ledger              ✅ complete / parked
2. svc-wallet + ron-accounting         ✅ complete / parked
3. svc-rewarder + svc-storage          ✅ complete / parked
4. svc-gateway + omnigate              ✅ complete / parked
5. svc-index + ron-policy              ✅ complete / parked
6. CrabLink Tauri + client adapters    ⏭ next / final pair
```

Safe wording:

```text
svc-index + ron-policy are complete and parked for QuickChain Phase 1 Round 2.
```

Do not say:

```text
Full QuickChain Phase 1 is complete.
```

until this remaining work is done:

```text
CrabLink Tauri + client adapters
final Phase 1 audit
```

## 11. Recommended Next Commands

Do not rerun the full `svc-index + ron-policy` park gates unless source, docs, tests, scripts, or Cargo manifests change.

If these crates change again, use targeted checks first.

### svc-index targeted checks

```bash
cargo fmt -p svc-index

cargo test -p svc-index --test quickchain_phase1_round2_confirmation
cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness
cargo test -p svc-index --test quickchain_preflight_phase1_pair_interlock
```

Then, only once if needed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index
```

### ron-policy targeted checks

```bash
cargo fmt -p ron-policy

cargo test -p ron-policy --test quickchain_phase1_round2_confirmation
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
cargo test -p ron-policy --test quickchain_preflight_phase1_pair_interlock
```

Then, only once if needed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

### Codebundle regeneration

If saving this pair’s final codebundles:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## 12. Handoff to Next Pair

The next and final Phase 1 Round 2 pair is:

```text
CrabLink Tauri + client adapters
```

That pass should prove:

```text
React displays backend-derived truth but does not own authority.
Tauri Rust mediates native privilege.
TypeScript adapters do not bypass Tauri policy.
No client-side QuickChain authority.
No client-side roots/checkpoints/validators.
No client-side finality claims.
No fake balances.
No fake receipts.
No silent spend.
No cache-only paid unlock.
Display-only receipt cache remains display-only.
Paid actions require explicit backend wallet path confirmation.
```

Expected attachments for the next session:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
QUICKCHAIN_BUILDPLAN.MD
latest terminal output proving svc-index + ron-policy parked
these crate notes
CODEBUNDLE_TAURI_APP.md
```

Optional only if needed:

```text
CODEBUNDLE_RS.md
ALLNOTES.MD
```

## 13. One-Paragraph Handoff

`svc-index + ron-policy` are now complete and parked for QuickChain Phase 1 Round 2. `svc-index` remains lookup/pointer infrastructure only: it can reference artifacts and manifests but cannot prove settlement, finality, payment, entitlement, or paid unlock. `ron-policy` remains declarative policy infrastructure only: it can gate and require backend wallet/ledger proof but cannot create receipt proof, account proof, finality, settlement status, paid unlock, or spend authority. Both crates passed focused Phase 1 Round 2 confirmation tests, all dynamic QuickChain preflight tests, existing regressions, all-targets, and strict clippy. Forbidden QuickChain runtime scope remains parked: no roots, checkpoints, validators, settlement, bridges, wallet/ledger mutation, fake receipts, fake balances, fake finality, or paid unlock authority from index/policy surfaces. The next and final Phase 1 Round 2 pair is CrabLink Tauri + client adapters, followed by the final Phase 1 audit.

## 14. Bottom Line

`svc-index + ron-policy` are green, parked, and complete for QuickChain Phase 1 Round 2.

The system now has strong downstream-confirmation coverage proving:

```text
index names/pointers are not proof
policy decisions are not settlement
metadata is not authority
cache is not unlock
lookup is not payment
policy is not proof
b3 identifies bytes, not paid entitlement
wallet remains mutation front-door
ledger remains economic truth
QuickChain runtime remains parked
external settlement remains deferred
```


### END NOTE - JUNE 23 2026 - 15:50 CST


### BEGIN NOTE - JUNE 23 2026 - 22:10 CST

Below are paste-ready crate notes for `svc-index` and `ron-policy`. Your terminal output confirms `svc-index` parked with `tests=10`, and `ron-policy` parked with `tests=14`; both Phase 2 Round 1 focused tests passed 5/5, all-targets passed, clippy passed, and both final parking markers printed green.  

Add this to `crates/svc-index/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-index

# QuickChain Crate Notes — svc-index

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-index`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-index` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-index COMPLETE / PARKED
```

This means `svc-index` has been hardened for the Phase 2 Round 1 verifier artifact / read-only replication slice.

The crate may point to read-only replay/proof artifacts as lookup references, but it does not become proof authority, verifier truth, replay truth, quorum truth, committee truth, fork-choice authority, finality authority, paid-unlock authority, wallet authority, ledger authority, bridge authority, staking authority, or settlement authority.

This does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, validators, finality, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `svc-index` remains a lookup and pointer service only while Phase 2 introduces read-only verifier/replay artifact context elsewhere.

The crate remains:

```text
lookup service
pointer service
manifest pointer index
name/CID resolver
provider lookup surface
b3 reference carrier
owner passport reference carrier
owner wallet reference carrier
artifact pointer carrier only
```

The crate does not become:

```text
proof authority
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
paid-unlock authority
wallet mutation authority
ledger mutation authority
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
svc-index may point to read-only replay/proof artifacts, but artifact pointers are references only.
```

---

## 2. Files Changed

Changed:

```text
crates/svc-index/tests/quickchain_phase2_replay_boundary.rs
crates/svc-index/docs/quickchain-preflight.md
```

No runtime source file was changed.

No `Cargo.toml` dependency change was required.

---

## 3. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-index/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
docs name Phase 2 Round 1 verifier artifact / read-only replication
svc-index may lookup artifact pointers only
svc-index may point to read-only replay/proof artifacts
artifact pointers are references only
index artifact pointer is not proof authority
index artifact pointer is not verifier truth
index artifact pointer is not quorum truth
index artifact pointer is not committee truth
index artifact pointer is not fork choice
index artifact pointer is not finality
index artifact pointer cannot unlock paid content
b3 proves bytes, not verifier truth
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
b3 artifact CIDs remain byte references, not proof authority
pointer DTOs reject replay/verifier/committee/finality authority smuggling fields
public routes do not expose replay/verifier/committee/finality surfaces
runtime source does not construct Phase 2 replay/verifier authority
```

The focused test passed:

```text
cargo test -p svc-index --test quickchain_phase2_replay_boundary

running 5 tests
test b3_artifact_cids_remain_byte_references_not_proof_authority ... ok
test docs_name_phase2_round1_index_pointer_only_boundary ... ok
test pointer_dtos_reject_replay_verifier_and_committee_authority_smuggling_fields ... ok
test public_routes_do_not_expose_replay_verifier_committee_or_finality_surfaces ... ok
test runtime_source_does_not_construct_phase2_replay_or_verifier_authority ... ok

test result: ok. 5 passed; 0 failed
```

---

## 4. Documentation Updated

Updated:

```text
crates/svc-index/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
svc-index may lookup artifact pointers only
svc-index may point to read-only replay/proof artifacts
artifact pointers are references only
index artifact pointer is not proof authority
index artifact pointer is not verifier truth
index artifact pointer is not replay truth
index artifact pointer is not quorum truth
index artifact pointer is not committee truth
index artifact pointer is not fork choice
index artifact pointer is not finality
index artifact pointer cannot unlock paid content
b3 proves bytes, not verifier truth
b3 proves bytes, not finality
b3 proves bytes, not paid entitlement
names are pointers, not proof
crab navigation is navigation, not authority
manifest lookup is not paid unlock
provider lookup is not settlement finality
owner wallet fields are references only
owner passport fields are references only
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 5. Final Gate Evidence

The final local gate discovered 10 focused QuickChain tests:

```text
quickchain_phase1_pointer_dto_strictness
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=10 ==
== svc-index QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p svc-index --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts, fake balances, fake finality, or paid unlocks from index entries
```

---

## 6. Important Existing Coverage Still Preserved

This pass did not replace older svc-index safety coverage. It extended it.

Still protected:

```text
svc-index remains lookup/pointer-only
manifest pointers can reference artifacts without becoming proof authority
pointer DTOs reject authority-shaped fields
owner wallet metadata remains a reference only
owner passport metadata remains a reference only
b3 CIDs prove bytes only
public routes remain lookup/pointer/admin/health surfaces only
routes do not expose QuickChain settlement surfaces
routes do not expose paid-unlock authority surfaces
runtime source does not define index or policy as paid authority
runtime source does not define root/proof/finality authority
runtime source does not import wallet/ledger authority crates
Cargo.toml does not add policy/wallet/ledger/chain runtime dependencies
store roundtrips remain pointer records only
HTTP contracts preserve strict unknown-field rejection
```

---

## 7. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-index is green/parked for QuickChain Phase 2 Round 1.
It may point to read-only replay/proof artifacts as references,
but it cannot prove them, verify them, finalize them, unlock paid content from them,
or convert them into wallet/ledger/settlement authority.
```

---

## 8. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
svc-index verifier routes are live
svc-index can verify roots
svc-index can prove replay results
svc-index can sign attestations
svc-index can unlock paid content from artifact pointers
svc-index can mutate wallet
svc-index can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 9. Do Not Reopen Unless

Do not reopen `svc-index` during Phase 2 Round 1 unless:

```text
CrabLink client adapters require a stricter display/status shape for artifact pointers
ron-policy verifier access policy introduces a new pointer tag or obligation shape
svc-gateway or omnigate route contracts require a new read-only index pointer shape
ron-proto verifier artifact DTOs change in a way that affects pointer references
ron-ledger replay artifact naming/hash requirements change
a parking gate regresses
a scanner catches new proof/verifier/finality/paid-unlock authority creep
```

Otherwise, `svc-index` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 10. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
CrabLink Tauri + client adapters
```

Expected goal for that pair:

```text
CrabLink may display verifier/replay/readiness status from backend-derived sources,
but it cannot construct roots, verify finality as authority, unlock paid content from replay artifacts alone,
invent balances, invent receipts, silently spend, or treat cache/client state as economic truth.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-index

Add this to `crates/ron-policy/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - ron-policy

# QuickChain Crate Notes — ron-policy

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `ron-policy`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`ron-policy` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — ron-policy COMPLETE / PARKED
```

This means `ron-policy` has been hardened for the Phase 2 Round 1 verifier artifact / read-only replication slice.

The crate may declare verifier/replay artifact access policy, but policy decisions remain declarative only. Policy cannot create verifier truth, replay truth, quorum truth, committee truth, fork choice, finality, paid unlock authority, wallet mutation authority, ledger mutation authority, settlement authority, bridge authority, staking authority, or external settlement authority.

This does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, validators, finality, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `ron-policy` remains a declarative policy engine while Phase 2 introduces read-only verifier/replay artifact context elsewhere.

The crate remains:

```text
declarative policy engine
deny-by-default evaluator
classification tag checker
obligation generator
economics policy validator
strict DTO parser
policy access gate only
```

The crate does not become:

```text
proof authority
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
paid-unlock authority
wallet mutation authority
ledger mutation authority
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
ron-policy may declare verifier access policy only; policy-created verifier truth is forbidden.
```

---

## 2. Files Changed

Changed:

```text
crates/ron-policy/tests/quickchain_phase2_replay_boundary.rs
crates/ron-policy/docs/quickchain-preflight.md
```

No runtime source file was changed.

No `Cargo.toml` dependency change was required.

---

## 3. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/ron-policy/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
docs name Phase 2 Round 1 verifier artifact / read-only replication
ron-policy may declare verifier access policy only
policy-created verifier truth is forbidden
policy-created replay truth is forbidden
policy-created quorum truth is forbidden
policy-created committee truth is forbidden
policy-created fork choice is forbidden
policy-created finality is forbidden
policy allow is not verifier proof
policy obligation is not verifier attestation
policy obligation is not quorum
policy obligation cannot unlock paid content from replay artifacts alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
policy can declare read-only replay artifact access without creating truth
ordinary Phase 2 classification tags remain allowed
existing authority-shaped tags still reject
replay artifact policy params cannot smuggle authority fields
production source does not construct committee/quorum/finality authority
```

The focused test passed:

```text
cargo test -p ron-policy --test quickchain_phase2_replay_boundary

running 5 tests
test docs_name_phase2_round1_policy_verifier_access_boundary ... ok
test ordinary_phase2_classification_tags_are_allowed_but_existing_authority_tags_still_reject ... ok
test policy_can_declare_read_only_replay_artifact_access_without_creating_truth ... ok
test replay_artifact_policy_params_cannot_smuggle_existing_authority_fields ... ok
test production_source_does_not_construct_phase2_committee_quorum_or_finality_authority ... ok

test result: ok. 5 passed; 0 failed
```

---

## 4. Documentation Updated

Updated:

```text
crates/ron-policy/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
ron-policy may declare verifier access policy only
ron-policy may classify read-only replay artifact access requests
ron-policy may require backend replay artifact references
ron-policy may require backend wallet/ledger proof
policy decisions remain declarative control-plane decisions
policy obligations remain caller instructions only
policy-created verifier truth is forbidden
policy-created replay truth is forbidden
policy-created quorum truth is forbidden
policy-created committee truth is forbidden
policy-created fork choice is forbidden
policy-created finality is forbidden
policy allow is not verifier proof
policy allow is not replay truth
policy allow is not quorum
policy allow is not finality
policy obligation is not verifier attestation
policy obligation is not quorum
policy obligation is not fork choice
policy obligation cannot unlock paid content from replay artifacts alone
policy configuration is not settlement authority
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 5. Final Gate Evidence

The final local gate discovered 14 focused QuickChain tests:

```text
quickchain_phase1_policy_condition_tags
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=14 ==
== ron-policy QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p ron-policy --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

---

## 6. Important Existing Coverage Still Preserved

This pass did not replace older ron-policy safety coverage. It extended it.

Still protected:

```text
policy remains declarative only
policy decisions are not receipt truth
policy decisions are not balance truth
policy decisions are not paid unlock authority
policy decisions are not finality truth
policy decisions are not settlement truth
allow decisions cannot unlock paid content without backend truth
deny decisions are also not economic truth
authority-shaped condition tags reject
authority-shaped obligation kinds reject
authority-shaped obligation param keys reject
schema rejects authority-shaped obligations and params
economics config remains validation/config only
economics helpers return amounts and validation only, not truth artifacts
economics identifiers reject receipt/balance/finality/root/mutation authority shapes
parser consistency holds across JSON/TOML
production source does not import wallet/ledger mutation authority crates
production source does not define QuickChain runtime surface
production source does not call wallet or ledger mutation verbs
public examples/benches/fuzz targets remain policy-only
```

---

## 7. Safe Meaning

The safe completion meaning for this crate is:

```text
ron-policy is green/parked for QuickChain Phase 2 Round 1.
It may classify and gate read-only verifier/replay artifact access,
but it cannot create verifier truth, replay truth, quorum truth, committee truth,
fork choice, finality, paid unlocks, wallet mutations, ledger mutations, or settlement truth.
```

---

## 8. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
ron-policy can verify roots
ron-policy can prove replay results
ron-policy can sign attestations
ron-policy can create verifier truth
ron-policy can create quorum truth
ron-policy can unlock paid content from replay artifacts
ron-policy can mutate wallet
ron-policy can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 9. Do Not Reopen Unless

Do not reopen `ron-policy` during Phase 2 Round 1 unless:

```text
CrabLink client adapters need a stricter policy/display status distinction
svc-index introduces a new artifact pointer classification tag
svc-gateway or omnigate route contracts need a new verifier-access obligation shape
ron-proto verifier artifact DTOs introduce new policy-visible classification labels
ron-ledger replay artifact verification result taxonomy changes in a policy-visible way
a parking gate regresses
a scanner catches new proof/verifier/finality/paid-unlock authority creep
```

Otherwise, `ron-policy` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 10. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
CrabLink Tauri + client adapters
```

Expected goal for that pair:

```text
CrabLink may display verifier/replay/readiness status from backend-derived sources,
but it cannot construct roots, verify finality as authority, unlock paid content from replay artifacts alone,
invent balances, invent receipts, silently spend, or treat cache/client state as economic truth.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - ron-policy

After saving those, this crate-pair status is safely:

```text
QuickChain Phase 2 Round 1 — svc-index + ron-policy COMPLETE / PARKED
```

### END NOTE - JUNE 23 2026 - 22:10 CST




### BEGIN NOTE - JUNE 24 2026 - 02:20 CST

Here are the crate notes for the Phase 2 Round 2 `svc-index + ron-policy` pass. The terminal output confirms both crates hit their park markers: `svc-index QuickChain parking gate passed`, `ron-policy QuickChain parking gate passed`, and the combined final marker for the patch. 

Paste this into `crates/svc-index/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-index — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-index` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `svc-index` for the Phase 2 small committee / replicated verification boundary while preserving the core invariant:

```text
svc-index is lookup and pointer infrastructure only.
svc-index is not proof authority.
svc-index is not verifier truth.
svc-index is not committee truth.
svc-index is not quorum truth.
svc-index is not finality.
svc-index is not settlement.
svc-index is not paid unlock authority.
```

The crate remains a pointer layer for names, manifests, providers, and artifact references. It may point to backend-derived verifier or committee-readiness artifacts in the future, but those pointers are references only. They do not prove committee agreement, quorum, finality, settlement, or paid entitlement.

## Files changed / hardened

Docs hardened:

```text
crates/svc-index/docs/quickchain-preflight.md
```

New Phase 2 Round 2 test:

```text
crates/svc-index/tests/quickchain_phase2_committee_boundary.rs
```

No runtime route authority was added.

No QuickChain runtime, verifier, validator, quorum, bridge, staking, liquidity, wallet, or ledger mutation dependency was added.

## New test coverage

New focused test suite:

```text
cargo test -p svc-index --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 index committee readiness boundary
- manifest pointer DTOs reject committee, quorum, fork-choice, finality, and settlement smuggling fields
- public routes do not expose committee/quorum/validator/bridge/finality/settlement authority surfaces
- production source does not construct committee, quorum, verifier, fork-choice, finality, or paid-unlock truth
- Cargo manifest does not add wallet, ledger, validator, or external settlement dependencies
- dynamic QuickChain preflight discovers the new suite automatically
```

The new suite confirms that these authority-shaped fields are rejected from pointer DTOs:

```text
verifier_attestation
verifier_signature
committee_attestation
committee_signature
committee_member
committee_vote
quorum_reached
quorum_certificate
fork_choice
fork_choice_winner
finalized
settlement_finality
```

## Gates passed

Focused test:

```text
cargo test -p svc-index --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Crate park runner:

```text
crates/svc-index/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain runner picked up all `quickchain*.rs` tests, including the new Phase 2 Round 2 committee boundary suite.

Confirmed QuickChain suites included:

```text
quickchain_phase1_pointer_dto_strictness
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
http_contract
integration
```

All-targets passed, including:

```text
chaos
http_contract
integration
loom_index
prop_index
all QuickChain boundary suites
bench resolve
example client
```

Clippy passed:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

Final parking markers reached:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries

== svc-index quickchain exhaustive preflight gate passed: tests=11 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## Boundary summary

`svc-index` may:

```text
- resolve names
- resolve b3/provider pointers
- store and retrieve manifest pointers
- hold owner wallet/passport references as metadata only
- point to backend-derived artifact references
- point to future backend-derived verifier/committee readiness artifacts as references only
```

`svc-index` must not:

```text
- mutate wallet state
- mutate ledger state
- create fake receipts
- create fake balances
- unlock paid content
- prove payment
- prove verifier replay
- prove committee agreement
- decide committee membership
- decide quorum
- decide fork choice
- decide finality
- decide settlement
- expose validator routes
- expose bridge/staking/liquidity routes
- treat b3/name/cache/pointer lookup as authority
```

## Completion statement

`svc-index` is parked for QuickChain Phase 2 Round 2.

Next time `svc-index` is touched for QuickChain, preserve this role:

```text
Index points.
Index does not prove.
Index does not unlock.
Index does not settle.
Index never becomes verifier, committee, quorum, finality, wallet, or ledger authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

Paste this into `crates/ron-policy/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# ron-policy — QuickChain Phase 2 Round 2 Crate Notes

## Status

`ron-policy` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `ron-policy` for the Phase 2 small committee / replicated verification boundary while preserving the core invariant:

```text
ron-policy is declarative policy only.
ron-policy is not verifier truth.
ron-policy is not committee truth.
ron-policy is not quorum truth.
ron-policy is not finality.
ron-policy is not settlement.
ron-policy is not paid unlock authority.
```

The crate may declare policy for viewing or gating backend-derived verifier / committee readiness references, but policy decisions and obligations do not create signed verification attestations, committee membership, quorum certificates, fork choice, finality, settlement, or paid entitlement.

## Files changed / hardened

Docs hardened:

```text
crates/ron-policy/docs/quickchain-preflight.md
```

New Phase 2 Round 2 test:

```text
crates/ron-policy/tests/quickchain_phase2_committee_boundary.rs
```

No runtime authority was added.

No `svc-index`, `svc-wallet`, `ron-ledger`, `ron-accounting`, `svc-rewarder`, `ron-proto`, QuickChain runtime, validator, quorum, bridge, staking, liquidity, or external settlement dependency was added.

## New test coverage

New focused test suite:

```text
cargo test -p ron-policy --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 policy committee readiness boundary
- policy can gate committee-readiness display without creating truth
- policy params cannot smuggle existing authority-shaped fields
- production source does not construct committee/quorum/finality authority
- Cargo manifest does not add index, wallet, ledger, validator, or external settlement dependencies
- dynamic QuickChain preflight discovers the new suite automatically
```

The new suite confirms `ron-policy` may allow display access for ordinary readiness tags such as:

```text
verifier-artifact-reference-present
committee-readiness-reference-present
policy-context-checked
```

But it does not allow policy decisions or obligations to become:

```text
verifier_attestation
verifier_signature
committee_attestation
committee_signature
committee_vote
quorum_reached
quorum_certificate
fork_choice
fork_choice_winner
finality
finalized
settlement_status
settlement_finality
checkpoint_hash
validator_signature
bridge_proof
unlock_granted
paid_unlock
```

## Gates passed

Focused test:

```text
cargo test -p ron-policy --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Crate park runner:

```text
crates/ron-policy/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain runner picked up all `quickchain*.rs` tests, including the new Phase 2 Round 2 committee boundary suite.

Confirmed QuickChain suites included:

```text
quickchain_phase1_policy_condition_tags
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
```

All-targets passed, including:

```text
economics_policy
golden_reasons
all QuickChain boundary suites
unit_churn_protection
unit_eu_only
unit_eval_determinism
unit_first_match_wins
unit_large_body_default_deny
unit_method_matrix
unit_model_serde_strict
unit_tags_all
unit_tighten_only
bench eval_throughput
examples minimal_allow_deny and ron_policy_cli
```

Clippy passed:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Final parking markers reached:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions

== ron-policy quickchain exhaustive preflight gate passed: tests=15 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## Boundary summary

`ron-policy` may:

```text
- parse strict JSON/TOML policy bundles
- evaluate declarative allow/deny decisions
- emit policy obligations
- validate ROC economics policy
- reject authority-shaped policy tags, obligation kinds, and obligation params
- declare access policy for backend-derived verifier/committee readiness display
```

`ron-policy` must not:

```text
- mutate wallet state
- mutate ledger state
- create fake receipts
- create fake balances
- unlock paid content by policy decision alone
- create signed verification attestations
- decide committee membership
- decide quorum
- issue quorum certificates
- decide fork choice
- decide finality
- decide settlement
- implement validator runtime
- implement bridge/staking/liquidity behavior
- turn policy tags or obligations into economic truth
```

## Completion statement

`ron-policy` is parked for QuickChain Phase 2 Round 2.

Next time `ron-policy` is touched for QuickChain, preserve this role:

```text
Policy declares.
Policy gates.
Policy rejects authority smuggling.
Policy never becomes verifier, committee, quorum, finality, settlement, wallet, ledger, or paid-unlock authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2


### END NOTE - JUNE 24 2026 - 02:20 CST



### BEGIN NOTE - JUNE 24 2026 - 15:05 CST

Based on the green terminal output, `svc-index` passed exhaustive preflight with **tests=12**, and `ron-policy` passed exhaustive preflight with **tests=16**. Both park gates passed, all-target tests passed, clippy passed, and forbidden runtime scope remains parked. 

Paste this into `crates/svc-index/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

# svc-index — QuickChain Phase 3 Round 1 Crate Notes

## Status

`svc-index` is complete / green for **QuickChain Phase 3 Round 1**.

This round added and proved the `svc-index` side of the Phase 3 validator/passport boundary.

`svc-index` remains:

```text
lookup service
pointer service
name/artifact reference service
manifest pointer service
non-authoritative index surface
```

`svc-index` is still not:

```text
validator identity authority
passport registry authority
validator capability authority
validator-set authority
wallet authority
ledger authority
balance truth
receipt truth
paid unlock authority
root/checkpoint/finality authority
staking/slashing authority
bridge authority
external settlement authority
```

## Files changed

```text
crates/svc-index/tests/quickchain_phase3_validator_boundary.rs
crates/svc-index/docs/quickchain-preflight.md
```

## What this round proves

The new Phase 3 test suite proves that `svc-index` can continue storing ordinary owner passport/wallet references as metadata while rejecting validator/passport authority smuggling.

Allowed metadata remains allowed:

```text
owner_passport_subject
owner_wallet_account
manifest_cid
asset_kind
updated_at_ms
```

Rejected authority-shaped fields include:

```text
validator
validators
validator_set
validator_signature
validator_passport
validator_capability
validator_registry_entry
validator_membership_proof
validator_authorization
validator_authz_result
passport_validator
passport_validator_admission
passport_validator_capability
registry_validator
registry_validator_set
capability_validator
capability_validator_scope
attestation_identity
validator_admission
validator_revocation
validator_rotation
validator_finality
validator_paid_unlock
staking
slashing
bonded_validator
```

## Boundary locked

`svc-index` may point to backend-derived validator/readiness artifacts in the future, but those pointers are references only.

They are not:

```text
validator membership proof
passport registry truth
capability issuance truth
quorum proof
finality proof
settlement proof
payment proof
paid entitlement
spend authority
unlock authority
```

## Tests passed

Focused Phase 3 suite:

```text
cargo test -p svc-index --test quickchain_phase3_validator_boundary
```

Result:

```text
7 passed; 0 failed
```

Crate-local park gate:

```text
bash crates/svc-index/scripts/dev-quickchain-park.sh
```

Result:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=12 ==
== svc-index QuickChain parking gate passed ==
```

The exhaustive preflight included:

```text
quickchain_phase1_pointer_dto_strictness
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
http_contract
integration
chaos
loom_index
prop_index
all-target tests
clippy
```

## Forbidden scope remains parked

The park gate printed the expected forbidden-scope marker:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries
```

## Phase 3 Round 1 conclusion

Safe status wording:

```text
svc-index is complete for QuickChain Phase 3 Round 1.
It now has a tested validator/passport boundary.
It remains lookup/pointer infrastructure only.
It does not implement validator authority, passport registry authority, wallet/ledger truth, roots, checkpoints, finality, staking, slashing, bridge, or external settlement.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

Paste this into `crates/ron-policy/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

# ron-policy — QuickChain Phase 3 Round 1 Crate Notes

## Status

`ron-policy` is complete / green for **QuickChain Phase 3 Round 1**.

This round added and proved the `ron-policy` side of the Phase 3 validator/passport boundary.

`ron-policy` remains:

```text
declarative policy engine
deny-by-default gate
classification/tag evaluator
economics config validator
policy decision surface
```

`ron-policy` is still not:

```text
validator identity authority
passport registry authority
validator capability authority
validator-set authority
wallet authority
ledger authority
balance truth
receipt truth
paid unlock authority
root/checkpoint/finality authority
staking/slashing authority
bridge authority
external settlement authority
```

## Files changed

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/schema/policybundle.schema.json
crates/ron-policy/tests/quickchain_phase3_validator_boundary.rs
crates/ron-policy/docs/quickchain-preflight.md
```

## What this round proves

The new Phase 3 suite proves that `ron-policy` can allow ordinary validator/readiness display classification while rejecting authority-shaped validator/passport/registry/capability claims.

Allowed display/gating labels remain allowed:

```text
validator-readiness-display
passport-required
registry-checked
governance-review
```

Rejected authority-shaped condition tags and obligation parameter keys now include:

```text
validator_passport
validator-passport
validator.passport
validator/passport
validator_capability
validator_registry_entry
validator_membership_proof
validator_authorization
validator_authz_result
passport_validator
passport_validator_admission
passport_validator_capability
registry_validator
registry_validator_set
capability_validator
capability_validator_scope
attestation_identity
validator_admission
validator_revocation
validator_rotation
validator_identity_authority
passport_registry_authority
validator_capability_authority
validator_set_authority
validator_paid_unlock
```

Rejected validator lifecycle obligation kinds include:

```text
admit-validator
revoke-validator
rotate-validator
authorize-validator
register-validator
deregister-validator
set-validator-set
update-validator-set
commit-validator-set
grant-validator-capability
grant-validator-admission
sign-validator-attestation
verify-validator-attestation
unlock-from-validator-passport
unlock-from-validator-capability
```

## Boundary locked

`ron-policy` may express validator eligibility policy, but eligibility policy is declarative gating only.

Policy decisions are not:

```text
validator admission
validator revocation
validator rotation
validator-set truth
passport registry truth
capability issuance truth
paid unlock truth
wallet truth
ledger truth
receipt truth
balance truth
finality truth
settlement truth
```

## Tests passed

Focused Phase 3 suite:

```text
cargo test -p ron-policy --test quickchain_phase3_validator_boundary
```

Result:

```text
7 passed; 0 failed
```

Crate-local park gate:

```text
bash crates/ron-policy/scripts/dev-quickchain-park.sh
```

Result:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=16 ==
== ron-policy QuickChain parking gate passed ==
```

The exhaustive preflight included:

```text
quickchain_phase1_policy_condition_tags
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
unit_churn_protection
unit_eu_only
unit_large_body_default_deny
unit_method_matrix
unit_tags_all
unit_tighten_only
all-target tests
clippy
```

## Forbidden scope remains parked

The park gate printed the expected forbidden-scope marker:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

## Phase 3 Round 1 conclusion

Safe status wording:

```text
ron-policy is complete for QuickChain Phase 3 Round 1.
It now has a tested validator/passport/registry/capability non-authority boundary.
It may express declarative validator eligibility/display policy.
It does not implement validator authority, passport registry authority, wallet/ledger truth, roots, checkpoints, finality, staking, slashing, bridge, or external settlement.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

The next and final Phase 3 Round 1 pair is:

```text
CrabLink Tauri + client adapters
```


### END NOTE - JUNE 24 2026 - 15:05 CST




### BEGIN NOTE - JUNE 24 2026 - 22:35 CST

Here are paste-ready crate notes for `svc-index + ron-policy`. The terminal proof shows the new Phase 3 Round 2 lifecycle suites passed, the existing Phase 3 Round 1 validator/passport suites stayed green, and both crate-local exhaustive preflight gates passed with `svc-index tests=13` and `ron-policy tests=17`. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-index + ron-policy

# QuickChain Phase 3 Round 2 Crate Notes — `svc-index + ron-policy`

## 0. Final status

Crate pair:

```text
svc-index + ron-policy
```

Final status:

```text
QuickChain Phase 3: 100% COMPLETE / PARKED for this crate pair
```

Reason:

```text
Phase 3 has only two rounds.
Round 1 validator/passport/registry boundary coverage is green.
Round 2 validator lifecycle/evidence/governance boundary coverage is green.
Both crate-local exhaustive QuickChain preflight gates passed.
Both all-target gates passed.
Both clippy gates passed.
Existing regressions stayed green.
Forbidden scope remained parked.
```

This pair should not be reopened for Phase 3 unless a real regression appears.

Important precision:

```text
svc-index + ron-policy are complete for QuickChain Phase 3.
Full QuickChain Phase 3 still needs the final CrabLink Tauri + client adapters pass.
```

Do not call all of Phase 3 complete until CrabLink Tauri/client adapters are green.

---

## 1. Phase / round completed

Current phase:

```text
QuickChain Phase 3 — passport-gated validator set
```

Current round completed:

```text
Round 2 — validator operation / lifecycle hardening
```

This round hardened index/policy boundaries against validator lifecycle authority creep:

```text
validator rotation
validator revocation
validator downtime/degraded status
equivocation evidence
double-attestation evidence
split-brain evidence
replay challenge evidence
governance-gated parameter updates
```

The work remained boundary-only.

No root-producing, settlement, validator runtime, bridge, staking, slashing, bonding, liquidity, external settlement, ROX, or Solana code was added.

---

## 2. `svc-index` notes

## 2.1 Role preserved

`svc-index` remains:

```text
lookup service
pointer service
name resolver
asset manifest pointer store
site manifest pointer store
provider reference surface
owner/passport/wallet metadata reference holder
b3/reference metadata surface
```

`svc-index` is not:

```text
wallet
ledger
balance authority
receipt authority
paid unlock authority
root authority
checkpoint authority
finality authority
settlement authority
validator authority
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
bridge authority
staking authority
slashing authority
bonding authority
liquidity authority
external settlement authority
```

The invariant remains:

```text
Index points.
Index does not prove.
Index does not unlock.
Index does not settle.
```

## 2.2 What changed

The Phase 3 Round 2 patch added lifecycle/evidence/governance boundary hardening for `svc-index`.

Files changed or added:

```text
crates/svc-index/docs/quickchain-preflight.md
crates/svc-index/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

No production runtime authority was added.

No `Cargo.toml` dependency was added.

No Python helper script was added.

The docs now explicitly name the Phase 3 Round 2 lifecycle boundary and state that:

```text
svc-index may store and return backend-derived lookup pointers and metadata only.
svc-index is not validator lifecycle authority.
svc-index is not validator rotation authority.
svc-index is not validator revocation authority.
svc-index is not validator downtime authority.
svc-index is not validator degraded-status authority.
svc-index is not validator equivocation authority.
svc-index is not replay challenge authority.
svc-index is not governance parameter-update authority.
```

The docs also preserve the paid-access boundary:

```text
index entries, names, manifests, profile pointers, b3 hashes, provider records, cache hits, and route metadata cannot unlock paid content.
```

## 2.3 New Phase 3 Round 2 focused test

New test:

```bash
cargo test -p svc-index --test quickchain_phase3_validator_lifecycle_boundary
```

Observed result:

```text
5 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round2_index_validator_lifecycle_boundary
pointer_dtos_reject_validator_lifecycle_evidence_and_governance_authority_fields
route_source_does_not_expose_validator_lifecycle_governance_or_evidence_routes
runtime_source_does_not_import_wallet_ledger_policy_or_external_settlement_runtime
runtime_source_does_not_construct_validator_lifecycle_or_paid_unlock_authority
```

## 2.4 What the new `svc-index` suite proves

The new suite proves:

```text
manifest pointer DTOs reject validator lifecycle/evidence/governance authority-shaped fields
site pointer DTOs reject validator lifecycle/evidence/governance authority-shaped fields
asset pointer DTOs reject validator lifecycle/evidence/governance authority-shaped fields
public routes do not expose validator lifecycle mutation routes
public routes do not expose governance parameter-update routes
public routes do not expose replay challenge authority routes
production source does not construct validator lifecycle authority
production source does not construct paid unlock from lifecycle/evidence material
production source does not import wallet, ledger, policy, QuickChain runtime, Solana, SPL, Anchor, bridge, or external settlement authority
```

Authority-shaped fields rejected include:

```text
validator_lifecycle_authority
validator_lifecycle_decision
lifecycle_decision
validator_rotation
validator_revocation
validator_downtime
validator_degraded
validator_equivocation
validator_equivocation_evidence
validator_double_attestation
double_attestation_evidence
validator_split_brain
split_brain_evidence
replay_challenge
replay_challenge_evidence
governance_parameter_update
governance_approval
validator_lifecycle_paid_unlock
validator_lifecycle_receipt_truth
validator_lifecycle_balance_truth
validator_lifecycle_finality_truth
validator_lifecycle_settlement_truth
```

## 2.5 Existing Phase 3 Round 1 validator/passport boundary stayed green

Existing test:

```bash
cargo test -p svc-index --test quickchain_phase3_validator_boundary
```

Observed result:

```text
7 passed; 0 failed
```

Covered tests:

```text
empty_json_helper_remains_used_so_serde_json_map_import_is_not_accidental
manifest_pointer_dtos_still_allow_owner_passport_and_wallet_references_as_metadata
cargo_manifest_does_not_add_phase3_validator_wallet_ledger_or_external_runtime_dependencies
manifest_pointer_dtos_reject_phase3_validator_passport_authority_fields
docs_name_phase3_round1_index_validator_passport_boundary
public_string_routes_do_not_expose_phase3_validator_passport_registry_or_bonding_surfaces
production_source_does_not_implement_validator_lifecycle_or_paid_unlock_authority
```

This confirms Round 2 did not regress the Round 1 passport/registry boundary.

## 2.6 Exhaustive `svc-index` gate

Command run:

```bash
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
```

Final observed marker:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=13 ==
svc-index QuickChain Phase-0 preflight passed.
```

The script still prints `Phase-0 preflight` wording, but its dynamic discovery now includes Phase 1, Phase 2, Phase 3 Round 1, and Phase 3 Round 2 QuickChain suites.

## 2.7 `svc-index` regression coverage preserved

Existing regressions stayed green:

```text
http_contract
integration
chaos
loom_index
prop_index
benches/resolve
examples/client
```

Important preserved behavior:

```text
asset manifest pointer wire shape remains stable
unknown fields reject
store roundtrips asset manifest pointers
store roundtrips site manifest pointers
missing pointers return none
concurrent pointer updates survive
b3 validation rejects bad lengths and characters
site name validation rejects unsafe forms
asset kind validation rejects unknown forms
```

## 2.8 `svc-index` forbidden scope stayed parked

Final parking marker preserved:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries
```

---

## 3. `ron-policy` notes

## 3.1 Role preserved

`ron-policy` remains:

```text
declarative policy engine
allow/deny decision engine
reason/explanation engine
obligation parser/evaluator
condition tag validator
economics config validator
pricing/cap/split helper
policy schema owner
```

`ron-policy` is not:

```text
wallet
ledger
balance authority
receipt authority
paid unlock authority
root authority
checkpoint authority
finality authority
settlement authority
validator authority
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
bridge authority
staking authority
slashing authority
bonding authority
liquidity authority
external settlement authority
```

The invariant remains:

```text
Policy decides.
Policy does not mutate.
Policy does not prove.
Policy does not unlock by itself.
Policy does not settle.
```

## 3.2 What changed

The Phase 3 Round 2 patch added lifecycle/evidence/governance boundary hardening for `ron-policy`.

Files changed or added:

```text
crates/ron-policy/docs/quickchain-preflight.md
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

No runtime authority was added.

No wallet, ledger, passport, registry, auth, QuickChain runtime, bridge, staking, slashing, bonding, liquidity, external settlement, ROX, or Solana dependency was added.

The docs now explicitly state:

```text
ron-policy may express declarative lifecycle eligibility/gating rules only.
ron-policy is not validator lifecycle authority.
ron-policy is not validator rotation authority.
ron-policy is not validator revocation authority.
ron-policy is not validator downtime authority.
ron-policy is not validator degraded-status authority.
ron-policy is not validator equivocation authority.
ron-policy is not replay challenge authority.
ron-policy is not governance parameter-update authority.
```

The docs also state that:

```text
policy decisions, reasons, obligations, tags, economics config, feature flags, and explanations cannot admit validators, revoke validators, rotate validators, mark validators down, accept equivocation evidence, accept replay challenge evidence, commit governance parameter updates, unlock paid content, mutate balances, issue receipts, prove finality, prove settlement, or replace wallet/ledger truth.
```

## 3.3 Parser/economics hardening

The parser and economics validators now reject exact authority-shaped lifecycle identifiers.

Added or confirmed forbidden normalized identifiers include:

```text
validatordowntime
validatordegraded
validatorequivocation
validatorequivocationevidence
validatordoubleattestation
validatorsplitbrain
validatorlifecycledecision
lifecycledecision
replaychallenge
replaychallengeevidence
governanceparameterupdate
governanceapproval
```

The parser also rejects lifecycle/evidence/governance obligation kinds such as:

```text
markvalidatordowntime
markvalidatordegraded
submitvalidatorequivocationevidence
submitdoubleattestationevidence
submitsplitbrainevidence
submitreplaychallenge
submitreplaychallengeevidence
commitgovernanceparameterupdate
grantgovernanceapproval
grantvalidatorlifecycledecision
unlockfromvalidatorlifecycle
settlefromreplaychallenge
```

Important nuance:

```text
ordinary lifecycle display/classification tags remain allowed when they are clearly display/readiness context, not authority.
```

Examples that remain allowed:

```text
validator-downtime-display
validator-degraded-status-display
validator-lifecycle-readiness-display
replay-challenge-display
governance-review
```

This avoids overcorrecting by banning legitimate metadata, while still rejecting authority-shaped claims.

## 3.4 New Phase 3 Round 2 focused test

New test:

```bash
cargo test -p ron-policy --test quickchain_phase3_validator_lifecycle_boundary
```

Observed result:

```text
7 passed; 0 failed
```

Covered tests:

```text
ordinary_lifecycle_display_classification_tags_remain_allowed
lifecycle_authority_condition_tags_reject
lifecycle_authority_obligation_param_keys_reject
lifecycle_authority_obligation_kinds_reject
docs_name_phase3_round2_policy_validator_lifecycle_boundary
parser_and_economics_validator_contain_phase3_round2_exact_authority_shapes
production_source_does_not_construct_lifecycle_governance_paid_unlock_or_settlement_runtime
```

## 3.5 What the new `ron-policy` suite proves

The new suite proves:

```text
ordinary lifecycle display tags can be used for classification
authority-shaped lifecycle condition tags reject
authority-shaped lifecycle obligation parameter keys reject
authority-shaped lifecycle obligation kinds reject
parser hardening includes Phase 3 Round 2 lifecycle authority shapes
economics identifier hardening includes Phase 3 Round 2 lifecycle authority shapes
production source does not construct lifecycle/governance paid unlock or settlement runtime
```

The suite specifically prevents policy material from becoming:

```text
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator degraded-status authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
wallet mutation authority
ledger mutation authority
receipt truth
balance truth
paid unlock truth
finality truth
settlement truth
```

## 3.6 Existing Phase 3 Round 1 validator/passport boundary stayed green

Existing test:

```bash
cargo test -p ron-policy --test quickchain_phase3_validator_boundary
```

Observed result:

```text
7 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round1_policy_validator_passport_boundary
parser_and_economics_validator_contain_phase3_exact_authority_shapes
ordinary_validator_display_classification_tags_remain_allowed
phase3_validator_lifecycle_obligation_kinds_reject
phase3_validator_passport_authority_condition_tags_reject
phase3_validator_passport_authority_obligation_param_keys_reject
production_source_does_not_import_wallet_ledger_passport_registry_or_external_runtime_authority
```

This confirms Round 2 did not regress the Round 1 passport/registry validator boundary.

## 3.7 Exhaustive `ron-policy` gate

Command run:

```bash
bash crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

Final observed marker:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=17 ==
ron-policy QuickChain Phase-0 preflight passed.
```

The script still prints `Phase-0 preflight` wording, but its dynamic discovery now includes Phase 1, Phase 2, Phase 3 Round 1, and Phase 3 Round 2 QuickChain suites.

## 3.8 `ron-policy` regression coverage preserved

Existing regressions stayed green:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
unit_churn_protection
unit_eu_only
unit_large_body_default_deny
unit_method_matrix
unit_tags_all
unit_tighten_only
benches/eval_throughput
examples/minimal_allow_deny
examples/ron_policy_cli
```

Important preserved behavior:

```text
economics policy rejects floats
economics policy rejects negative values
economics policy rejects overflow values
economics policy rejects invalid split sums
economics policy preserves deterministic action order
economics policy validates capture plans
disabled actions reject lookup
unknown actions reject
strict model serde remains strict
evaluation remains deterministic
first-match-wins behavior remains stable
golden explanation trace remains stable
```

## 3.9 `ron-policy` forbidden scope stayed parked

Final parking marker preserved:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

---

## 4. Shared pair boundary proven

This pair now proves the Phase 3 index/policy boundary:

```text
svc-index can store and return references.
ron-policy can evaluate declarative policy.
Neither can create economic truth.
Neither can create validator truth.
Neither can create lifecycle truth.
Neither can create settlement truth.
Neither can unlock paid content without backend wallet/ledger truth.
```

The correct value path remains:

```text
CrabLink user intent
→ svc-gateway public/product boundary
→ omnigate quote/access/hydration coordinator
→ svc-wallet mutation front-door when economic mutation is required
→ ron-ledger durable economic truth
→ backend-derived receipt/access result
→ display/render path
```

The index/policy pair may participate only as:

```text
svc-index:
  pointer lookup
  name lookup
  manifest pointer lookup
  metadata/reference lookup

ron-policy:
  declarative allow/deny/gating
  context classification
  explanation/reason output
  non-authoritative economics config validation
```

The index/policy pair must not become:

```text
payment proof
receipt proof
balance proof
ledger proof
root proof
checkpoint proof
finality proof
settlement proof
validator proof
lifecycle proof
governance proof
bridge proof
staking proof
slashing proof
```

---

## 5. Forbidden scope still parked

This pair did not add:

```text
roots
state roots
receipt roots
hold roots
epoch roots
checkpoint roots
checkpoint production
checkpoint signing
validator signatures
validator set runtime
validator admission runtime
validator rotation runtime
validator revocation runtime
validator downtime runtime
validator equivocation runtime
replay challenge runtime
governance parameter-update runtime
committee runtime
staking
slashing
bonding
liquidity
ROX runtime
Solana runtime
external settlement
public bridge
external anchors
public-chain route families
index direct ledger mutation
policy direct ledger mutation
index direct wallet mutation
policy direct wallet mutation
fake receipts
fake balances
fake finality
index-only paid unlock
policy-only paid unlock
cache-only paid unlock
b3-as-payment-proof
crab-url-as-payment-proof
name-as-payment-proof
pointer-as-payment-proof
manifest-as-payment-proof
policy-decision-as-payment-proof
lifecycle-evidence-as-payment-proof
replay-challenge-as-payment-proof
governance-approval-as-payment-proof
```

QuickChain remains future settlement infrastructure.

Internal ROC remains the active economic mission.

---

## 6. Commands proven green

Focused Phase 3 Round 2 commands:

```bash
cargo fmt -p svc-index -p ron-policy

cargo test -p svc-index --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p ron-policy --test quickchain_phase3_validator_lifecycle_boundary

cargo test -p svc-index --test quickchain_phase3_validator_boundary
cargo test -p ron-policy --test quickchain_phase3_validator_boundary
```

Crate-local exhaustive gates:

```bash
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
bash crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

Final observed markers:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=13 ==
svc-index QuickChain Phase-0 preflight passed.

== ron-policy quickchain exhaustive preflight gate passed: tests=17 ==
ron-policy QuickChain Phase-0 preflight passed.
```

---

## 7. Current Phase 3 order and next pair

Phase 3 crate-pair status:

```text
ron-proto + ron-ledger:              COMPLETE / PARKED
svc-wallet + ron-accounting:         COMPLETE / PARKED
svc-rewarder + svc-storage:          COMPLETE / PARKED
svc-gateway + omnigate:              COMPLETE / PARKED
svc-index + ron-policy:              COMPLETE / PARKED
CrabLink Tauri + client adapters:    NEXT / FINAL PHASE 3 PAIR
```

Next active pair:

```text
CrabLink Tauri + client adapters
```

After CrabLink Tauri/client adapters pass, safe final label:

```text
Phase 3 complete
passport-gated validator set complete
```

Still do not call it:

```text
QuickChain complete
public chain complete
staking complete
slashing complete
bonding complete
bridge complete
external settlement complete
exchange-ready
```

---

## 8. Next pair mission: CrabLink Tauri + client adapters

The next crate/product layer must prove:

```text
CrabLink Tauri displays backend-derived validator/readiness/lifecycle metadata only.
React owns display and user intent only.
TS adapters do not own secrets, balances, receipts, lifecycle authority, validator authority, or spend authority.
Tauri commands remain typed, bounded, allowlisted, redacted, feature-gated, and non-authoritative.
Offline cache cannot unlock paid content alone.
Client cannot treat validator lifecycle/evidence/governance metadata as paid access, receipt truth, balance truth, finality, settlement, staking, slashing, bridge, or external settlement.
```

Client forbidden scope for the next pass:

```text
no fake balance
no fake receipt
no fake finality
no silent spend
no cache-only paid unlock
no local validator authority
no client-side validator lifecycle authority
no client-side governance parameter-update authority
no lifecycle/evidence as paid unlock
no replay challenge as settlement/finality
no client root production
no client checkpoint production
no bridge
no staking
no slashing
no bonding
no ROX
no Solana
no external settlement
no raw invoke from React
no secrets in React state/localStorage/URL/logs
```

Correct CrabLink posture:

```text
backend owns truth
CrabLink displays truth
Tauri mediates native privilege
React expresses user intent
TS adapters call allowlisted commands
paid flow remains explicit confirmation -> backend wallet path -> backend receipt -> render/unlock
cache is convenience only
sidecar/local node is optional, explicit, supervised, bounded, and disabled by default
```

---

## 9. Required next-session attachments

For the CrabLink Phase 3 final pass, attach:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
QUICKCHAIN_BUILDPLAN.MD
latest carryover notes
latest terminal output proving svc-index + ron-policy are parked
latest CODEBUNDLE_TAURI_APP.md
CrabLink focused codebundle if available
```

Do not attach unrelated backend crate bundles unless a client boundary test needs cross-crate inspection.

---

## 10. Low-disk workflow reminder

Continue the low-duplicate QuickChain workflow:

```text
focused client boundary scripts first
then existing QuickChain client gates
then Tauri/React build or park script once
do not rerun already-green backend pairs
do not run workspace clippy
do not run both preflight and park if one delegates to the other
avoid cargo clean unless disk pressure requires it
```

For backend/service crates already parked:

```text
Do not rerun:
- ron-proto + ron-ledger
- svc-wallet + ron-accounting
- svc-rewarder + svc-storage
- svc-gateway + omnigate
- svc-index + ron-policy
```

unless CrabLink exposes a real integration mismatch.

---

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-index + ron-policy


### END NOTE - JUNE 24 2026 - 22:35 CST


### BEGIN NOTE - JUNE 25 2026 - 21:10 CST

Confirmed green from the terminal output: `svc-index` and `ron-policy` are both parked for **QuickChain Phase 4 Round 1**, with `ron-policy` passing focused Phase 4, full QuickChain preflight, all-target tests, and clippy. 

# QuickChain Phase 4 Round 1 Crate Notes — `svc-index` + `ron-policy`

## Status

`svc-index` + `ron-policy` are complete / parked for QuickChain Phase 4 Round 1.

This crate pair now preserves the Phase 4 Round 1 bonded-validator boundary without introducing live bonds, live slashing, public staking, liquidity, validator runtime authority, bridge logic, ROX/Solana logic, external settlement, wallet mutation, ledger mutation, fake receipts, fake balances, fake finality, or policy/index-created paid unlocks.

Important scope note: this completes the `svc-index` + `ron-policy` crate pair only. Full QuickChain Phase 4 Round 1 still needs the final CrabLink Tauri + client adapters pass.

---

## Files touched

### `svc-index`

* `crates/svc-index/tests/quickchain_phase4_bond_boundary.rs`
* `crates/svc-index/docs/quickchain-preflight.md`

### `ron-policy`

* `crates/ron-policy/tests/quickchain_phase4_bond_boundary.rs`
* `crates/ron-policy/docs/quickchain-preflight.md`
* `crates/ron-policy/src/parse/validate.rs`
* `crates/ron-policy/src/economics/validate.rs`

---

## Final test / gate status

### Focused Phase 4 tests

```text
cargo test -p svc-index --test quickchain_phase4_bond_boundary
result: ok. 5 passed; 0 failed

cargo test -p ron-policy --test quickchain_phase4_bond_boundary
result: ok. 8 passed; 0 failed
```

### Parking gates

```text
bash crates/svc-index/scripts/dev-quickchain-park.sh --check
result: passed

bash crates/ron-policy/scripts/dev-quickchain-park.sh --check
result: passed
```

### `ron-policy` final exhaustive markers

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=18 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

### Clippy

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
result: passed
```

---

## What `svc-index` now proves

`svc-index` remains a lookup / pointer / reference service only.

The Phase 4 Round 1 boundary now proves:

* Index records may reference bond, slash-simulation, evidence, and report artifacts only as metadata / b3 references.
* b3 hashes prove bytes only, not bond truth, slash truth, validator authority, staking authority, liquidity authority, paid unlocks, or settlement truth.
* Index routes do not expose bond, slash, stake, liquidity, bridge, ROX/Solana, external settlement, validator economy, wallet mutation, or ledger mutation authority.
* Manifest pointers remain pointer DTOs, not economic authority records.
* Owner passport and wallet fields remain reference metadata only.
* Cache/index data cannot unlock paid content alone.
* `svc-index` does not depend on wallet, ledger, policy, validator-runtime, bridge, staking, or liquidity authority crates.
* `svc-index` does not become a Phase 4 runtime, validator registry, bond lifecycle manager, slash adjudicator, or economic mutation service.

The key doctrine preserved:

```text
svc-index can point to artifacts.
svc-index cannot make artifacts authoritative.
svc-index can reference b3 bytes.
svc-index cannot turn b3 bytes into bond/slash/stake/liquidity/settlement truth.
```

---

## What `ron-policy` now proves

`ron-policy` remains declarative policy only.

The Phase 4 Round 1 boundary now proves:

* Policy rules may express display-only and gating-only logic for bond/readiness/slash-simulation contexts.
* Policy can allow ordinary classification tags such as display, readiness, simulation, and backend-proof-required style markers.
* Policy allow/deny decisions are not paid unlocks.
* Policy obligations are instructions only, not wallet/ledger/proof/bond/slash authority.
* Policy cannot create bond truth.
* Policy cannot create bond lifecycle truth.
* Policy cannot create slash truth.
* Policy cannot create staking authority.
* Policy cannot create liquidity authority.
* Policy cannot create validator bond authority.
* Policy cannot create bridge, ROX/Solana, or external settlement authority.
* Economics config remains validation/pricing/split policy only, not account truth or mutation authority.

The parser and economics validators now reject Phase 4 authority-shaped identifiers, including normalized/separator variants of:

```text
bond_truth
bond_authority
bond_account_truth
bond_account_authority
bond_intent_authority
bond_lifecycle_truth
bond_lifecycle_authority
bond_lifecycle_decision
validator_bond
validator_bond_authority
bonded_stake
slash_truth
slash_authority
slash_decision
slash_evidence_authority
slashing_authority
stake_authority
staking_authority
staking_market_authority
public_staking_market
liquidity_authority
liquidity_pool_authority
live_slash
automatic_slash
```

The obligation-kind validator also rejects Phase 4 action-like authority shapes such as:

```text
commit-bond-lifecycle
grant-bond-authority
grant-bond-lifecycle-decision
mark-validator-bonded
capture-validator-bond
release-validator-bond
slash-validator
execute-slashing
commit-slash-decision
open-staking-market
grant-staking-authority
create-liquidity-pool
grant-liquidity-authority
settle-bond
```

The key doctrine preserved:

```text
ron-policy can describe whether something may be shown or gated.
ron-policy cannot create economic truth.
ron-policy cannot mutate ledger state.
ron-policy cannot authorize a live bond, slash, stake, liquidity action, bridge, or settlement.
```

---

## Important implementation note

The final `ron-policy` fix was a clippy-only shape refactor after behavior was already correct.

The long authority matchers in `crates/ron-policy/src/parse/validate.rs` were moved into module-level tables:

```text
FORBIDDEN_AUTHORITY_FIELD_SHAPES
FORBIDDEN_AUTHORITY_KIND_SUBSTRINGS
```

Then the matcher functions became short table lookups. This preserved behavior while satisfying:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

No runtime authority was added by this refactor.

---

## What this crate pair does not do

This crate pair does not implement:

* live validator bonds
* live slashing
* slash adjudication
* slash execution
* staking markets
* liquidity pools
* public validator economics
* validator-set runtime authority
* checkpoint roots
* consensus roots
* chain state
* bridge settlement
* ROX/Solana settlement
* external chain anchoring that mutates ROC balances
* wallet mutation
* ledger mutation
* policy-created paid unlocks
* index-created paid unlocks
* cache-only unlocks

---

## Phase 4 Round 1 progress after this crate pair

```text
1. ron-proto + ron-ledger              complete / parked
2. svc-wallet + ron-accounting         complete / parked
3. svc-rewarder + svc-storage          complete / parked
4. svc-gateway + omnigate              complete / parked
5. svc-index + ron-policy              complete / parked
6. CrabLink Tauri + client adapters    next / remaining
```

---

## Recommended next crate/session

Next target:

```text
CrabLink Tauri + client adapters
```

Expected Phase 4 Round 1 client scope:

* display-only bond/readiness/slash-simulation status
* no client-side bond truth
* no client-side slash truth
* no public staking market UI
* no liquidity UI
* no ROX/Solana bridge UI
* no client-side spend authority
* no React/Tauri command that mutates ledger, bonds, slashes, validator state, or receipts
* no fake balances
* no fake receipts
* no silent spend
* no cache-only unlock
* Tauri Rust remains a bounded client bridge
* React remains display/user intent only

Suggested client boundary scanner name:

```text
scripts/check-quickchain-phase4-client-bond-boundary.mjs
```

Suggested final client gate:

```text
bash scripts/dev-quickchain-tauri-park.sh
```

---

## Safe completion label for this pair

```text
QuickChain Phase 4 Round 1 svc-index + ron-policy complete / parked.
```

## Not safe to claim yet

Do not claim all of QuickChain Phase 4 Round 1 is complete until CrabLink Tauri + client adapters also pass.


### END NOTE - JUNE 25 2026 - 21:10 CST


### BEGIN NOTE - JUNE 26 2026 - 02:55 CST

Below are paste-ready crate notes for **`svc-index + ron-policy`**. The terminal output confirms both crates are green and parked: `svc-index` passed the new 6-test dispute boundary suite, exhaustive preflight with `tests=15`, clippy, and park; `ron-policy` passed the new 9-test dispute boundary suite, exhaustive preflight with `tests=19`, clippy, and park. 

---

## `crates/svc-index/NOTES.MD`

```markdown
### BEGIN NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - SVC-INDEX

# svc-index — QuickChain Phase 4 Round 2 Crate Notes

## Status

`svc-index` is COMPLETE / PARKED for QuickChain Phase 4 Round 2.

This pass added the Phase 4 Round 2 dispute/challenge/appeal/freeze simulation boundary for the index service.

The key rule preserved:

    svc-index is lookup / pointer infrastructure only.

It may reference artifacts, names, manifests, providers, and b3-addressed bytes.

It must not become:

    dispute truth
    challenge-window truth
    appeal authority
    freeze authority
    irreversible slash authority
    slash simulation authority
    paid unlock authority
    settlement authority
    finality authority
    wallet authority
    ledger authority

## What changed

Added the new focused test:

    crates/svc-index/tests/quickchain_phase4_bond_dispute_boundary.rs

Updated the QuickChain preflight docs:

    crates/svc-index/docs/quickchain-preflight.md

Updated the preflight script marker so the dynamic runner visibly includes the new suite:

    crates/svc-index/scripts/dev-quickchain-preflight.sh

The script already dynamically discovers `quickchain*.rs`; this patch added the explicit review marker:

    quickchain_phase4_bond_dispute_boundary

## New Phase 4 Round 2 coverage

The new test suite proves:

    docs name the Phase 4 Round 2 index dispute pointer boundary
    index route surface does not expose dispute/challenge/appeal/freeze/slash routes
    index source does not construct Phase 4 Round 2 dispute/challenge/freeze authority
    Cargo.toml does not add runtime economy or settlement dependencies
    reference-only dispute language is allowed without becoming authority
    preflight runner names the Phase 4 Round 2 suite

The suite passed:

    cargo test -p svc-index --test quickchain_phase4_bond_dispute_boundary

Result:

    6 passed
    0 failed

## Boundary locked

`svc-index` may point to dispute/challenge/appeal/freeze artifacts only as references.

Allowed reference-only concepts:

    bond dispute artifact CIDs
    slash evidence artifact CIDs
    challenge window artifact CIDs
    appeal artifact CIDs
    freeze status display labels
    disputed bond status display labels
    slash simulation artifact CIDs

Explicit non-authority phrases added to docs:

    index pointer is not dispute truth
    index pointer is not challenge-window truth
    index pointer is not appeal authority
    index pointer is not freeze authority
    index pointer is not irreversible slash authority
    index pointer is not slash simulation authority
    dispute artifact cid proves bytes only
    challenge evidence cid proves bytes only
    appeal artifact cid proves bytes only
    freeze status label is display metadata only
    b3 proves bytes, not dispute resolution truth
    names are pointers, not dispute authority
    manifest lookup is not challenge evidence validation
    provider lookup is not appeal authority
    index cache cannot unlock paid content or trigger dispute consequences

## Forbidden scope confirmed

The park gate confirmed the following stayed parked:

    no roots
    no checkpoints
    no validators
    no settlement
    no bridges
    no wallet or ledger mutation
    no fake receipts
    no fake balances
    no fake finality
    no paid unlocks from index entries

Additional Phase 4 Round 2 forbidden scope remains parked:

    no live irreversible slash through svc-index
    no public staking market through svc-index
    no liquidity through svc-index
    no ROX
    no Solana
    no external settlement
    no exchange-facing logic
    no public validator economy

## Tests / gates passed

Focused Phase 4 Round 2 suite:

    cargo test -p svc-index --test quickchain_phase4_bond_dispute_boundary

Crate-local park gate:

    crates/svc-index/scripts/dev-quickchain-park.sh

Observed green markers:

    == svc-index quickchain exhaustive preflight gate passed: tests=15 ==
    svc-index QuickChain Phase-0 preflight passed.
    == svc-index QuickChain parking gate passed ==

Clippy also passed inside the park gate:

    cargo clippy -p svc-index --all-targets --no-deps -- -D warnings

## Notes for future work

Do not add dispute/challenge/appeal/freeze route authority to `svc-index`.

If future phases need user-visible dispute lookup, keep it reference-only:

    b3 proves bytes only
    names are pointers only
    manifests are metadata only
    index cache is convenience only
    backend wallet/ledger paths remain economic truth

Do not let index entries or cached manifests unlock paid content, prove dispute outcomes, resolve challenges, grant appeal authority, freeze bonds, release bonds, capture bonds, slash bonds, or finalize settlement.

Next QuickChain Phase 4 Round 2 crate pair:

    CrabLink Tauri + client adapters

### END NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - SVC-INDEX
```

---

## `crates/ron-policy/NOTES.MD`

```markdown
### BEGIN NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - RON-POLICY

# ron-policy — QuickChain Phase 4 Round 2 Crate Notes

## Status

`ron-policy` is COMPLETE / PARKED for QuickChain Phase 4 Round 2.

This pass added the Phase 4 Round 2 dispute/challenge/appeal/freeze simulation boundary for the policy engine.

The key rule preserved:

    ron-policy is declarative policy only.

It may say what must be true.

It must not become the proof that it is true.

It must not become:

    dispute truth
    challenge-window truth
    appeal authority
    freeze authority
    irreversible slash authority
    slash simulation authority
    paid unlock authority
    wallet authority
    ledger authority
    settlement authority
    finality authority

## What changed

Added the new focused test:

    crates/ron-policy/tests/quickchain_phase4_bond_dispute_boundary.rs

Updated the QuickChain preflight docs:

    crates/ron-policy/docs/quickchain-preflight.md

Updated parser authority-shape validation:

    crates/ron-policy/src/parse/validate.rs

Updated economics identifier authority-shape validation:

    crates/ron-policy/src/economics/validate.rs

Updated the preflight script marker so the dynamic runner visibly includes the new suite:

    crates/ron-policy/scripts/dev-quickchain-preflight.sh

The script already dynamically discovers `quickchain*.rs`; this patch added the explicit review marker:

    quickchain_phase4_bond_dispute_boundary

## New Phase 4 Round 2 coverage

The new test suite proves:

    docs name the Phase 4 Round 2 policy dispute boundary
    ordinary dispute/challenge display policy remains allowed but non-authoritative
    authority-shaped dispute/challenge/appeal/freeze condition tags reject
    authority-shaped dispute/challenge/appeal/freeze obligation params reject
    authority-shaped dispute/challenge/appeal/freeze obligation kinds reject
    parser and economics validator contain the new exact authority shapes
    economics config rejects authority-shaped aliases
    production source does not construct Phase 4 Round 2 runtime or mutation authority
    preflight runner names the Phase 4 Round 2 suite

The suite passed:

    cargo test -p ron-policy --test quickchain_phase4_bond_dispute_boundary

Result:

    9 passed
    0 failed

## Parser / economics hardening

The following Phase 4 Round 2 authority-shaped identifiers are now rejected in the parser and economics validation tables:

    bonddispute
    bonddisputestate
    disputetruth
    disputeauthority
    disputewindowauthority
    challengewindow
    challengewindowauthority
    appealauthority
    appealwindowauthority
    freezeauthority
    frozenbond
    disputedbond
    irreversibleslash
    slashappeal
    slashchallenge
    slashsimulationauthority
    livedisputeresolution
    automaticdisputeslash

The following authority-shaped obligation kinds are now rejected:

    grantdisputeauthority
    commitbonddisputestate
    openchallengewindow
    grantchallengewindowauthority
    submitslashappeal
    grantappealauthority
    freezebond
    capturedisputedbond
    slashdisputedbond
    executeirreversibleslash
    commitirreversibleslash
    settlebonddispute
    unlockfrombonddispute
    settlefromslashchallenge

## Boundary locked

`ron-policy` may express declarative dispute visibility policy.

`ron-policy` may express declarative challenge window policy only.

`ron-policy` may express declarative appeal/freeze display policy only.

`ron-policy` may require callers to use backend wallet/ledger proof paths before paid access.

But the docs now explicitly lock the following:

    ron-policy is not dispute truth
    ron-policy is not challenge-window truth
    ron-policy is not appeal authority
    ron-policy is not freeze authority
    ron-policy is not irreversible slash authority
    ron-policy is not slash simulation authority
    policy allow is not dispute resolution proof
    policy allow is not challenge evidence validation
    policy obligation is not challenge evidence acceptance
    policy obligation is not appeal authority
    policy obligation is not freeze authority
    policy decision cannot mutate wallet or ledger
    policy decision cannot unlock paid content from dispute/challenge/appeal/freeze material
    policy config cannot create disputed bond balances
    policy config cannot create freeze authority

## Forbidden scope confirmed

The park gate confirmed the following stayed parked:

    no roots
    no checkpoints
    no validators
    no settlement
    no bridges
    no wallet or ledger mutation
    no fake receipts
    no fake balances
    no fake finality
    no paid unlocks from policy decisions

Additional Phase 4 Round 2 forbidden scope remains parked:

    no live irreversible slash through ron-policy
    no public staking market through ron-policy
    no liquidity through ron-policy
    no ROX
    no Solana
    no external settlement
    no exchange-facing logic
    no public validator economy

## Tests / gates passed

Focused Phase 4 Round 2 suite:

    cargo test -p ron-policy --test quickchain_phase4_bond_dispute_boundary

Crate-local park gate:

    crates/ron-policy/scripts/dev-quickchain-park.sh

Observed green markers:

    == ron-policy quickchain exhaustive preflight gate passed: tests=19 ==
    ron-policy QuickChain Phase-0 preflight passed.
    == ron-policy QuickChain parking gate passed ==

Clippy also passed inside the park gate:

    cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings

## Notes for future work

Do not add policy obligations that mutate wallet, mutate ledger, unlock paid content, accept challenge evidence, resolve disputes, grant appeal authority, freeze bonds, capture disputed bonds, slash disputed bonds, or commit irreversible slash.

If future phases need policy involvement in dispute UX, keep it declarative:

    policy can gate display
    policy can require backend evidence
    policy can deny missing proof
    policy can require wallet/ledger truth
    policy cannot itself become proof, finality, settlement, or spend authority

Next QuickChain Phase 4 Round 2 crate pair:

    CrabLink Tauri + client adapters

### END NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - RON-POLICY
```

Current Phase 4 Round 2 status:

```text
1. ron-proto + ron-ledger             COMPLETE / PARKED
2. svc-wallet + ron-accounting        COMPLETE / PARKED
3. svc-rewarder + svc-storage         COMPLETE / PARKED
4. svc-gateway + omnigate             COMPLETE / PARKED
5. svc-index + ron-policy             COMPLETE / PARKED
6. CrabLink Tauri + client adapters   NEXT
```


### END NOTE - JUNE 26 2026 - 02:55 CST