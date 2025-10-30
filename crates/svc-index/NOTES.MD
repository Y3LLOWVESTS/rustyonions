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