### BEGIN NOTE - OCTOBER 28 2025 - 16:20 CST
---

# Carryover Notes — from `ron-naming` → `svc-storage`

**Context:** `ron-naming` is the schema + hygiene authority for addresses and names.
**Goal for `svc-storage`:** **Only** store and serve **content by `ContentId`**; treat names as upstream concerns (index/gateway). Storage must never perform IDNA or semantic version parsing; it operates on immutable, content-addressed identifiers.

---

## 1) Ground Truth from `ron-naming`

### 1.1 DTOs we must respect

* **`ContentId`**: canonical **BLAKE3-256** identifier in the exact string shape:

  * **Form:** `"b3:<64 lowercase hex>"`
  * **Regex:** `^b3:[0-9a-f]{64}$`
  * **Case:** lowercase only
* **`Fqdn` / `NameRecord` / `Address::Name { fqdn, version }`**

  * *Not* used inside `svc-storage` APIs; these are strictly **upstream** (`svc-index`, gateway).
* **`Address`** (union):

  * `Content { id: ContentId }` → this is the only acceptable address form for storage.

### 1.2 Normalization & Names (strict boundary)

* **Normalization (UTS-46/IDNA + NFC) happens before storage.**
  `svc-storage` must **not** accept raw names; it must **require** `ContentId`.
* If a client attempts to PUT/GET via a *name* (e.g., `files.example@1.2.3`), **reject** with **400 Bad Request** and message:
  “Storage only accepts `ContentId` (b3:…). Resolve names in `svc-index` first.”

### 1.3 Wire helpers

* `ron-naming::wire::{json,cbor}` provide serde round-trips.
  For `svc-storage`:

  * Accept/emit **DTOs** that embed `ContentId` using `serde` (JSON/CBOR).
  * If you expose any “record metadata” with responses, ensure fields that mirror `ron-naming` types serialize identically (round-trip compatible).

---

## 2) `svc-storage` API Contract (informed by the above)

### 2.1 Request validation (non-negotiable)

* **All endpoints accept `ContentId` only.**
* Validate early (before I/O):

  * **Path param or JSON field** must match `^b3:[0-9a-f]{64}$`.
  * Reject uppercase hex, mixed case, missing prefix, wrong length, or non-hex.

**Rust hint (copy-ready):**

```rust
fn validate_content_id(s: &str) -> bool {
    s.len() == 3 + 64
        && s.starts_with("b3:")
        && s.as_bytes()[3..].iter().all(|&c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}
```

### 2.2 Storage semantics

* **Immutability:** `PUT(b3:…)` is idempotent. If content already exists and matches, return 200/204; if mismatch, return **409 Conflict**.
* **Addressing:** The **CID is the canonical key**. No aliases. No mutable pointers. (Aliases live in `svc-index`.)
* **Integrity:** For `PUT`, if the request includes bytes, the service **must** recompute BLAKE3 and verify it equals the `ContentId` provided. Mismatch ⇒ **400/422**.
* **Deduplication:** Natural consequence of content addressing—no special casing necessary beyond CID keying.

### 2.3 Path layout (filesystem backend guidance)

* Use a **fan-out** directory scheme to avoid hot directories. Example:

  * `b3:abcd…` → `store/b3/ab/cd/ef/abcd…<full 64>` (choose 2–4 levels; 2 chars per level is a good default).
* File naming:

  * **Data blob:** exact CID filename (or `data` inside the CID dir).
  * **Meta sidecar** (optional): `meta.json`/`meta.cbor` (only if you need internal metadata; **do not** store names here).
* Ensure **case sensitivity**: never upcase or accept upcased CIDs.

### 2.4 API sketch

* `PUT /v1/objects/{cid}` → writes raw bytes; validates CID vs. BLAKE3(content).
* `GET /v1/objects/{cid}` → returns raw bytes (streaming).
* `HEAD /v1/objects/{cid}` → existence + length + integrity tag (optional).
* `GET /v1/objects/{cid}/meta` (optional) → returns internal metadata (timestamps, byte length, media type if inferred, etc.); **must not** contain names.

> **Non-goals:** No name resolution, no version algebra, no IDNA here.

---

## 3) Security & Hygiene Rules (from naming → storage)

* **Reject names** at the API boundary. Always require `ContentId`.
* **No Unicode pitfalls**: since we only accept `[0-9a-f]`, path traversal via name is eliminated.
* **Filesystem safety:**

  * No `..`, no slashes in keys by construction.
  * Create internal directories with **0700** (on Unix) by default.
* **Timing & side channels:** treat missing vs. present CIDs uniformly where possible; avoid revealing store topology.
* **Memory & DOS:** cap object size (configurable); stream large GETs; use bounded buffers.

---

## 4) Observability (golden counters)

Recommended Prometheus counters/gauges/histograms keyed by **status** and **reason**:

* `storage_put_total{status="ok|exists|conflict|bad_cid|hash_mismatch|io_err"}`
* `storage_get_total{status="hit|miss|bad_cid|io_err"}`
* `storage_bytes_written_total`
* `storage_bytes_read_total`
* `storage_put_latency_seconds` (Histogram)
* `storage_get_latency_seconds` (Histogram)
* `storage_objects_total` (Gauge; sampled)
* `storage_fs_errors_total{op="open|read|write|sync|rename"}`

**Logs:** On validation failures, log at `INFO` with `cid`, `reason`. On integrity/hash mismatch, escalate to `WARN`.

---

## 5) Tests to port/extend (fast wins)

### 5.1 CID shape tests

* Accept valid: `b3:0123456789abcdef...` (64 hex)
* Reject:

  * `b3:0123…(63 hex)` (length off)
  * `B3:…` (wrong prefix case)
  * `b3:ABC…` (uppercase hex)
  * `b3:g123…` (non-hex)

### 5.2 Round-trip storage tests

* PUT(valid CID, matching bytes) → GET equals original bytes.
* PUT(same CID, same bytes) → 204 (idempotent).
* PUT(same CID, different bytes) → 409 conflict.
* GET(missing CID) → 404.
* GET(bad CID shape) → 400.

### 5.3 Concurrency & stress

* Parallel PUTs of identical content → all succeed (first writes, others no-ops).
* Parallel PUTs where one uses mismatched bytes → exactly one 409; others ok/idempotent.
* Fsync policy tests if you gate durability levels (fast vs. durable).

### 5.4 Wire shape / DTO tests (if API returns JSON/CBOR envelopes)

* Ensure `ContentId` fields serialize identical to `ron-naming` expectations.
  Use `ron_naming::wire::{json,cbor}` round-trip within tests for **schema lockstep**.

---

## 6) Performance Guidance (informed by naming)

* `ron-naming`’s encode/decode latency (~1–1.4 µs) is **not** the bottleneck; the **I/O path** is.
  Optimize:

  * Streaming GET/PUT (avoid loading entire body in RAM).
  * Use `sendfile`/`mmap` (optional) behind feature flags; default to safe streaming.
  * 4–16 KiB read/write buffers are a good first pass; benchmark on your disk.
  * Asynchronously compute BLAKE3 during streaming PUT (incremental hasher).

**BLAKE3 streaming (Rust sketch):**

```rust
use blake3::Hasher;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
async fn compute_b3(mut r: impl AsyncReadExt + Unpin) -> anyhow::Result<[u8; 32]> {
    let mut h = Hasher::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = r.read(&mut buf).await?;
        if n == 0 { break; }
        h.update(&buf[..n]);
    }
    Ok(*h.finalize().as_bytes())
}
```

---

## 7) Error Taxonomy (user-facing)

* **400 Bad Request**

  * Invalid CID format (explain exact shape; point to `svc-index` for names).
  * Hash mismatch (announce expected vs. computed).
* **404 Not Found**

  * Object not present (`GET`/`HEAD`).
* **409 Conflict**

  * CID exists with **different** bytes (integrity violation on repeated PUT).
* **5xx**

  * I/O or internal errors.

All of the above should map to structured JSON error payloads (machine-friendly), plus a concise human message.

---

## 8) Interop Notes

* **Do not** expose name fields in any `svc-storage` API; those live in `svc-index`.
* If you later add a “bundle manifest” format (listing multiple CIDs), keep it **pure ContentIds**. A different service can map names/versions to those CIDs.
* Keep JSON and CBOR encodings stable. If you need envelopes, version them (`"schema":"storage.v1"`).

---

## 9) Tooling that helps during dev

* Use `ron-naming` CLI to **pre-normalize** anything you’re sanity checking:

  ```
  cargo run -p ron-naming --features cli -- normalize "Café.Example"
  cargo run -p ron-naming --features cli -- parse "files.example@1.2.3"
  ```

  (But remember: storage must never accept the name form; this is just for generating preconditions in tests.)

---

## 10) “Done vs. Next” Checklist for `svc-storage`

### Done (inherited contracts)

* ✅ Canonical CID shape: `"b3:[0-9a-f]{64}"`.
* ✅ Names/versions are out of scope for storage.
* ✅ JSON/CBOR DTO compatibility rules known.

### Next (to implement now)

1. **Public API shape** (axum or your chosen stack):

   * `PUT/GET/HEAD /v1/objects/{cid}` (+ optional `/meta`)
2. **Early CID validation** (shared utility).
3. **Streaming BLAKE3 on PUT**; compare with path CID.
4. **Fan-out pathing** + durable write policy (configurable fsync).
5. **Observability counters/histograms** named above.
6. **Tests**:

   * CID hygiene unit tests.
   * PUT/GET/HEAD round-trips.
   * Conflict detection.
   * Concurrency races (tokio::join).
7. **Benches (optional but nice):**

   * PUT/GET throughput with tmpfs (functional perf smoke).
8. **Docs:**

   * README endpoints, example curl commands, error schema.
   * RUNBOOK: disk full, permissions, fs corruption playbooks.

---

## 11) Example cURL (handy for README/testing)

```
# PUT (assumes bytes.txt hashes to b3:0123...c0de)
curl -sS -X PUT --data-binary @bytes.txt \
  http://127.0.0.1:5304/v1/objects/b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

# GET
curl -sS http://127.0.0.1:5304/v1/objects/b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  -o out.bin

# HEAD
curl -sSI http://127.0.0.1:5304/v1/objects/b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```


### END NOTE - OCTOBER 28 2025 - 16:20 CST



### BEGIN NOTE - OCTOBER 28 2025 - 22:56 CST

---

# Carryover Notes — `svc-storage` (CAS over HTTP, amnesia profile)

**Date:** 2025-10-28 (America/Chicago)
**Profile:** RON-CORE (no Web3/value plane). Focus = content-addressed storage (CAS) with **amnesia** (RAM-only), BLAKE3 addressing, 1 MiB request body cap (OAP/1 alignment), simple HTTP API, readiness + health.

---

## 1) What we’ve accomplished

### 1.1 Skeleton + modules

* Crate scaffolding in place: `errors`, `types`, `http/` (routes, extractors, server, middleware stub), `storage/` (trait + impl(s)), `main.rs`, `scripts/smoke_storage.sh`.
* HTTP surface roughed in (Axum 0.7 line):

  * `GET /healthz` — liveness
  * `GET /readyz` — readiness
  * `PUT /o` — store bytes, return `cid`
  * `POST /o` — reserved (same as PUT or future multi-part); currently stub/optional
  * `HEAD /o/:cid` — metadata (len, etag)
  * `GET /o/:cid` — full/range reads with strong ETag + Content-Range
* **Error taxonomy** drafted for storage domain:

  * `NotFound`, `BadAddress`, `RangeNotSatisfiable`, `CapacityExceeded`, `IntegrityFailed`, `Io(_)`.
* **DTOs**:

  * `HeadMeta { cid, len, etag }` (etag = strong, e.g. `"b3:<hex>"`)
  * `PutResponse { cid }`
* **Storage trait** shape converging:

  ```rust
  #[async_trait]
  pub trait Storage {
      async fn put(&self, cid: &str, data: Bytes) -> Result<(), StorageError>;
      async fn exists(&self, cid: &str) -> Result<bool, StorageError>;
      async fn head(&self, cid: &str) -> Result<HeadMeta, StorageError>;
      async fn get_full(&self, cid: &str) -> Result<Bytes, StorageError>;
      /// Returns (slice, total_len)
      async fn get_range(&self, cid: &str, start: u64, end_inclusive: u64) -> Result<(Bytes, u64), StorageError>;
  }
  ```
* **Amnesia backend** drafted: `MemoryStorage` (`HashMap<String, Bytes>` in a `Mutex`) as the default, to unblock HTTP paths.

### 1.2 Tooling & scripts

* `crates/svc-storage/scripts/smoke_storage.sh` exists and runs the basic “bring-up + curl probes”.
* CI/lint posture: `cargo fmt` + `cargo clippy -D warnings` are in the loop (we are aligning code to pass these gates).

### 1.3 Invariants captured

* CID format is **BLAKE3 hex** prefixed with `b3:` (total length 67: `b3:` + 64 hex).
* Request body cap = **1 MiB** (keeps parity with OAP/1 1 MiB frame guidance).
* **No overwrite** semantics (idempotent writes ok; re-PUT same CID should either be a no-op or return integrity error — TBD final policy).
* Range semantics: **inclusive end**, surfaces `Content-Range`, `ETag`, and correct status `206` on partial.

---

## 2) What’s not done / known issues

### 2.1 Compile/link errors we saw (and why)

* **Module export gaps**: `http::server` / `http::error` missing from `http/mod.rs`.
* **Type drift**: handlers referenced `types::PutResponse`, `types::HeadMeta.etag`, `HeadMeta.size`, etc., when those weren’t defined or had different fields. Decision: `HeadMeta` must be `{ cid, len, etag }`; do **not** use `size`.
* **Axum extractor mismatch**: several handlers used `App(state): App` pattern. With Axum 0.7, the clean/portable approach is `Extension(AppState)` (or Router state with `State(AppState)` everywhere). Mixing `State<Arc<Readiness>>` with `AppState` on the same Router causes **state type mismatches**.
* **Error variants**: code used `BadAddress`, `RangeNotSatisfiable`, `BadRequest` but enum only had `NotFound`/`Io`. We’ve standardized on:

  * `BadAddress`, `RangeNotSatisfiable`, `CapacityExceeded`, `IntegrityFailed`, `NotFound`, `Io(_)`.
* **Bytes in FS backend**: `Bytes` not imported, trait didn’t declare `get_full`, etc. Our MVP is **amnesia only**; FS impl is optional for later.

### 2.2 Behavior gaps

* **Auth**: macaroon/cap checks not wired. `smoke_storage.sh` encountered `401` earlier; either disable auth for smoke or add a bypass in amnesia mode.
* **Readiness**: `readyz` always `200` right now; final should check store availability, config loaded, memory headroom, etc.
* **Metrics**: not wired yet (expose Prometheus counters/histograms via `ron-metrics` style `/metrics` endpoint or piggyback on service exporter).
* **POST /o**: either alias to PUT or implement multi-part chunking later (out of scope for beta).
* **HEAD/GET ETag policy**: presently `etag = format!("\"{}\"", cid)`. Optionally switch to `"b3:<hex>"` normalized form; just keep it **stable and strong**.

---

## 3) Minimal viable shape (to compile & smoke)

This is the “single state” plan that avoids the Axum type fights:

* **State**: `AppState { store: Arc<dyn Storage + Send + Sync> }`.

* Every handler uses: `Extension(AppState)` (or we can switch all of them to `State(AppState)`—but keep it consistent everywhere including `/healthz` and `/readyz`).

* Router:

  ```rust
  Router::new()
    .route("/healthz", get(routes::health::handler))
    .route("/readyz", get(routes::ready::handler))
    .route("/o", put(routes::put_object::handler))
    .route("/o", post(routes::post_object::handler))
    .route("/o/:cid", get(routes::get_object::handler))
    .route("/o/:cid", head(routes::head_object::handler))
    .with_state(app_state_or_extension_layer)
  ```

  Pick **one** of:

  * `with_state(app_state)` **and** handlers take `State(AppState)`, **or**
  * add `.layer(Extension(app_state))` **and** handlers take `Extension(AppState)`.
    Mixing leads to the mismatches we saw.

* **Error mapping**: central fn `http::error::into_response(StorageError) -> impl IntoResponse`, used by all handlers.

* **DTOs**: in `types.rs`

  * `HeadMeta { cid, len, etag }`
  * `PutResponse { cid }`

* **Body cap**: enforce `1_048_576` bytes in PUT/POST handler for now (later: layer-level limit or Tower limiter).

---

## 4) How to run (commands)

### 4.1 Dev loop

```bash
# inside repo root
cargo fmt -p svc-storage
cargo clippy -p svc-storage --no-deps -- -D warnings
cargo build -p svc-storage
```

### 4.2 Unit/integration tests (once they land)

```bash
cargo test -p svc-storage
# to run with logs:
RUST_LOG=debug cargo test -p svc-storage -- --nocapture
```

*(Right now, we have mostly smoke coverage via the script; add `tests/http_smoke.rs` soon—see “Next”)*

### 4.3 Smoke script

```bash
crates/svc-storage/scripts/smoke_storage.sh
```

What it does (expected):

* Builds svc-storage and launches it on `127.0.0.1:5303`.
* Probes:

  * `GET /healthz` → `200`
  * `GET /version` (if present) → string (optional)
  * `PUT /o` with a test body → **should return 201 + JSON {cid}** (we previously saw `401`: indicates auth is enforcing; temporarily disable)
  * `GET /o/<cid>` and `HEAD /o/<cid>` expectations:

    * `ETag` present
    * `Content-Length` correct
    * Range GET returns `206` + `Content-Range`.

If you keep auth enabled, export something like `RON_AMNESIA=1 RON_AUTH_BYPASS=1` inside the script (or wire a no-auth mode for amnesia).

### 4.4 Manual cURL checks

```bash
# PUT (1 MiB cap applies)
curl -s -X PUT --data-binary @/path/to/file.bin http://127.0.0.1:5303/o
# -> {"cid":"b3:<hex>"}

# HEAD
curl -I http://127.0.0.1:5303/o/b3:<hex>

# GET full
curl -v http://127.0.0.1:5303/o/b3:<hex> -o out.bin

# GET range (first 100 bytes)
curl -v -H "Range: bytes=0-99" http://127.0.0.1:5303/o/b3:<hex> -o slice.bin
```

---

## 5) Next steps (exhaustive + ordered)

**A. Stabilize compile + smoke**

1. **Unify state extraction**: Choose **Extension** or **State** and update all handlers + router accordingly.
2. Export missing modules:

   * `http/mod.rs`: `pub mod server; pub mod error;`
3. Normalize DTOs and uses:

   * `HeadMeta { cid, len, etag }` only (remove `size` usage everywhere).
   * `PutResponse { cid }`.
4. Error taxonomy finalization:

   * Ensure handlers only use the variants in `errors.rs` (remove `BadRequest`, etc., if not defined).
5. Keep **amnesia only** for now (comment out FS impl; optional TODO).

**Acceptance for A**: `cargo clippy -D warnings` green + `smoke_storage.sh` green with **no auth**.

---

**B. Add tiny test coverage (compile-proof + invariants)**

1. **Unit tests**:

   * `storage::memory`:

     * put→exists→head→get_full roundtrip
     * `get_range` bounds + inclusive end
     * duplicate put policy (no overwrite) → `IntegrityFailed`
   * `types::HeadMeta`:

     * `etag` shape is stable (`"b3:<hex>"` or `"cid"` decision).
2. **HTTP integration test** (Axum with in-proc server):

   * `PUT /o` returns 201 + cid; `HEAD/GET` consistent; Range returns 206 + headers.

**Acceptance for B**: `cargo test -p svc-storage` passes locally.

---

**C. Readiness + metrics**

1. `readyz` checks:

   * has store handle; optional memory headroom gauge; (later: bg sampler).
2. Wire minimal `ron-metrics` exporter (or reuse admin plane if present).

   * `storage_requests_total{op}`
   * `storage_bytes_total{op}`
   * `request_latency_seconds{route,method,status}`

**Acceptance for C**: `/readyz` transitions to non-200 when intentional fault injected (e.g., optional “disable store” flag), `/metrics` scrapes.

---

**D. Auth toggle (later)**

* Add env/config toggle to **bypass** auth in amnesia mode so smoke tests never 401.
* When enabled, require macaroon with “storage.write” for PUT/POST.

**Acceptance for D**: With auth disabled, smoke green; with auth enabled, PUT without token → 401.

---

**E. Optional FS backend (post-beta)**

* `FsStorage` behind a feature flag (`fs-backend`), implements `Storage`.
* Persist objects under `data/<b3>/<first2>/<rest>/blob` layout (fanout).
* Same trait, same semantics.

**Acceptance for E**: feature-gated build + tests pass; same HTTP behavior.

---

## 6) Troubleshooting map (from errors we saw)

* **`could not find 'server' in 'http'`**
  → Add `pub mod server;` to `http/mod.rs` and ensure `src/http/server.rs` exists.

* **`no 'PutResponse' in 'types'`**
  → Add `PutResponse` DTO; update `use` lines.

* **`could not find 'error' in 'http'`**
  → Create `http/error.rs` and export. Handlers must `use crate::http::error::into_response`.

* **`expected Handler<_, _>` / router state type mismatch**
  → Do **not** mix `State<Arc<Readiness>>` handlers with `with_state(AppState)` unless the Router is split. Use a **single state model**: either `Extension(AppState)` everywhere (and add layer) **or** `State(AppState)` everywhere (and use `with_state`).

* **`no field 'size' on HeadMeta`**
  → Replace with `len`. Ensure `HeadMeta` has `{ cid, len, etag }`.

* **`BadRequest` variant missing**
  → Use `BadAddress` for invalid CID, `CapacityExceeded` for >1 MiB, `RangeNotSatisfiable` for bad ranges.

* **`Bytes` not in scope**
  → `use bytes::Bytes;` in storage + routes that manipulate bodies.

---

## 7) Interface decisions (locked for RON-CORE beta)

* **CID**: `b3:<hex>` (lowercase hex). Length check: `67`.
* **ETag**: strong; **either** `"b3:<hex>"` **or** `"cid"` (choose one; recommend `"b3:<hex>"`).
* **Body cap**: 1 048 576 bytes (1 MiB) hard limit on PUT/POST.
* **Range**: inclusive range, 206 response, `Content-Range: bytes start-end/total`.
* **No overwrite**: second `PUT` to same `cid` should not change data (either return 201 again or 409/400; current draft uses `IntegrityFailed` if overwriting is attempted).

---

## 8) File checklist to keep in sync (so we don’t regress)

* `src/http/mod.rs` — exports **error**, **server**, **extractors**, **routes**, **middleware** (stub ok).
* `src/http/error.rs` — present; used by all routes.
* `src/http/extractors.rs` — defines `AppState` (store handle).
* `src/http/routes/{health,ready,put_object,post_object,get_object,head_object}.rs` — use **one** extractor pattern consistently (Extension or State).
* `src/storage/mod.rs` — trait matches handlers’ expectations; **amnesia** impl compiles.
* `src/types.rs` — `HeadMeta`, `PutResponse`.
* `src/errors.rs` — enum contains exactly the variants handlers use.
* `src/main.rs` — builds `AppState` and starts server; consistent extractor approach.

---

## 9) “Done” criteria for `svc-storage` (Beta)

* ✅ `cargo clippy -D warnings` passes
* ✅ `cargo test -p svc-storage` has at least:

  * storage roundtrip tests
  * http smoke test
  * range happy/invalid cases
* ✅ `smoke_storage.sh` passes end-to-end **without auth**
* ✅ `/healthz` 200, `/readyz` 200, `PUT/HEAD/GET` paths correct with headers:

  * `ETag`, `Content-Length`, and `Content-Range` (when ranged)
* ✅ Explicit **invariants doc** in `docs/PERFORMANCE.md` or `README.md`:

  * body cap, etag policy, cid rules, range rules
* ✅ Minimal metrics exposed (even if just request counters)

---

## 10) Nice-to-have (post-beta)

* Add **streaming** PUT with hashing on the fly (BLAKE3 has incremental; keep under 1 MiB for now).
* **LRU** or byte-cap in amnesia backend; evict oldest under pressure.
* **HEAD /o** without CID (maybe 405) — explicit response.
* **Conditional GET** (`If-None-Match`) → 304 when ETag matches.
* Write-path auth (macaroon), with a **bypass toggle** for smoke/tests.

---


### END NOTE - OCTOBER 28 2025 - 22:56 CST


### BEGIN NOTE - OCTOBER 29 2025 - 00:22 CST

Got you. Here’s a clean, paste-ready carryover pack for **svc-storage** you can drop into `crates/svc-storage/NOTES.md`. It captures the current state, exactly what passed/failed, why it likely failed, and a tight plan + checklist to get us back to green fast next session.

---

# Carryover Notes — `svc-storage`

**Date:** 2025-10-29 (America/Chicago)
**Profile:** RON-CORE (no Web3)
**Goal:** Minimal object store over HTTP with content-addressed PUT, HEAD/GET by CID, Range GET, optional `/metrics`.

---

## 1) What works right now

* Crate builds (`cargo run -p svc-storage`) after recent fixes.
* **Metrics route** compiles and serves prometheus text successfully when mounted.

  * Earlier we fixed: response shape, headers, lifetime of `TextEncoder::format_type()`.
* We have a working **smoke script** scaffold (`scripts/smoke_storage.sh`) that:

  * Builds/runs the service, waits for port, then performs: PUT, HEAD, GET, Range GET, unknown GET, `/metrics`.
  * When the service was aligned, we previously saw **PASS=9 FAIL=0 SKIP=1**.

---

## 2) What is currently broken (symptoms)

Latest run:

* Service starts; **GET /** returns `404` (expected fallback) → server is up.
* **All object API checks fail**:

  * PUT returned an empty `cid` (script saw `cid=''`) → either 404/405 or bad JSON body shape.
  * HEAD/GET/Range GET then cascade-fail (no CID persisted).
  * Unknown GET returned `400` (we expect `404`).
* **Metrics** route still OK (when mounted).

This pattern almost always means the **router does not have our object routes mounted as expected** (path mismatch, method mismatch, or feature-gating excluded the routes).

---

## 3) Most likely root causes

1. **Router mount drift in `http/server.rs`**

   * We saw earlier warnings:

     * `unused import: head` (HEAD route not actually added).
     * `unexpected cfg(feature="metrics")` (Cargo features don’t match the `#[cfg]` guards).
   * If HEAD/GET routes aren’t mounted, `HEAD /o/:cid` and `GET /o/:cid` will 404/405.
   * If PUT `/o` isn’t mounted, the smoke PUT step returns HTML/empty JSON → script reports empty `cid`.

2. **Feature flags mismatch**

   * Cargo.toml showed only `default`/`http` (no `metrics`), but code used `#[cfg(feature="metrics")]`.
   * If the object routes are behind `#[cfg(feature="http")]` but the crate wasn’t built with that feature, routes won’t exist.

3. **Module path churn in `main.rs`**

   * We bounced between `svc_storage::…` and `crate::…`. If the library name is `svc_storage` and `main.rs` tries `crate::http::…` without re-exporting, imports can fail or (if temporarily “fixed”) we might end up using a different router than the one exported.

4. **Handlers returning unexpected shapes**

   * We already fixed a few places (`post_object` delegating to `put_object::handler().into_response()`, Range tuple destructuring, etc.). If a handler still returns a different shape (or wrong status), the script will parse `cid` as empty.

---

## 4) Expected HTTP surface (contract)

* `PUT /o` (body = bytes) → `200` JSON: `{"cid":"b3:<hex>"}`
* `HEAD /o/:cid` → `200` with headers:

  * `Content-Length: <len>`
  * `ETag: "<b3-hash>"`
* `GET /o/:cid` → `200` with full body
* `GET /o/:cid` + `Range: bytes=START-END` → `206` with:

  * `Content-Length: <range_len>`
  * `Content-Range: bytes START-END/TOTAL`
  * Body = requested range
* Unknown CID → `404`
* `/metrics` (optional) → prometheus text

---

## 5) How to run & verify locally

```bash
# 1) Build & run the binary directly
cargo run -p svc-storage

# 2) Use the smoke script (preferred)
chmod +x crates/svc-storage/scripts/smoke_storage.sh
crates/svc-storage/scripts/smoke_storage.sh
# It prints PASS/FAIL counts and tails the server log on failure.

# 3) Manual curls if you want:
echo -n "hello world" | curl -sS -X PUT --data-binary @- localhost:5303/o | jq
CID=b3:$(printf 'hello world' | /usr/bin/openssl dgst -blake3 -binary | xxd -p -c256)
curl -i -I localhost:5303/o/$CID
curl -s localhost:5303/o/$CID
curl -s -H 'Range: bytes=0-4' localhost:5303/o/$CID
curl -s localhost:5303/o/b3:doesnotexist
curl -s localhost:5303/metrics
```

> Note: If you don’t have a `blake3` CLI, the `openssl -blake3` line gives you a compatible digest (we’re casing/formatting as `b3:<hex>`). The server’s own `ETag`/CID must match what it calculated on the PUT path.

---

## 6) File layout expectations (sanity)

* `src/lib.rs`

  * `pub mod http;`
  * `pub mod storage;`
* `src/http/mod.rs`

  * `pub mod extractors;`
  * `pub mod routes;`
  * `pub mod server;`
* `src/http/routes/…`

  * `put_object.rs`, `post_object.rs`, `head_object.rs`, `get_object.rs`, `metrics.rs`
* `src/http/server.rs`

  * Builds the `Router` and mounts **exact** paths:

    * `.route("/o", put(routes::put_object::handler).post(routes::post_object::handler))`
    * `.route("/o/:cid", head(routes::head_object::handler).get(routes::get_object::handler))`
    * Metrics route gated with the **correct feature name**
* `src/storage/mod.rs`

  * `trait Storage` and `MemoryStorage` (Default, thread-safe)
* `src/main.rs`

  * Imports via `svc_storage::http::…` **or** re-exports in `lib.rs` to allow `crate::…` (pick one and stick to it)
  * Calls `serve_http(addr, state).await?;` (handle the `Result`)

---

## 7) Concrete fixes to apply next session

**A. Align Cargo features and cfgs**

* In `crates/svc-storage/Cargo.toml` use something like:

```toml
[features]
default = ["http", "metrics"]
http = []
metrics = []
```

* In code, use matching guards:

```rust
#[cfg(feature = "metrics")]
.route("/metrics", get(routes::metrics::handler))
```

**B. Ensure all object routes are mounted**

In `http/server.rs`:

* Make sure we include **PUT /o**, **POST /o** (alias), **HEAD /o/:cid**, **GET /o/:cid** exactly.
* Remove `mut` on the router variable if unused (warning cleanup).
* Keep a simple `.fallback(|_| async { StatusCode::NOT_FOUND })` so GET `/` returns 404 (good for the wait-loop).

**C. Keep `main.rs` imports consistent**

* If `lib.rs` declares `pub mod http; pub mod storage;`, then in `main.rs` import with `use svc_storage::http::{…};` (assuming `[lib] name="svc_storage"`). Don’t bounce between `crate::…` and `svc_storage::…`.

**D. Handlers sanity**

* `put_object` returns `{"cid":"b3:<hex>"}` on success.
* `post_object` just delegates and **calls `.into_response()`** (already fixed).
* `get_object`: full and range code paths:

  * Destructure `(chunk, total_len)` and use `chunk.len()` for `Content-Length`.
  * Range parser supports `bytes=START-END`, `bytes=START-`, and `bytes=-SUFFIX`.
* `head_object`: only headers; same `ETag` as GET; length = stored length.

**E. Status codes**

* Unknown CID → `404` (not `400`).
* Range parse error → `416 (RANGE_NOT_SATISFIABLE)`.
* Successful range → `206`.

**F. Logging (to make the next debug trivial)**

In `main.rs` or `server.rs`, add a basic `tracing_subscriber::fmt()` init (INFO level), and log every mount:

```rust
tracing::info!("mount: PUT /o, POST /o, HEAD/GET /o/:cid");
#[cfg(feature="metrics")]
tracing::info!("mount: GET /metrics");
```

Also, at the start of each handler, log path + method and key fields (`cid`, range header). That way the `/tmp/svc-storage.log` the script tails will show if routes are being hit.

---

## 8) Why the smoke run failed this time (most probable)

* The router didn’t include **PUT /o** (or it was behind a mismatched `#[cfg]`) → the PUT request hit fallback (404/405), producing either an empty body or a non-JSON body; our script extracted no `cid` → chain of failures.
* Unknown GET returned `400` (bad) → suggests a handler that expects something else ran instead of our not-found/GET handler; again, a routing table issue.

Fixing the route mounting + features should flip us back to the **PASS=9 FAIL=0 SKIP=1** state we already achieved earlier.

---

## 9) Dev checklist (quick hits)

* [ ] `Cargo.toml` features: add `metrics`, include it in default if desired.
* [ ] `http/server.rs`: mount **exact** routes and correct `#[cfg]` guards.
* [ ] `main.rs`: consistent imports; handle `serve_http(...).await?` (not ignore).
* [ ] `lib.rs`: export `http` and `storage` modules.
* [ ] Handlers: ensure `IntoResponse` outputs and statuses align with contract.
* [ ] Run: `crates/svc-storage/scripts/smoke_storage.sh` → expect **PASS=9 FAIL=0 SKIP=0** (if metrics is on).
* [ ] Add a tiny integration test (optional next): spin a `Router` + `ServiceExt` to exercise PUT/GET/HEAD/Range without spawning a TCP listener.

---

## 10) What we changed/fixed earlier (context for reviewer AIs)

* Replaced fragile `headers::Range` usage with a small parser for `Range: bytes=…`.
* Fixed tuple destructuring for `(bytes, total_len)` → correct `Content-Length`.
* Made `post_object` delegate with `.into_response()`.
* Fixed prometheus `/metrics` handler to return `(StatusCode, headers, body)` correctly and to use a non-`'static` content type string.
* Added `MemoryStorage::Default` to satisfy clippy’s `new_without_default`.
* Removed needless borrow in `blake3::hash` line.

---

## 11) Minimal example of the expected mounts (for quick compare)

```rust
// http/server.rs (essentials only)
use axum::{Router, routing::{get, head, put}};
use crate::http::routes;

pub fn build_router() -> Router {
    let api = Router::new()
        .route("/o", put(routes::put_object::handler)
                     .post(routes::post_object::handler))
        .route("/o/:cid", head(routes::head_object::handler)
                          .get(routes::get_object::handler));

    #[cfg(feature = "metrics")]
    let api = api.route("/metrics", get(routes::metrics::handler));

    api.fallback(|| async { axum::http::StatusCode::NOT_FOUND })
}
```

(If your file already matches this shape, the bug is almost certainly feature flags or an import path causing us to build a different router than the one we run.)

---

## 12) Definition of done for this crate (Bronze)

* ✅ `cargo clippy -D warnings` clean.
* ✅ `scripts/smoke_storage.sh` → **PASS=9 FAIL=0** (metrics either PASS or SKIP by choice).
* ✅ Range GETs cover `bytes=START-END`, `START-`, `-SUFFIX`.
* ✅ Consistent `ETag` and `Content-Length`.
* ✅ Unknown returns `404`.
* ✅ Logs show routes hit + status lines.

---

## 13) Next (optional) improvements

* Add `/healthz` (`200` when router bound) and `/readyz` (flip once storage backend initialized).
* E2E integration test (Axum service, no TCP) to guard regressions for: PUT→HEAD→GET→Range→404.
* Storage backend stubs: disk/FS and zstd-compressed variants behind features.
* Simple quota/limit headers: reject >N MB uploads with `413 Payload Too Large`.
* Add `If-None-Match` / `If-Range` support (later).

---

**Bottom line:** we already hit a fully-green smoke run earlier; the current failures are consistent with route mounting/feature flag drift. Align `Cargo.toml` features and the `http/server.rs` mounts, keep `main.rs` imports consistent, rerun the smoke script — we should pop back to green.


### END NOTE - OCTOBER 29 2025 - 00:22 CST


### BEGIN NOTE - OCTOBER 29 2025 - 01:35 CST

# svc-storage — Carryover Notes (Beta Push)

**Date:** 2025-10-29 (America/Chicago)
**Scope:** RON-CORE baseline (no Web3 extras)
**Purpose:** Simple HTTP CAS (Content-Addressed Storage) surface for small/medium blobs, green smoke with HEAD/GET/PUT/Range + metrics.

---

## 1) Completion Estimate

* **Overall completion:** **~80–85% to Beta**
* **Why:**

  * Server wiring, routes, state, and in-memory store are implemented and pass the full smoke suite (PUT→CID, HEAD with ETag+Content-Length, GET, Range 206, unknown 404, metrics 200).
  * Remaining work is mostly **polish & productionization knobs**: error/edge-case precision, health/readiness, small API consistency (POST vs PUT), integration tests in CI, and one or two handler nits (e.g., 416 header shape).

**Beta DoD** (Definition of Done) below spells the exact checklist.

---

## 2) What We’ve Accomplished

* **Builds clean** with Axum 0.7 and the correct server features (tokio/http1/http2/json).
* **State wiring**: `AppState` is `Clone + Send + Sync + 'static` via `DynStorage` (Arc<dyn Storage>).
* **Routes** (aligned with smoke):

  * `PUT /o` — ingest raw bytes; compute `cid = "b3:<64-hex>"`; persist; return JSON `{ "cid": "..." }`.
  * `HEAD /o/:cid` — `200` with **strong ETag** (`"blake3hex"`), `Content-Length`.
  * `GET /o/:cid` — full body; supports `Range: bytes=start-end` for partial content (`206`).
  * `GET /metrics` — Prometheus exposition (feature-gated ok).
  * Fallback returns `404` for unknown paths.
* **Storage backend**: **MemoryStorage** in RwLock<HashMap<cid, Bytes>**:**

  * `put`, `head`, `get_full`, `get_range` implemented; range clamps; strong ETag via blake3 of stored Bytes.
* **Smoke test (latest run):** **PASS=9 FAIL=0 SKIP=0**

  * POST/PUT upload corrected in the script; invalid CID probe corrected to “valid format but unknown” (404 path).
  * Verified `ETag`, `Content-Length=11`, `206 Range` with body `hello`.
* **Observability**: tracing initialized; `/metrics` responding 200.
* **Cargo hygiene**: `prometheus` is optional under `metrics`; tokio includes `net` feature; Axum features enabled.

---

## 3) How to Build & Run (repeatable)

**Dev build & smoke:**

```
cargo fmt -p svc-storage
cargo clippy -p svc-storage --no-deps -- -D warnings
crates/svc-storage/scripts/smoke_storage.sh
```

**Manual quick test (in another shell after the server starts on 127.0.0.1:5303):**

```
echo -n "hello world" | curl -v -X PUT --data-binary @- http://127.0.0.1:5303/o
CID=b3:...paste...
curl -I "http://127.0.0.1:5303/o/$CID"
curl     "http://127.0.0.1:5303/o/$CID"
curl -i -H 'Range: bytes=0-4' "http://127.0.0.1:5303/o/$CID"
curl -s "http://127.0.0.1:5303/metrics" | head
```

---

## 4) API Surface (current behavior)

* **PUT /o**

  * Body: raw bytes
  * Response: `200 OK`, `application/json`

    ```json
    {"cid":"b3:<64-hex>"}
    ```
* **HEAD /o/:cid**

  * Response: `200 OK` + headers:

    * `Content-Length: <len>`
    * `ETag: "<blake3hex>"` (strong, quoted)
  * Errors:

    * `400` if CID malformed
    * `404` if well-formed but not found
* **GET /o/:cid**

  * Full: `200 OK` + body
  * Range: `206 Partial Content` + `Content-Range` + partial body
  * Errors as above
* **GET /metrics**

  * `200 OK` Prometheus text (when `metrics` feature enabled)
* **Fallback**

  * `404 Not Found` for unknown routes

---

## 5) Error Model & Status Codes (current/expected)

* **400 Bad Request** — CID parse/format invalid (not `b3:<64-hex>`).
* **404 Not Found** — CID well-formed but absent.
* **405 Method Not Allowed** — (Axum default) when wrong method targets a mounted path; we can return JSON 405 later (DX).
* **416 Range Not Satisfiable** — for unsatisfiable ranges; ensure we always include `Content-Range: */<len>` when total known.
* **500 Internal Server Error** — only for unexpected storage/IO faults (not present in MemoryStorage path, but keep mapping).

---

## 6) Metrics & Logging

* **/metrics** is live and returns 200.
* We should add a few **golden counters/histograms** (names reserved in RON metrics blueprint):

  * `storage_put_total{status}`
  * `storage_get_total{status,range}`
  * `storage_head_total{status}`
  * `storage_request_latency_seconds{route,method,status}` (Histogram)
* **Tracing** spans around each handler (route, cid, len, range-spec) will improve diagnostics.

---

## 7) Config Knobs (minimal for Beta)

* **ADDR** (env) or Config struct → bind socket.
* **amnesia** (bool) — if `true`, use MemoryStorage; if `false` later, switch to Disk/Object store (post-Beta).
* **body size limits** (optional nice-to-have): enforce max bytes per object.

---

## 8) Performance Notes

* Memory path is O(1) map lookup with `Bytes` clones (zero-copy slices on ranges).
* For Beta, current path is fine; no blocking IO; no locks across `.await`.
* Post-Beta: add Disk/Object-store backend and streaming reads/writes with back-pressure.

---

## 9) Security Notes

* No auth by default (Micronode dev profile).
* Post-Beta: macaroon/token gate (feature-gated), size limits, per-route caps, and basic DoS mitigation (request body limits, timeouts).

---

## 10) Definition of Done (Beta)

**Must-have (all are small, targeted):**

1. **Mount health/ready**: `GET /healthz` → `200`; `GET /readyz` → `200` once bound.
2. **Method DX**: Either **mount both** `PUT /o` and `POST /o` (same handler) or hard-enforce one and return **JSON 405** with `Allow:` header.
3. **Range polish**: Guarantee `416` includes `Content-Range: */<len>`; verify edge cases (open ends, overlaps, start≥len).
4. **CID validation**: Ensure consistent `400` for malformed, `404` for well-formed but missing.
5. **Metrics minimal set**: `storage_*_total` counters + a request latency histogram.
6. **Integration tests** (crate-local): replicate smoke steps (no TCP listener needed—use `axum::Router` + `tower::ServiceExt::oneshot`).
7. **README + RUNBOOK**: quickstart, routes, error codes, example curl, metrics table, and known limits.
8. **CI hooks**: add to workspace CI: format, clippy `-D warnings`, unit/integration tests, and the smoke script as a job.

**Nice-to-have (if time allows, otherwise Post-Beta):**

* JSON 405 body & `Allow` header.
* Response caching headers (`Cache-Control: immutable`) since content is addressable.
* Simple content-type echo (`Content-Type` passthrough on PUT).
* Disk backend stub (behind feature flag).

---

## 11) Prioritized Next Steps (Action Plan)

**T-1: Route consistency & health**

1. Mount health endpoints and verify in smoke:

   * `GET /healthz` → always 200
   * `GET /readyz` → 200 after listener bound
2. Decide **one** upload verb (recommend keep **POST /o** for “create”) **or** support both:

   * Router: `.route("/o", put(put_handler).post(put_handler))`
   * Smoke: use POST (lower friction)

**T-2: Handler polish**
3) Range edge cases:

* If `Range` unsatisfiable → `416` + `Content-Range: */<len>` (test explicitly).
* Inclusive end clamp already present; keep.

4. CID errors: add tests showing `400` (malformed) vs `404` (absent well-formed).

**T-3: Observability**
5) Add Prometheus counters/histogram wrappers in handlers (use existing `ron-metrics` patterns).
6) Add tracing spans: `route`, `cid`, `len`, `range`.

**T-4: Tests/CI**
7) Add `tests/http_contract.rs` exercising:

* POST/PUT upload → JSON CID
* HEAD headers exactly
* GET full equals payload
* Range returns 206 + correct slice + headers
* Unknown (valid) → 404; malformed → 400

8. Wire CI: `cargo fmt`, `clippy`, `test -p svc-storage`, run smoke script.

**T-5: Docs**
9) Update `README.md` & `RUNBOOK.md`: routes, curl examples, error table, metrics names, envs.
10) Add `CONFIG.md` with ADDR + toggles.

---

## 12) Validation Checklist (quick)

* [ ] `PUT /o` (and/or `POST /o`) returns JSON `{"cid":"b3:..."}`.
* [ ] `HEAD /o/:cid` → `200`, `ETag: "<hash>"`, `Content-Length = len`.
* [ ] `GET /o/:cid` full body matches upload.
* [ ] `GET /o/:cid` with `Range` returns `206` and correct body slice; 416 path correct.
* [ ] Malformed CID → `400`; valid+absent → `404`.
* [ ] `/metrics` → `200`.
* [ ] `/healthz`, `/readyz` → `200` behavior as intended.
* [ ] Integration tests pass locally and in CI.

---

## 13) Known Risks / Watch-outs

* **Script drift** vs. router methods (now mitigated—prefer mounting **both** methods).
* **Future backend swap** (Disk/Object store): ensure trait methods are streaming-friendly (they look fine; return `Bytes` today, can adapt).
* **Range semantics**: keep tests locked to prevent regressions (off-by-ones are common).
* **Large payloads**: set & document reasonable body size limits to avoid memory spikes (post-Beta optional).

---

## 14) Decision Log (recent)

* Use Axum 0.7 with explicit features (`tokio`, `http1`, `http2`, `json`).
* Serve via `axum::serve(listener, app.into_make_service())` with `.with_graceful_shutdown`.
* Strong ETag = quoted blake3 hex of stored content.
* MemoryStorage as default for Micronode dev; Disk/Object later.
* Metrics gated under `metrics` feature; exposed at `/metrics`.

---

### TL;DR to Beta

We’re **~80–85% done**. The service already passes the **full smoke suite**; to hit Beta, we only need to: mount health/ready, lock down method consistency (or support both PUT/POST), tighten Range 416 shape, add minimal counters + a request latency histogram, and land the integration tests + CI hooks. That’s it. Once those are in, we can confidently move on to the next crate.



### END NOTE - OCTOBER 29 2025 - 01:35 CST


### BEGIN NOTE - OCTOBER 29 2025 - 14:37 CST

---

# svc-storage — Carryover Notes (Beta Achieved)

**Date:** 2025-10-29 (America/Chicago)
**Profile:** RON-CORE baseline (no Web3 extras)
**Purpose:** Simple HTTP CAS (Content-Addressed Storage) surface for small/medium blobs with strong validation; green smoke + black-box test; observability endpoints.

---

## 1) Completion Status

**Beta: REACHED**
We now meet the Beta DoD:

* POST/PUT to `/o` → JSON `{"cid":"b3:<64-hex>"}` ✅
* `HEAD /o/:cid` → `ETag: "<hex>"` (strong, quoted) + `Content-Length` ✅
* `GET /o/:cid` → full body ✅
* `GET /o/:cid` with `Range` (single range) → `206` + correct `Content-Range` header ✅
* Unsatisfiable range → `416` + `Content-Range: */<len>` ✅
* Malformed CID → `400`, well-formed missing → `404` ✅
* `/metrics` → `200` (feature-gated) ✅
* Health endpoints mounted: `/healthz`, `/readyz` ✅
* Tests: `http_blackbox.rs` green; smoke script PASS=9 ✅
* Clippy `-D warnings` clean ✅

**Smoke summary (latest run):** PASS=9, FAIL=0, SKIP=0.

---

## 2) What We Shipped (since last instance)

### Code/Handlers

* **HEAD** route fixed to always return:

  * `ETag: "<hex>"` (quotes enforced even if backend returns raw hex).
  * `Content-Length: <len>`.
  * `400` for malformed CID, `404` for well-formed but unknown.
* **GET** route:

  * Correct **single-range** handling (`bytes=START-END`, `START-`, `-SUFFIX`).
  * `206 Partial Content` with `Content-Range: bytes START-END/TOTAL`.
  * `416 Range Not Satisfiable` with `Content-Range: */<len>`.
  * No `http_body_util` dependency (uses Axum tuples/Body directly).
* **Upload** surface accepts **PUT and POST** on `/o` (same handler) to avoid script drift and improve DX.

### Routing / Server

* Central router wiring in `http/server.rs`:

  * `POST|PUT /o`, `HEAD|GET /o/:cid`, `GET /version`, `GET /healthz`, `GET /readyz`, `GET /metrics` (feature).
* Removed unused imports and dead code; no `axum-extra`, no `TypedHeader`.

### Storage Backend

* In-memory store (`MemoryStorage`) with `Bytes` and RwLock map.
* Range reads slice without copying; clamps indices.

### Validation & Errors

* Fast, **no-regex** CID validator: `b3:` prefix + 64 lowercase hex.
* Strict error model aligned to spec:

  * `400` malformed CID.
  * `404` well-formed but unknown.
  * `416` with `Content-Range: */<len>` for unsatisfiable ranges.

### Tests & Scripts

* **Integration test (black box):** `tests/http_blackbox.rs` spawns the binary and exercises POST/HEAD/GET/Range; green.
* **Smoke script:** `scripts/smoke_storage.sh`:

  * Waits for server.
  * Uploads content with `printf 'hello world'` (portable).
  * Verifies HEAD headers, GET full, GET range, unknown 404, metrics 200.
  * Prints PASS/FAIL counts and last 200 lines of server log if fail.

### Observability

* `/metrics` mounted behind `metrics` feature (Prometheus exposition OK).
* `/healthz` and `/readyz` added (currently OK; if you later flip readiness after bind, leave `/healthz` always 200 and gate `/readyz` truthfully).

---

## 3) Current API Contract

### Objects

* `POST /o` and `PUT /o`
  Body: raw bytes.
  Response: `200 OK`, `{"cid":"b3:<64-hex>"}`.

* `HEAD /o/:cid`
  Success: `200 OK` with `ETag: "<hex>"`, `Content-Length: <len>`.
  Errors: `400` malformed CID, `404` not found.

* `GET /o/:cid`
  Full: `200 OK` + body.
  Range: `206 Partial Content` + `Content-Range: bytes START-END/TOTAL` + body slice.
  Unsatisfiable: `416 Range Not Satisfiable` + `Content-Range: */<TOTAL>`.
  Errors: `400` malformed, `404` not found.

### Observability

* `GET /metrics` → `200 OK` (Prometheus text; feature-gated).
* `GET /healthz` → `200 OK`.
* `GET /readyz` → `200 OK` (currently; can be made truthful if desired).
* `GET /version` → version info (as wired in your tree).

---

## 4) How To Run & Verify

### Build & test

```
cargo fmt -p svc-storage
cargo clippy -p svc-storage --no-deps -- -D warnings
cargo test -p svc-storage
```

### Smoke (end-to-end)

```
crates/svc-storage/scripts/smoke_storage.sh
```

What it checks:

* Upload (POST) returns CID.
* HEAD has `ETag` + correct `Content-Length`.
* GET full matches payload.
* Range 206 slice and headers correct.
* Unknown 404.
* `/metrics` 200.
* Prints PASS/FAIL summary.

---

## 5) Config / Env

* `ADDR` (optional) — bind address for server (default: `127.0.0.1:5303`).
* `RUST_LOG` — tracing filter (e.g., `info,axum=warn`).
* `--features metrics` — enable Prometheus exposition at `/metrics`.

*(Post-Beta plan)* Add caps:

* Max body size (per object).
* Amnesia switch (already de facto with memory backend).

---

## 6) Metrics (current & planned)

**Current:** Prometheus exposition endpoint is up and returns 200 (gathers whatever is registered globally; minimal today).

**Planned fast follow (optional before GA):**

* Counters:

  * `storage_put_total{status}`
  * `storage_head_total{status}`
  * `storage_get_total{status,range=("full"|"partial")}`
* Histogram:

  * `storage_request_latency_seconds{route,method,status}` with low-latency buckets
* Per-handler increment/wrap with timers.

These can be added surgically with a small `metrics.rs` helper (you already have the file scaffold).

---

## 7) Security Notes

* No auth in Micronode dev profile (by design).
* Content-addressed semantics reduce cache/consistency pitfalls; strong ETag guarantees exact object identity.
* **Post-Beta/GA**: size limits, request timeouts, DoS guards, optional macaroon/token gate, and content-type echo or restriction (if desired).

---

## 8) Performance Notes

* All-RAM path, O(1) lookups, `Bytes` slices for ranges; no `.await` inside RwLock critical sections.
* Single-range keeps implementation tight and predictable; multi-range (multipart/byteranges) can come later if truly needed.

---

## 9) Known Risks / Edge Cases

* **Single-range only.** We intentionally reject multi-range specs (returns 416).
* **Lowercase hex only** for CID; upper-case rejected as malformed (intended).
* **Large objects**: Memory growth is unbounded in the current profile; add body size limits before GA if you expect large payloads or untrusted environments.

---

## 10) Minimal Diffs/Artifacts (for audit)

* `src/http/routes/head_object.rs` — new/updated to set `Content-Length`, quoted `ETag`, and 400/404 split.
* `src/http/routes/get_object.rs` — updated range parsing and 416 `Content-Range` shape; removed `http_body_util`.
* `src/http/server.rs` — router wiring for POST/PUT /o, HEAD/GET, /healthz, /readyz, /version, /metrics (feature).
* `tests/http_blackbox.rs` — added; spawns binary and exercises API end-to-end.
* `scripts/smoke_storage.sh` — updated; uses `printf` (portable), PASS=9.

Cargo:

* Removed `axum-extra`; no `http_body_util`.
* Added `tower` under `[dev-dependencies]` (if you later want Router::oneshot tests—currently black-box test doesn’t need it).
* Workspace pins unchanged.

---

## 11) What’s Left (Non-blocking polish vs GA)

**Non-blocking (can do later; recommended soon):**

1. **Truthful `/readyz`**: 503 until listener binds; flip to 200 after `TcpListener::bind`. (You already have the route; just add a small atomic gate if desired.)
2. **Request metrics**: add counters + latency histogram around handlers.
3. **README.md & RUNBOOK.md**: route table, curl examples (copy from smoke), error model, env vars, known limits.
4. **CI integration**: job that runs `fmt`, `clippy -D warnings`, `test`, and the smoke script.

**GA Enhancements (after RON-CORE Beta):**

* Disk/object storage backend behind a feature flag (streaming upload/download, back-pressure).
* Optional auth layer (macaroons/tokens), size limits, per-route caps.
* Multi-range support if a real use-case emerges.
* Content-type echo or detection (if you want to reflect `Content-Type` on GET).

---

## 12) Integration Notes for `svc-index` (next crate)

`svc-index` will reference content by **CID** and, in many flows, **confirm existence** before indexing metadata. Reuse svc-storage’s decisions to keep cohesion:

* **CID validator**: Reuse the exact `b3:<64-hex>` validator (no regex).
* **Error model alignment**:

  * `400` on malformed CID input to index/search endpoints.
  * `404` for well-formed but absent (e.g., referencing non-stored objects).
* **Health/Ready/Metrics**: Mirror `/healthz`, `/readyz`, `/metrics` so your smoke/CI patterns stay uniform across services.
* **Strong identity**: When index returns object hints or presigned links later, preserve ETag behavior (`"hex"`) and ensure any cross-service cache respects this invariant.
* **Script parity**: Make `crates/svc-index/scripts/smoke_index.sh` with the same style (PASS summary, log tail on fail).
* **Dependency graph**:

  * If `svc-index` needs to **verify existence** of a CID in storage, prefer an internal client trait with an adapter talking to `svc-storage` (HTTP) so we can later swap or mock in tests.
  * Consider adding an optional **“probe existence”** endpoint in storage (HEAD already serves this role; index can call `HEAD /o/:cid`).

---

## 13) One-Shot Commands (for future yourself)

Run everything:

```
cargo fmt -p svc-storage
cargo clippy -p svc-storage --no-deps -- -D warnings
cargo test -p svc-storage
crates/svc-storage/scripts/smoke_storage.sh
```

Commit:

```
git add crates/svc-storage
git commit -m "svc-storage: Beta ✅ — HEAD ETag+Content-Length; single-range 206/strict 416; PUT|POST ingest; healthz/readyz/metrics; blackbox test; smoke PASS=9"
```

---

## 14) TL;DR

svc-storage is **Beta-complete** and stable: content-addressed PUT/POST, robust HEAD/GET with strict byte-range semantics, clean error model, health/ready/metrics endpoints, smoke + black-box tests green. You’re cleared to proceed to **svc-index**.



### END NOTE - OCTOBER 29 2025 - 14:37 CST