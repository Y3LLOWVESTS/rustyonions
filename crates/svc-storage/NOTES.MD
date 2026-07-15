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


### BEGIN NOTE - APRIL 27 2026 - 20:00 CST


# 5. NOTE — `svc-storage`

## Current Status

`svc-storage` is now both a working CAS service and the first storage-side WEB3 enforcement point.

Its roles are now:

```text
free/dev CAS object store
paid ROC-gated CAS write endpoint
accounting-compatible usage event emitter
paid-write metrics producer
future wallet receipt verifier host
```

The latest sweep proves `svc-storage` is clippy clean and test clean. It now has:

```text
http_blackbox.rs:          1 passed
paid_write_policy.rs:     5 passed
paid_write_verifier.rs:   11 passed
web3_paid_storage_loop.rs: 1 passed
bench targets compile
```

All passed. 

The original storage notes defined the crate as a content-addressed storage service with BLAKE3 `b3:<hex>` identifiers, free CAS HTTP routes, health/ready, metrics, range GET, and amnesia profile.  The docs also identify CAS invariants, byte-range correctness, `/metrics`, `/healthz`, `/readyz`, and capability-only writes as part of the intended storage contract. 

## What We Accomplished

### 1. Clean `svc-storage` Baseline Restored

We first removed unused direct path dependencies on `ron-kernel` and `ron-metrics` from `svc-storage` so clippy could reach the crate itself.

This was necessary because `svc-storage` currently uses its own local minimal Prometheus route and does not yet need kernel/metrics integration.

Result:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings ✅
cargo test -p svc-storage --all-targets ✅
```

### 2. Replaced Flaky TCP Spawn Test With In-Process Router Test

The old `http_blackbox.rs` spawned `cargo run -p svc-storage`, which caused:

```text
clippy::zombie_processes
flaky startup timing
port/process issues
```

We replaced it with an in-process Axum router test.

Now `http_blackbox.rs` proves:

```text
POST /o stores bytes and returns cid
HEAD /o/:cid returns metadata headers
GET /o/:cid returns original bytes
Range GET returns exact slice and Content-Range
unknown CID returns 404
bad CID path returns 404
/metrics is mounted when enabled
```

This gives a stable CAS baseline without spawning a child process.

### 3. Added Dedicated Paid Write Route

We added:

```text
POST /paid/o
PUT  /paid/o
```

The existing free/dev route remains unchanged:

```text
POST /o
PUT  /o
HEAD /o/:cid
GET  /o/:cid
```

This means:

```text
/o       = free/dev CAS path
/paid/o  = ROC-paid write path
```

This avoids breaking existing developer workflows while giving the value plane an enforcement seam.

### 4. Paid Write Proof Enforcement

`/paid/o` now requires wallet hold proof metadata.

It rejects before writing when:

```text
proof is missing
operation is not hold
asset is not roc
estimate is zero
estimate is non-integer
receipt hash is not b3:<64 lowercase hex>
payer is missing
escrow is missing
```

It also rejects malformed accounting context:

```text
tenant is zero
tenant is non-integer
pin_seconds is non-integer
```

The `paid_write_policy.rs` tests prove these failures do not store the object.

### 5. Paid Write Usage Events

Successful `/paid/o` responses now include accounting-compatible usage events:

```text
bytes_stored
request_ok
pin_seconds, when x-ron-pin-seconds > 0
```

The cross-crate paid storage loop parses these as `ron-accounting::UsageEvent` and ingests them through `record_usage_events`.

This proves:

```text
svc-storage paid write
→ usage_events
→ ron-accounting ingest
→ recorded accounting rows
```

### 6. Paid Write Metrics

We added local Prometheus metrics:

```text
storage_paid_write_total{status="accepted"}
storage_paid_write_total{status="payment_required"}
storage_paid_write_total{status="bad_accounting_context"}
storage_paid_write_total{status="storage_error"}
storage_paid_write_bytes_total
```

The paid write metrics test proves these series are exposed through `/metrics`.

### 7. Cross-Crate Paid Storage Loop

`web3_paid_storage_loop.rs` now proves the full dev loop:

```text
unfunded wallet hold rejects
unfunded storage write does not create object
funded payer receives ROC
wallet hold reserves ROC into escrow
/paid/o accepts hold proof and stores object
GET /o/:cid returns exact object bytes
storage emits accounting-compatible usage events
ron-accounting ingests those events
wallet capture pays svc_storage
wallet release refunds payer
final wallet value is conserved
```

This is the strongest proof so far that the value plane is not just crate-local.

### 8. Paid Write Verifier Seam

We refactored trusted proof parsing out of `paid_object.rs` into:

```text
policy::paid_write
```

New components:

```text
PaidWriteVerifier trait
DevHeaderVerifier
WalletReceiptVerifier
WalletReceipt DTO
PaidWriteProof
VerifiedPaidWrite
PaidWriteVerificationError
```

Current behavior:

```text
/paid/o → DevHeaderVerifier → CAS write
```

Future behavior:

```text
/paid/o → WalletReceiptVerifier → svc-wallet receipt lookup → CAS write
```

`WalletReceiptVerifier` currently fails closed until the real lookup adapter is wired.

### 9. Wallet Receipt Contract

`WalletReceipt` now validates the receipt shape storage will require from wallet.

It accepts only:

```text
op = hold
asset = roc
amount_minor = positive integer
from = payer present
to = escrow present
receipt_hash = b3:<64 lowercase hex>
```

Tests prove it rejects:

```text
wrong operation
wrong asset
zero amount
non-integer amount
missing payer
missing escrow
bad receipt hash
```

It also tolerates future wallet fields during JSON deserialization, which protects storage against additive wallet receipt changes.

## What Remains for `svc-storage`

### 1. Real Wallet Receipt Verification

Current `/paid/o` uses `DevHeaderVerifier`. This is correct for dev/test, but production must not trust headers blindly.

Remaining work:

```text
- add wallet receipt lookup client trait
- add mock client tests
- implement real GET /v1/tx/:txid lookup when ready
- verify returned receipt matches paid write proof
- ensure mismatched txid/receipt/account/amount rejects
```

Recommended progression:

```text
Step A: WalletReceiptLookup trait + MockWalletReceiptLookup
Step B: WalletReceiptVerifier validates looked-up receipt
Step C: HTTP client implementation
Step D: route config chooses DevHeaderVerifier or WalletReceiptVerifier
```

### 2. Paid Route Configuration

We need a mode switch:

```text
dev mode       /paid/o uses DevHeaderVerifier
production     /paid/o uses WalletReceiptVerifier
disabled       /paid/o maybe 404/403
```

Remaining work:

```text
- add config enum for paid_write_verifier_mode
- add tests for fail-closed production default if desired
- decide beta default
```

### 3. Stronger Proof Binding

Eventually, the paid proof should bind to more than “some wallet hold happened.”

Possible future binding fields:

```text
expected CID
payer
escrow
service = svc-storage
route = /paid/o
max bytes or price estimate
expiry timestamp
receipt hash
idempotency key
```

Remaining work:

```text
- define hold context/memo schema
- ensure wallet hold receipt includes context
- verify context in storage before write
```

### 4. Actual Capture/Release Automation

Current loop does:

```text
wallet hold
storage write
manual/test capture
manual/test release
```

Production may need storage or a billing coordinator to trigger:

```text
capture actual cost
release remainder
emit final settlement event
```

Remaining work:

```text
- decide whether svc-storage captures directly or emits event for coordinator
- add capture/release adapter if storage owns final settlement
- add failure handling for write succeeded but capture failed
```

### 5. Storage Usage Event Export

Currently usage events are returned in the `/paid/o` response and ingested in-process by tests.

Production needs a real export path:

```text
POST to ron-accounting
bus event
local WAL/export file
batch forwarder
```

Remaining work:

```text
- define event delivery mechanism
- add retry/backoff
- add idempotency key for usage events
- add dead-letter/quarantine behavior
```

### 6. Broader CAS Hardening

Current CAS baseline is good, but remaining storage hardening includes:

```text
PUT /o/:cid with hash verification, if desired
strict CID-only APIs
body cap enforcement tests
concurrency/race tests
range edge cases
conditional GET/If-None-Match
FS backend, if beta needs persistence
amnesia-vs-durable mode decision
```

The older notes emphasized BLAKE3 `b3:<hex>`, no names inside storage, and content-address-only semantics.  Those invariants should continue to guide all storage changes.

### 7. Metrics Expansion

Current paid metrics exist. Broader storage metrics remain:

```text
storage_put_total
storage_get_total
storage_bytes_written_total
storage_bytes_read_total
storage_put_latency_seconds
storage_get_latency_seconds
storage_objects_total
storage_fs_errors_total
```

These are part of the intended storage observability contract. 

## Recommended Next Steps for `svc-storage`

```text
1. Add WalletReceiptLookup trait and mock verifier tests.
2. Keep DevHeaderVerifier as dev/test mode.
3. Add real WalletReceiptVerifier only after wallet receipt lookup contract is locked.
4. Add config switch for paid verifier mode.
5. Add production accounting event exporter.
6. Add capture/release automation strategy.
7. Expand storage metrics to full golden set.
```

## Completion Estimate

```text
svc-storage CAS baseline: ~80–85%
svc-storage paid-write beta path: ~75–82%
svc-storage production paid storage: ~60–70%
```

The paid-storage path is now real and tested, but production verification, accounting export, and capture/release automation remain the major hardening steps.

---


### END NOTE - APRIL 27 2026 - 20:00 CST


### BEGIN NOTE - APRIL 28 2026 - 15:10 CST


# svc-storage NOTES.MD — WEB3 Paid Storage / Wallet Receipt Verification Plateau

Date: 2026-04-28  
Crate: `svc-storage`  
Workspace path: `crates/svc-storage`  
Project: RustyOnions / RON-CORE / WEB3 / ROC internal value plane  
Current status: **WEB3 paid-storage beta path is now strongly green**  
Completion estimate:  
- **CAS baseline:** ~88–92%  
- **Paid storage internal beta path:** ~93–96%  
- **Production-hardened paid storage:** ~82–88%  

---

# 0. Executive Summary

`svc-storage` has now become more than a normal content-addressed object store. It is currently the **first real paid-use enforcement point** for the RustyOnions internal ROC economy.

The crate now supports:

```text
free/dev CAS object writes
free/dev CAS object reads
HEAD metadata checks
range reads
BLAKE3 b3:<64 lowercase hex> addressing
paid ROC-gated object writes through /paid/o
paid-write proof validation
wallet hold receipt validation
configurable paid verifier mode
real HTTP wallet receipt lookup client
route-level wallet-receipt mode test
accounting-compatible usage event DTOs
paid-write Prometheus metrics
cross-crate paid storage loop with svc-wallet and ron-accounting
````

The latest verified sweep passed:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_wallet_mode
cargo test -p svc-storage --all-targets
```

Final observed status:

```text
clippy -D warnings: clean

config tests:                  3 passed
http_blackbox.rs:              1 passed
paid_write_http_clients.rs:    7 passed
paid_write_policy.rs:          5 passed
paid_write_verifier.rs:        19 passed
paid_write_wallet_mode.rs:     1 passed
web3_paid_storage_loop.rs:     1 passed
bench targets:                 compile as test targets
```

The latest `paid_write_wallet_mode` test proves the route-level production-shaped flow:

```text
/paid/o
→ RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
→ mock svc-wallet GET /v1/tx/{txid}
→ matching wallet hold receipt accepted
→ paid object stored

mismatched wallet receipt
→ rejected
→ object not stored
```

The terminal output confirms `paid_write_wallet_mode` passed, the HTTP wallet client tests passed 7/7, the verifier contract tests passed 19/19, paid-write policy passed 5/5, and the WEB3 paid storage loop still passed. 

This is a major upgrade from the older state where `/paid/o` only trusted dev headers and `WalletReceiptVerifier` failed closed. Earlier carryover notes listed the target progression as `WalletReceiptLookup trait`, mock tests, real `GET /v1/tx/:txid`, and configurable verifier mode; those items are now substantially implemented and tested. 

---

# 1. Strategic Role of `svc-storage` in WEB3 / ROC

`svc-storage` is the content-addressed storage service and the first concrete paid in-network use case.

Its role is:

```text
store immutable content-addressed bytes
serve objects by BLAKE3 ContentId
enforce paid storage writes
emit storage usage signals
prove pay-for-storage/pinning as the first ROC use case
```

Its role is **not**:

```text
not the ledger
not the wallet
not the accounting truth engine
not the reward calculator
not the name resolver
not the passport authority
not the external-chain bridge
```

Correct boundaries remain:

```text
svc-storage:
  CAS bytes
  paid-write admission
  usage event emission
  paid storage metrics

svc-wallet:
  hold/capture/release
  balance truth boundary
  nonce/idempotency
  wallet receipts

ron-ledger:
  append-only economic truth
  replay
  conservation
  balances

ron-accounting:
  transient usage counters
  storage usage events
  reward snapshots

svc-rewarder:
  deterministic reward calculation
  settlement/issue intents

svc-index:
  names/manifests/owner lookups
  not raw byte storage

ron-naming:
  ContentId/address schema hygiene
```

Older storage notes correctly emphasize that `svc-storage` should only store and serve content by canonical BLAKE3 `ContentId`, while names and semantic resolution stay upstream in `svc-index` / `ron-naming`. 

---

# 2. Core Storage Invariants

The storage crate must continue to preserve these invariants:

```text
BLAKE3 is the content-addressing algorithm.
Canonical CID form is b3:<64 lowercase hex>.
CID is the canonical storage key.
Storage does not resolve human names.
Storage does not perform IDNA/UTS-46/name normalization.
Names, manifests, crab links, site routes, and mutable pointers belong upstream.
Objects are addressed by content identity, not by mutable aliases.
Paid writes must fail closed.
Unpaid or malformed paid writes must not store bytes.
Wallet receipt verification must reject mismatches.
```

The earlier naming-to-storage notes define the canonical ContentId form as `b3:<64 lowercase hex>` and state that names are strictly upstream, with `svc-storage` only accepting content IDs. 

---

# 3. Current Public HTTP Surface

Current route surface:

```text
POST /o
PUT  /o
HEAD /o/:cid
GET  /o/:cid

POST /paid/o
PUT  /paid/o

GET  /healthz
GET  /readyz
GET  /metrics
GET  /version
```

## 3.1 Free / Dev CAS Route Behavior

The free/dev CAS route is still intact.

Behavior:

```text
POST /o stores bytes and returns computed cid.
PUT /o stores bytes and returns computed cid.
HEAD /o/:cid returns existence and metadata.
GET /o/:cid returns full object bytes.
GET /o/:cid with valid range returns partial content.
Unknown well-formed CID returns 404.
Malformed CID returns client error.
```

The `http_blackbox.rs` test currently proves basic put/head/get/range behavior.

## 3.2 Paid Route Behavior

The paid route is now the WEB3 enforcement path.

Route:

```text
POST /paid/o
PUT  /paid/o
```

Behavior:

```text
/paid/o without proof                  → 402, no write
/paid/o with malformed proof            → 402, no write
/paid/o with bad accounting context     → 400, no write
/paid/o with valid dev proof            → 200, writes object
/paid/o in disabled mode                → 403, no write
/paid/o in wallet-receipt mode          → calls wallet receipt lookup
/paid/o with matching wallet receipt    → 200, writes object
/paid/o with mismatched wallet receipt  → 402, no write
```

The previous carryover notes documented the original paid route behavior and paid proof headers, including `x-ron-paid-op=hold`, `x-ron-paid-asset=roc`, positive `x-ron-paid-estimate-minor`, wallet txid, receipt hash, payer, and escrow. 

---

# 4. Paid Proof Requirements

The paid proof headers currently required by `DevHeaderVerifier` and used as the expected proof in wallet-receipt mode are:

```text
x-ron-paid-op = hold
x-ron-paid-asset = roc
x-ron-paid-estimate-minor = positive integer
x-ron-wallet-txid = non-empty txid
x-ron-wallet-receipt-hash = b3:<64 lowercase hex>
x-ron-wallet-from = payer account
x-ron-wallet-to = escrow account
```

Optional accounting headers:

```text
x-ron-tenant
x-ron-accounting-subject
x-ron-region
x-ron-pin-seconds
```

The `WalletReceipt` contract validates only wallet hold receipts:

```text
op = hold
asset = roc
amount_minor = positive integer string
from = payer present
to = escrow present
receipt_hash = b3:<64 lowercase hex>
```

Tests prove rejection for:

```text
wrong operation
wrong asset
zero amount
non-integer amount
missing payer
missing escrow
bad receipt hash
missing wallet lookup receipt
mismatched txid
mismatched payer
mismatched escrow
mismatched asset
mismatched amount
mismatched receipt hash
```

---

# 5. Paid Write Verifier System

We now have a dedicated paid-write verifier subsystem under:

```text
crates/svc-storage/src/policy/paid_write.rs
```

Current components:

```text
PaidWriteVerifier trait
WalletReceiptLookup trait
DevHeaderVerifier
WalletReceiptVerifier
FailClosedWalletReceiptLookup
WalletReceiptHttpClient
WalletReceipt DTO
PaidWriteProof
VerifiedPaidWrite
PaidWriteVerificationError
```

## 5.1 DevHeaderVerifier

Current dev/test verifier.

Behavior:

```text
reads proof directly from request headers
validates hold/roc/amount/payer/escrow/receipt hash
accepts only valid paid hold metadata
does not call svc-wallet
```

This is useful for local dev, micronode demo, and simple test harnesses.

## 5.2 WalletReceiptVerifier

Mockable verifier using `WalletReceiptLookup`.

Behavior:

```text
extract expected proof from headers
lookup wallet receipt by txid
validate receipt as hold proof
compare looked-up receipt to expected proof
reject mismatches
```

This proved the production verifier contract before adding outbound HTTP.

## 5.3 WalletReceiptHttpClient

Real HTTP receipt lookup client.

Behavior:

```text
base_url + /v1/tx/{txid}
optional Bearer token
timeout
Accept: application/json
rejects non-2xx
rejects bad JSON
rejects unsafe txid path segments
parses WalletReceipt
validates hold receipt
compares against expected headers
fails closed on every network/parse/validation problem
```

The HTTP client tests currently prove:

```text
fetches wallet receipt with bearer
verify_headers accepts matching receipt
fails closed without bearer
fails closed on missing receipt
fails closed on bad JSON
rejects unsafe txid before network
rejects invalid base URL
```

The terminal confirms `paid_write_http_clients.rs` passed 7/7. 

## 5.4 Route-Level Wallet Mode

The route now supports wallet receipt mode through env/config.

The dedicated route-level test proves:

```text
wallet-receipt mode uses the wallet HTTP lookup path
matching wallet receipt allows /paid/o to store object
mismatched wallet receipt rejects payment
rejected mismatch does not store the object
```

This was the most important storage-side production-hardening proof in the latest session.

---

# 6. Paid Write Configuration

We added the config module and paid verifier mode.

Important files:

```text
crates/svc-storage/src/config.rs
crates/svc-storage/src/lib.rs
```

`lib.rs` now exposes:

```rust
pub mod config;
```

Current env vars:

```text
RON_STORAGE_PAID_WRITE_VERIFIER_MODE
RON_STORAGE_WALLET_BASE_URL
RON_STORAGE_WALLET_BEARER
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS
```

Supported modes:

```text
dev-header
wallet-receipt
disabled
```

Aliases accepted:

```text
dev
dev-header
dev_header
header
headers

wallet
wallet-receipt
wallet_receipt
walletreceipt
wallet-receipt-verifier
wallet_receipt_verifier
production
prod

disabled
disable
off
false
0
```

Default:

```text
dev-header
```

Reason:

```text
keeps current local beta/dev tests green
avoids breaking existing /paid/o dev loop
allows explicit wallet-receipt mode for stronger tests and beta/prod hardening
```

Production expectation:

```text
production should use wallet-receipt or disabled
production should not accidentally use dev-header
```

Config tests currently prove stable spelling, display format, and rejection of unknown spelling. The latest terminal output confirms the three config tests pass. 

---

# 7. Cargo / Dependency Changes

`reqwest` is now a production dependency, not only a dev dependency, because `WalletReceiptHttpClient` lives in production source under `src/policy/paid_write.rs`.

Current dependency reasoning:

```text
reqwest in [dependencies]
  required by WalletReceiptHttpClient production path

tower in [dev-dependencies]
  used for Router::oneshot tests

svc-wallet in [dev-dependencies]
  used only for integration proofs

ron-accounting in [dev-dependencies]
  used only for integration proofs
```

This preserves the correct production boundary:

```text
svc-storage production code can call wallet over HTTP
svc-storage production code does not embed svc-wallet
svc-storage production code does not embed ron-accounting
```

Older codebundle snippets showed `reqwest` as dev-only before the HTTP wallet client existed, which was correct then, but it is now intentionally a normal dependency. 

---

# 8. Accounting Usage Events

`/paid/o` emits accounting-compatible usage event DTOs in the response.

Current emitted usage event kinds:

```text
bytes_stored
request_ok
pin_seconds, when requested
```

Optional context fields come from headers:

```text
x-ron-tenant
x-ron-accounting-subject
x-ron-region
x-ron-pin-seconds
```

The cross-crate paid storage loop proves:

```text
storage emits accounting-compatible usage events
ron-accounting can ingest those events
```

Current state:

```text
events are returned in response
tests ingest them in-process
production exporter is not yet implemented
```

Older notes identify this clearly: production still needs a real export path such as HTTP to `ron-accounting`, a bus event, local WAL/export file, or batch forwarder. 

---

# 9. Metrics

Paid-write metrics currently exist and are exposed through `/metrics`.

Current paid metrics:

```text
storage_paid_write_total{status="accepted"}
storage_paid_write_total{status="payment_required"}
storage_paid_write_total{status="bad_accounting_context"}
storage_paid_write_total{status="storage_error"}
storage_paid_write_total{status="disabled"}
storage_paid_write_total{status="config_error"}
storage_paid_write_bytes_total
```

Tests prove paid-write metrics expose admission statuses.

Remaining broader storage metrics still needed:

```text
storage_put_total
storage_get_total
storage_head_total
storage_bytes_written_total
storage_bytes_read_total
storage_put_latency_seconds
storage_get_latency_seconds
storage_head_latency_seconds
storage_objects_total
storage_fs_errors_total
```

Older storage notes listed these broader golden counters/histograms as part of the intended observability contract. 

---

# 10. Cross-Crate Paid Storage Loop

The current cross-crate loop is one of the strongest WEB3 proofs.

`web3_paid_storage_loop.rs` proves:

```text
unfunded wallet hold rejects
unfunded storage write does not create object
funded payer receives ROC
wallet hold reserves ROC into escrow
/paid/o accepts hold proof and stores object
GET /o/:cid returns exact object bytes
storage emits accounting-compatible usage events
ron-accounting ingests those events
wallet capture pays svc_storage
wallet release refunds payer
final wallet value is conserved
```

This demonstrates the current intended internal ROC flow:

```text
wallet issue
→ wallet hold
→ paid storage write
→ accounting usage ingest
→ wallet capture
→ wallet release
→ conserved balances
```

Older notes already identified this as the strongest proof that the value plane is not only crate-local. 

---

# 11. Current Test Inventory

Current key tests:

```text
tests/http_blackbox.rs
tests/paid_write_policy.rs
tests/paid_write_verifier.rs
tests/paid_write_http_clients.rs
tests/paid_write_wallet_mode.rs
tests/web3_paid_storage_loop.rs
```

## 11.1 `http_blackbox.rs`

Proves:

```text
POST/PUT object storage
HEAD metadata
GET exact bytes
range read behavior
```

## 11.2 `paid_write_policy.rs`

Proves:

```text
missing proof rejects
malformed proof rejects
bad accounting context rejects
valid minimal paid proof stores object
paid-write metrics expose admission statuses
```

## 11.3 `paid_write_verifier.rs`

Proves:

```text
DevHeaderVerifier accepts valid hold metadata
DevHeaderVerifier rejects non-hold proof
DevHeaderVerifier rejects bad receipt hash
WalletReceipt DTO validates hold receipts
WalletReceipt DTO rejects invalid receipt shapes
WalletReceiptVerifier fails closed until wired
WalletReceiptVerifier accepts matching mock lookup receipt
WalletReceiptVerifier rejects lookup mismatch cases
```

## 11.4 `paid_write_http_clients.rs`

Proves:

```text
HTTP wallet receipt client fetches with bearer
matching HTTP wallet receipt verifies
missing bearer fails closed
missing receipt fails closed
bad JSON fails closed
unsafe txid rejected before network
invalid base URL rejected
```

Note: the test target is plural:

```bash
cargo test -p svc-storage --test paid_write_http_clients
```

## 11.5 `paid_write_wallet_mode.rs`

Proves:

```text
/paid/o in wallet-receipt mode calls mock wallet
matching wallet receipt stores object
mismatched wallet receipt rejects
mismatched wallet receipt does not store object
usage_events include expected accounting event kinds
```

## 11.6 `web3_paid_storage_loop.rs`

Proves the wider wallet/storage/accounting/capture/release path.

---

# 12. Current Completion Estimate

Updated estimates after the latest wallet-receipt route-level green run:

```text
svc-storage CAS baseline:                  ~88–92%
svc-storage paid-write internal beta path: ~93–96%
svc-storage production paid storage:       ~82–88%
```

Overall interpretation:

```text
The paid-storage beta path is now very strong.
The production path is no longer theoretical.
The remaining gaps are mostly live deployment hardening, settlement automation, event export, proof binding, and broader CAS durability/observability.
```

---

# 13. What Remains for `svc-storage`

## 13.1 Live Multi-Service Smoke

Current wallet-receipt route-level mode uses a mock wallet server.

Remaining:

```text
run real svc-wallet on 127.0.0.1:8088
run real svc-storage
set RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
set RON_STORAGE_WALLET_BASE_URL=http://127.0.0.1:8088
set RON_STORAGE_WALLET_BEARER=dev
create wallet hold
POST /paid/o with txid and receipt hash
verify storage writes object only if real wallet receipt matches
```

Why it matters:

```text
mock wallet proves the route/client contract
live wallet smoke proves deployment wiring
```

## 13.2 Stronger Proof Binding

Current paid proof verifies that a matching wallet hold receipt exists.

Future proof should bind the hold to the actual storage operation.

Potential context fields:

```text
expected CID
payer
escrow
service = svc-storage
route = /paid/o
max bytes
estimated price
expiry timestamp
idempotency key
asset = roc
operation = paid_storage_put
```

Remaining:

```text
define wallet hold context/memo schema
extend svc-wallet hold receipt to include context
verify context in svc-storage before write
reject receipt that was valid but intended for another service/route/CID
```

This is the next major anti-replay/anti-misuse hardening step.

## 13.3 Capture/Release Automation

Current integration test does:

```text
wallet hold
storage write
manual/test capture
manual/test release
```

Production needs a decision:

```text
Option A: svc-storage directly calls wallet capture/release after write
Option B: svc-storage emits settlement event and a billing coordinator captures/releases
Option C: gateway/omnigate orchestrates hold/capture/release around storage
```

Remaining:

```text
decide settlement ownership
add capture/release adapter or event path
handle write succeeds but capture fails
handle write fails after hold succeeds
handle release retry
define settlement idempotency key
emit final settlement event
```

Recommended beta path:

```text
gateway/omnigate or a small billing coordinator should orchestrate settlement
svc-storage should stay focused on proof admission + storage + usage events
```

## 13.4 Production Accounting Export

Current:

```text
usage events are returned in /paid/o response
tests ingest them into ron-accounting in-process
```

Production needs:

```text
HTTP event ingestion into ron-accounting
or bus event
or local WAL/export queue
or batch forwarder
```

Remaining:

```text
define event delivery mechanism
add retry/backoff
add idempotency key for usage events
add dead-letter/quarantine behavior
add metrics for export success/failure
test duplicate event handling
```

## 13.5 Durable Storage Backend / Amnesia Decision

Current tests use memory/in-process storage.

Need explicit beta decision:

```text
Micronode dev/amnesia:
  MemoryStorage or RAM-only storage acceptable

Beta durable node:
  FS backend or sled-backed CAS needed

Production:
  fan-out path layout, fsync policy, repair/replication plan
```

Remaining:

```text
confirm whether beta needs durable FS backend
add restart/replay persistence tests if durable
define amnesia-vs-durable config
ensure amnesia mode never spills to disk
```

## 13.6 Strict CID APIs and Hash Verification

Older notes specify strict `PUT /v1/objects/{cid}` behavior where the path CID must match BLAKE3(bytes). Current `/o` computes and returns CID.

Remaining possible hardening:

```text
add PUT /o/:cid or /v1/objects/:cid with hash verification
reject body whose BLAKE3 does not match CID
return 409 on same CID with different bytes
preserve POST /o compute-CID route for convenience
```

This matters for deterministic upstream bundles/manifests.

## 13.7 Broader Range / Conditional GET Hardening

Current range smoke exists.

Remaining:

```text
more range edge tests
unsatisfiable range tests
suffix range tests if supported
If-None-Match / conditional GET
ETag consistency tests
Content-Range correctness tests
large object streaming tests
```

## 13.8 Request Body Caps and Abuse Controls

Remaining:

```text
body cap enforcement tests
decompression guard tests if compressed requests accepted
rate limit integration through gateway
concurrency cap tests
memory pressure tests
large body rejection before full allocation
```

## 13.9 Full Metrics Expansion

Add broader storage metrics:

```text
storage_put_total{status}
storage_get_total{status,range}
storage_head_total{status}
storage_bytes_written_total
storage_bytes_read_total
storage_put_latency_seconds
storage_get_latency_seconds
storage_head_latency_seconds
storage_objects_total
storage_fs_errors_total{op}
```

## 13.10 Auth / Capability Integration

Current paid storage tests use dev bearer/dev headers/mock wallet.

Production should enforce:

```text
real ron-auth verification
svc-passport-derived capabilities
wallet receipt lookup authorization
storage write scopes
service-scoped receipt lookup caveats
passport/account/asset caveats
```

Remaining:

```text
define storage:put capability
define paid_storage_put capability
define wallet receipt lookup service cap
verify bearer/cap at storage/gateway boundary
avoid trusting raw headers in production
```

## 13.11 WEB3 2.0 b3 Asset Pages

The new WEB3 2.0 blueprint introduces b3 asset pages.

`svc-storage` will likely support the raw-byte side:

```text
store raw uploaded bytes
compute BLAKE3 CID
serve bytes by CID
provide metadata needed by asset manifest flow
support HEAD existence checks for svc-index
```

But `svc-storage` should not own:

```text
asset ownership claims
curator tags
site/page manifests
crab link resolution
name resolution
payout percentage policy
```

Those belong to:

```text
ron-naming
svc-index
ron-policy
svc-gateway / omnigate
svc-wallet
```

Potential storage-adjacent future work:

```text
return stable CID on upload for every image/post/comment/page/song/video
ensure b3 hash is canonical for WEB3 2.0 asset pages
support HEAD /o/:cid for index existence verification
avoid storing human names inside storage
```

---

# 14. Recommended Next Steps

## Immediate Next Step

Do a live two-process smoke:

```text
real svc-wallet
real svc-storage
wallet-receipt mode
actual GET /v1/tx/{txid}
actual /paid/o write
actual GET /o/:cid readback
```

## Next Code Work Packages

Recommended order:

```text
1. Add live smoke script for svc-wallet + svc-storage wallet-receipt mode.
2. Add stronger hold context/memo schema.
3. Add storage verification of hold context.
4. Decide capture/release ownership.
5. Add production accounting event exporter.
6. Expand storage golden metrics.
7. Add strict CID path write / hash verification route if desired.
8. Add b3 asset-page support hooks for WEB3 2.0, without making storage resolve names.
```

## Suggested New Tests

```text
paid_write_live_wallet_smoke.rs or script:
  real svc-wallet receipt lookup path

paid_write_hold_context.rs:
  receipt intended for different route rejects
  receipt intended for different CID rejects
  expired receipt rejects

paid_write_settlement.rs:
  storage success triggers capture/release or emits settlement event
  storage failure releases hold
  capture failure is idempotently recoverable

accounting_export.rs:
  paid write exports usage event to accounting adapter
  duplicate event is idempotent
  export failure goes to retry/dead-letter

cid_strict_put.rs:
  PUT /o/:cid matching bytes succeeds
  PUT /o/:cid mismatched bytes rejects
```

---

# 15. Known Commands

Fresh check:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

Focused paid-write tests:

```bash
cargo test -p svc-storage --test paid_write_policy
cargo test -p svc-storage --test paid_write_verifier
cargo test -p svc-storage --test paid_write_http_clients
cargo test -p svc-storage --test paid_write_wallet_mode
cargo test -p svc-storage --test web3_paid_storage_loop
```

Wallet-receipt mode env shape:

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_WALLET_BASE_URL=http://127.0.0.1:8088
RON_STORAGE_WALLET_BEARER=dev
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS=2000
```

Dev-header mode:

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=dev-header
```

Disabled mode:

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=disabled
```

---

# 16. Risks / Guardrails

Do not regress these:

```text
Do not remove BLAKE3 b3:<64 lowercase hex> as canonical CID.
Do not add name resolution inside svc-storage.
Do not let svc-storage mutate ledger directly.
Do not let svc-storage embed svc-wallet in production code.
Do not make ron-accounting a direct production dependency unless architecture explicitly changes.
Do not make dev-header mode the accidental production default without deployment safeguards.
Do not trust headers in production without wallet receipt lookup.
Do not allow wallet receipt lookup mismatches.
Do not store objects after failed paid verification.
Do not silently persist in amnesia mode.
```

Current safest production posture:

```text
wallet-receipt mode or disabled
never dev-header by accident
```

---

# 17. Short Commit Summary

Suggested commit summary for the recent storage work:

```text
web3: harden svc-storage paid writes with wallet receipt mode
```

Suggested commit body:

```text
- Add paid-write verifier mode config for dev-header, wallet-receipt, and disabled
- Add WalletReceiptLookup seam and mock verifier tests
- Add WalletReceiptHttpClient for GET /v1/tx/{txid}
- Move reqwest into production dependencies for svc-storage wallet lookup
- Add fail-closed HTTP receipt lookup tests
- Wire /paid/o wallet-receipt mode to real HTTP wallet lookup
- Add route-level wallet-receipt mode test with mock svc-wallet
- Prove matching wallet hold receipt stores object
- Prove mismatched wallet receipt rejects and does not store object
- Preserve dev-header default for local beta/dev compatibility
- Keep /o CAS route unchanged
- Keep paid storage loop green with svc-wallet and ron-accounting
```

---

# 18. TL;DR

`svc-storage` is now a working CAS service and a strong WEB3 paid-use enforcement point.

The current state proves:

```text
free CAS works
paid CAS writes reject without proof
dev paid proof works
wallet receipt DTO contract works
mock wallet receipt verifier works
real HTTP wallet receipt client works
wallet-receipt route mode works
mismatched wallet receipt rejects
paid storage emits accounting-compatible usage events
cross-crate wallet/storage/accounting loop works
```

The biggest remaining production gaps are:

```text
live real svc-wallet + svc-storage smoke
stronger hold context binding
capture/release automation
real accounting export
durable storage/amnesia decision
full storage metrics expansion
real auth/passport/capability integration
```

Current estimate:

```text
svc-storage paid WEB3 beta path: ~93–96%
svc-storage production paid storage: ~82–88%
```

`svc-storage` is now very close to beta-ready for the internal ROC paid storage loop.

```
```


### END NOTE - APRIL 28 2026 - 15:10 CST


### BEGIN NOTE - APRIL 28 2026 - 19:30 CST

Absolutely. Here is a **paste-ready `svc-storage` progress note** you can drop into:

```text
crates/svc-storage/NOTES.MD
```

This reflects the current green state after the live paid-storage settlement smoke. The latest terminal run shows `svc-storage` clippy passing, `paid_write_settlement` passing, all `svc-storage` tests passing, the live `svc-wallet + svc-storage` smoke running successfully, wallet-backed capture/release receipts being returned, final balances matching expected values, and replay with different bytes being rejected.   

````md
---

### BEGIN NOTE - APRIL 28/29 2026 - WEB3 PAID STORAGE SETTLEMENT GREEN

# svc-storage NOTES.MD — WEB3 Paid Storage / Wallet Receipt / Settlement Plateau

Date: 2026-04-28 / 2026-04-29  
Crate: `svc-storage`  
Workspace path: `crates/svc-storage`  
Project: RustyOnions / RON-CORE / WEB3 / ROC internal value plane  
Current status: **WEB3 paid-storage beta loop is live-smoke green with wallet-backed settlement**  

Current estimated completion:

```text
svc-storage CAS baseline:                  ~90–94%
svc-storage paid-write beta path:          ~97–99%
svc-storage production paid-storage path:  ~88–93%
svc-storage WEB3 readiness overall:        ~94–97%
````

The remaining gap is no longer “does paid storage work?” It does. The remaining gap is now production hardening:

```text
real accounting export path
settlement failure recovery / quarantine
broader metrics
durable storage / amnesia posture decisions
gateway/omnigate/passport integration
b3 asset manifest integration
```

---

# 0. Executive Summary

`svc-storage` has become the first fully live paid-use enforcement point for the RustyOnions internal ROC economy.

It now supports the core beta-shaped paid storage lifecycle:

```text
wallet issue
→ wallet hold
→ paid storage receipt verification
→ content-addressed object write
→ wallet capture of actual storage cost
→ wallet release of unused hold remainder
→ object readback by canonical b3 CID
→ replay rejection for mismatched bytes/context
```

This is a major milestone because it proves the first real in-network ROC use case:

```text
pay ROC to store/pin content in RustyOnions
```

The latest live smoke proved:

```text
svc-wallet starts and reports /healthz
svc-storage starts and reports /healthz
wallet issues 100 ROC to acct_user
wallet holds 70 ROC into escrow_paid_write
svc-storage verifies the wallet hold receipt over HTTP
svc-storage binds the hold to the exact paid-storage context/CID
svc-storage stores the object by canonical b3:<64 lowercase hex> CID
svc-storage calls wallet capture for actual cost: 48 ROC
svc-storage calls wallet release for remainder: 22 ROC
object reads back successfully
final balances are conserved:
  acct_user = 52
  escrow_paid_write = 0
  svc_storage = 48
same hold cannot be replayed for different bytes
```

That means the crate now proves the WEB3 invariant:

```text
paid operations use hold → capture → release
svc-wallet remains the mutation front-door
svc-storage never directly mutates ledger state
content is addressed by canonical BLAKE3 b3 CID
```

This aligns with the WEB3/TODO strategy that identified paid storage/pinning as the fastest path toward 90% beta completion, with paid enforcement in storage/gateway/omnigate and economic mutation flowing through wallet/ledger boundaries.

---

# 1. Strategic Role of `svc-storage`

`svc-storage` is the content-addressed object storage service. It owns raw byte storage and byte retrieval.

Its current WEB3 role is:

```text
store immutable bytes by canonical content ID
serve objects by BLAKE3 b3:<64 lowercase hex>
support HEAD metadata checks
support range reads
enforce paid writes through /paid/o
validate wallet hold receipts
bind paid holds to exact storage operation context
emit accounting-compatible usage events
optionally settle paid writes through svc-wallet capture/release
```

Its role is **not**:

```text
not the ledger
not the wallet
not the durable accounting truth
not the reward calculator
not the name resolver
not the passport authority
not the external-chain bridge
not the place for ROX/Solana logic
```

Correct boundaries remain:

```text
svc-storage:
  CAS bytes
  b3 CID storage and retrieval
  paid-write admission
  wallet receipt verification
  optional wallet-backed settlement calls
  usage event emission
  paid-write metrics

svc-wallet:
  balance mutation front-door
  issue/transfer/burn/hold/capture/release
  nonce/idempotency enforcement
  stable receipts
  ledger commit path

ron-ledger:
  durable append-only economic truth
  replay
  conservation
  non-negativity
  crash-safe economic record

ron-accounting:
  transient usage counters
  storage usage event ingestion
  sealed snapshots
  reward projection inputs

svc-rewarder:
  deterministic reward planning
  payout/issue intent generation
  no direct ledger mutation

svc-index / ron-naming:
  names, manifests, lookup, b3 asset metadata
  not raw byte storage
```

---

# 2. Current Feature Inventory

`svc-storage` now has the following confirmed capabilities.

## 2.1 Core CAS behavior

Implemented/proven:

```text
free/dev CAS object writes
free/dev CAS object reads
HEAD object metadata checks
GET exact bytes
range read behavior
ETag/hash-oriented behavior
BLAKE3 b3:<64 lowercase hex> addressing
in-memory storage backend for tests/dev
```

Current object identity rule:

```text
CID = b3:<BLAKE3-256 digest of object bytes>
```

Important invariant:

```text
Storage stores bytes by CID.
Storage must not resolve names.
Storage must not own manifests.
Storage must not decide ownership/payout metadata.
Names/manifests/asset pages belong upstream in svc-index, ron-naming, svc-gateway, omnigate, and future WEB3_2 product layers.
```

## 2.2 Paid `/paid/o` route

Implemented/proven:

```text
POST /paid/o
valid paid proof stores object
missing paid proof rejects
malformed paid proof rejects
bad accounting context rejects
wallet receipt mismatch rejects
wallet receipt replay for different bytes rejects
successful paid write returns:
  cid
  paid=true
  payer
  escrow
  wallet_txid
  wallet_receipt_hash
  wallet_idem
  paid_context_idem
  verifier
  estimate_minor
  optional settlement receipts
  usage_events
```

The paid route is now the first concrete enforcement seam for ROC-backed storage.

## 2.3 Paid-write verifier modes

Current modes:

```text
dev-header
wallet-receipt
disabled
```

### `dev-header`

Purpose:

```text
local development
tests
simple manual experiments
no wallet service required
```

Behavior:

```text
trusts explicit request headers
validates shape/op/asset/amount/hash/payer/escrow
does not call svc-wallet
does not require context binding
```

Risk:

```text
should not be used accidentally in production
```

### `wallet-receipt`

Purpose:

```text
production-shaped beta mode
real wallet receipt lookup
paid write admission based on svc-wallet truth
```

Behavior:

```text
uses headers only as expected values
calls svc-wallet GET /v1/tx/{txid}
fetches wallet receipt
validates receipt fields against expected request proof
requires wallet receipt idem for context binding
rejects mismatch
fails closed
```

### `disabled`

Purpose:

```text
fail-closed paid route
safe production posture if paid storage should not be active
```

Behavior:

```text
rejects /paid/o
does not store object
```

---

# 3. Wallet Receipt Verification

The paid-write verifier now supports a production-shaped wallet receipt path.

Important types/functions:

```text
WalletReceipt
PaidWriteProof
VerifiedPaidWrite
PaidWriteVerifier
WalletReceiptLookup
DevHeaderVerifier
WalletReceiptVerifier
WalletReceiptHttpClient
paid_storage_context_idem(...)
```

## 3.1 `WalletReceipt`

The accepted wallet receipt shape includes:

```text
txid
op
from
to
asset
amount_minor
nonce
idem
ts
ledger_seq_start
ledger_seq_end
ledger_root
receipt_hash
```

For paid storage, a valid receipt must satisfy:

```text
op == "hold"
asset == "roc"
amount_minor is positive integer
txid is present
receipt_hash is canonical b3:<64 lowercase hex>
from/payer is present
to/escrow is present
```

## 3.2 Header expectations

Paid write request headers currently include:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <integer>
x-ron-wallet-txid: <wallet txid>
x-ron-wallet-receipt-hash: b3:<64 lowercase hex>
x-ron-wallet-from: <payer account>
x-ron-wallet-to: <escrow account>
x-ron-wallet-idem: <paid storage context idem>
```

Optional accounting context headers:

```text
x-ron-tenant: <integer>
x-ron-accounting-subject: <subject>
x-ron-region: <region>
x-ron-pin-seconds: <integer>
```

## 3.3 Context binding

A major anti-replay improvement is now implemented.

`svc-storage` computes a deterministic paid-storage context idempotency key from:

```text
service = svc-storage
route = /paid/o
operation = paid_storage_put
cid = b3:<hash of body bytes>
payer
escrow
asset
amount_minor
```

The resulting key has the form:

```text
storage_put:<32 lowercase hex chars>
```

The live smoke demonstrated:

```text
paid_context_idem == wallet_idem
```

And replaying the same valid hold against different bytes produced:

```text
payment_required:
wallet receipt paid storage context idem mismatch
```

This proves a valid hold for one CID cannot be reused for another body.

---

# 4. Settlement Automation

`svc-storage` now has an opt-in wallet-backed settlement mode.

Current settlement modes:

```text
disabled
wallet-capture
```

Environment variable:

```bash
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
```

Default:

```text
disabled
```

Reason for default:

```text
existing dev/test flows should not unexpectedly mutate wallet state
settlement must be explicit
```

## 4.1 `wallet-capture` behavior

When enabled:

```text
1. Verify paid write hold receipt.
2. Bind hold to exact storage CID/context.
3. Store object.
4. Compute actual storage cost.
5. Call svc-wallet /v1/capture from escrow to storage payee.
6. Call svc-wallet /v1/release from escrow back to payer for remaining amount.
7. Return capture and release receipts in /paid/o response.
```

Current beta pricing model:

```text
actual_capture_amount_minor = max(bytes_stored, 1)
```

Example from live smoke:

```text
object bytes: 48
hold amount: 70
capture amount: 48
release amount: 22
```

## 4.2 Settlement response shape

`/paid/o` returns:

```json
"settlement": {
  "mode": "wallet-capture",
  "capture_amount_minor": "48",
  "release_amount_minor": "22",
  "capture_receipt": {
    "op": "capture",
    "from": "escrow_paid_write",
    "to": "svc_storage",
    "asset": "roc",
    "amount_minor": "48",
    "idem": "storage_cap:<...>",
    "receipt_hash": "b3:<...>"
  },
  "release_receipt": {
    "op": "release",
    "from": "escrow_paid_write",
    "to": "acct_user",
    "asset": "roc",
    "amount_minor": "22",
    "idem": "storage_rel:<...>",
    "receipt_hash": "b3:<...>"
  }
}
```

## 4.3 Settlement idempotency

Settlement uses deterministic idempotency keys:

```text
capture:
  storage_cap:<32 lowercase hex chars>

release:
  storage_rel:<32 lowercase hex chars>

failed-write release:
  storage_failrel:<32 lowercase hex chars>
```

These are derived from canonical settlement material such as:

```text
kind
hold_txid
cid
payer
escrow
payee
asset
amount_minor
```

## 4.4 Current settlement failure behavior

Current behavior:

```text
storage write failure before settlement:
  attempts release_failed_paid_storage(...)
  returns storage error

settlement failure after successful write:
  returns 502 settlement_failed
  increments/observes settlement_error path
```

Remaining hardening needed:

```text
persistent failed-settlement quarantine
retry/backoff
dead-letter queue
admin visibility for unresolved settlement failures
idempotent reconciliation job
```

This is now a production hardening issue, not an MVP architecture blocker.

---

# 5. Local Helper Binary

Added helper:

```text
crates/svc-storage/src/bin/roc_b3_tool.rs
```

Purpose:

```text
avoid requiring external b3sum tool
compute canonical b3 CID from local file
compute paid-storage context idem using same code path as service
```

Commands:

```bash
cargo run -q -p svc-storage --bin roc_b3_tool -- cid <path>
cargo run -q -p svc-storage --bin roc_b3_tool -- context-idem <cid> <payer> <escrow> <asset> <amount_minor>
```

Examples:

```bash
cargo run -q -p svc-storage --bin roc_b3_tool -- cid target/web3-smoke/paid-object.bin

cargo run -q -p svc-storage --bin roc_b3_tool -- context-idem \
  b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57 \
  acct_user \
  escrow_paid_write \
  roc \
  70
```

This helper is used by:

```text
scripts/web3_paid_storage_live_smoke.sh
```

---

# 6. Files Added / Updated in This Work

## 6.1 Added

```text
crates/svc-storage/src/bin/roc_b3_tool.rs
crates/svc-storage/src/policy/settlement.rs
crates/svc-storage/tests/paid_write_settlement.rs
scripts/web3_paid_storage_live_smoke.sh
```

## 6.2 Updated

```text
crates/svc-storage/src/config.rs
crates/svc-storage/src/policy/mod.rs
crates/svc-storage/src/policy/paid_write.rs
crates/svc-storage/src/http/routes/paid_object.rs
crates/svc-storage/tests/paid_write_wallet_mode.rs
```

## 6.3 Existing tests still relevant

```text
crates/svc-storage/tests/http_blackbox.rs
crates/svc-storage/tests/paid_write_policy.rs
crates/svc-storage/tests/paid_write_verifier.rs
crates/svc-storage/tests/paid_write_http_clients.rs
crates/svc-storage/tests/web3_paid_storage_loop.rs
```

---

# 7. Test Inventory and What Each Test Proves

## 7.1 `http_blackbox.rs`

Proves:

```text
basic object storage
HEAD metadata
GET exact bytes
range read behavior
```

## 7.2 `paid_write_policy.rs`

Proves:

```text
missing paid proof rejects
malformed paid proof rejects
bad accounting context rejects
valid minimal paid proof stores object
paid-write metrics expose admission statuses
object is not stored on failed admission
```

## 7.3 `paid_write_verifier.rs`

Proves:

```text
DevHeaderVerifier accepts valid hold metadata
DevHeaderVerifier rejects non-hold operation
DevHeaderVerifier rejects bad receipt hash
WalletReceipt DTO validates hold/roc/positive amount
WalletReceipt DTO rejects invalid receipt shapes
WalletReceiptVerifier fails closed until lookup is wired
WalletReceiptVerifier accepts matching lookup receipt
WalletReceiptVerifier rejects wrong amount
WalletReceiptVerifier rejects wrong asset
WalletReceiptVerifier rejects wrong escrow
WalletReceiptVerifier rejects wrong hash
WalletReceiptVerifier rejects wrong operation
WalletReceiptVerifier rejects wrong payer
WalletReceiptVerifier rejects missing lookup receipt
```

## 7.4 `paid_write_http_clients.rs`

Proves:

```text
HTTP wallet receipt client fetches with bearer token
matching HTTP wallet receipt verifies
missing bearer fails closed
missing receipt fails closed
bad JSON fails closed
unsafe txid rejected before network
invalid base URL rejected
```

## 7.5 `paid_write_wallet_mode.rs`

Proves:

```text
/paid/o in wallet-receipt mode calls mock wallet
matching wallet receipt stores object
mismatched wallet receipt rejects
mismatched receipt does not store object
same valid hold cannot be reused for a different CID/body
usage_events include bytes_stored/request_ok/pin_seconds
```

## 7.6 `paid_write_settlement.rs`

Proves:

```text
/paid/o in wallet-receipt + wallet-capture mode works
matching hold receipt stores object
settlement calls mock wallet capture endpoint
settlement calls mock wallet release endpoint
capture amount equals actual byte cost
release amount equals hold remainder
capture receipt has op=capture
release receipt has op=release
object reads back after settlement
```

## 7.7 `web3_paid_storage_loop.rs`

Proves the in-process cross-crate WEB3 flow:

```text
wallet issue
wallet hold
paid storage admission
usage events compatible with ron-accounting
wallet capture
wallet release
balance conservation
```

## 7.8 Live script

```text
scripts/web3_paid_storage_live_smoke.sh
```

Proves real multi-process deployment wiring:

```text
real svc-wallet process
real svc-storage process
wallet health
storage health
issue ROC
hold ROC
wallet receipt lookup over HTTP
paid object write
capture through wallet
release through wallet
read object back
check final balances
reject replay for different bytes
```

---

# 8. Commands

## 8.1 Full local validation

Run from repo root:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

Latest known result:

```text
clippy clean
all svc-storage tests passing
bench targets compile as test targets
```

## 8.2 Focused paid-write tests

```bash
cargo test -p svc-storage --test paid_write_policy
cargo test -p svc-storage --test paid_write_verifier
cargo test -p svc-storage --test paid_write_http_clients
cargo test -p svc-storage --test paid_write_wallet_mode
cargo test -p svc-storage --test paid_write_settlement
cargo test -p svc-storage --test web3_paid_storage_loop
```

## 8.3 Live paid storage settlement smoke

```bash
chmod +x scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_storage_live_smoke.sh
```

What it starts by default:

```text
svc-wallet:
  127.0.0.1:18088

svc-storage:
  127.0.0.1:15303
```

Override ports if needed:

```bash
SVC_WALLET_ADDR=127.0.0.1:18088 \
ADDR=127.0.0.1:15303 \
bash scripts/web3_paid_storage_live_smoke.sh
```

Logs:

```text
target/web3-smoke/svc-wallet.log
target/web3-smoke/svc-storage.log
```

Generated files:

```text
target/web3-smoke/paid-object.bin
target/web3-smoke/paid-object.get.bin
target/web3-smoke/bad-object.bin
target/web3-smoke/bad-response.json
```

---

# 9. Environment Variables

## 9.1 Service bind / storage config

```bash
RON_STORAGE_ADDR=127.0.0.1:5303
ADDR=127.0.0.1:15303
RON_STORAGE_DATA_DIR=./data/storage
RON_STORAGE_MAX_BODY=67108864
```

Note:

```text
The live script currently uses ADDR for svc-storage bind, matching the current binary behavior.
```

## 9.2 Paid verifier mode

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=dev-header
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=disabled
```

Recommended posture:

```text
local tests/dev: dev-header or wallet-receipt
beta smoke: wallet-receipt
production if not ready: disabled
production paid mode: wallet-receipt
```

## 9.3 Wallet lookup / settlement config

```bash
RON_STORAGE_WALLET_BASE_URL=http://127.0.0.1:18088
RON_STORAGE_WALLET_BEARER=dev
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS=2000
```

## 9.4 Paid settlement mode

```bash
RON_STORAGE_PAID_SETTLEMENT_MODE=disabled
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
```

Recommended posture:

```text
local unit tests without wallet mutation:
  disabled

live paid-storage beta proof:
  wallet-capture

production beta:
  wallet-capture only with monitoring/reconciliation
```

## 9.5 Settlement payee

```bash
RON_STORAGE_PAID_SETTLEMENT_PAYEE=svc_storage
```

Default:

```text
svc_storage
```

## 9.6 Accounting context headers

Used on `/paid/o`:

```text
x-ron-tenant
x-ron-accounting-subject
x-ron-region
x-ron-pin-seconds
```

Defaults if omitted:

```text
tenant = 1
subject = svc_storage
region = local
pin_seconds = 0
```

---

# 10. Live Smoke Expected Output Shape

The live smoke should show:

```text
svc-wallet is healthy
svc-storage is healthy
object CID: b3:<64 lowercase hex>
context idem: storage_put:<32 lowercase hex>
expected capture: <bytes>
expected release: <hold - bytes>
issuing ROC to payer
creating wallet hold bound to paid-storage context
writing paid object through svc-storage; storage should capture and release through svc-wallet
reading object back
checking final wallet balances
proving same hold cannot be replayed for different bytes
WEB3 paid-storage live smoke with settlement green
```

Example final balances from the latest green run:

```text
payer balance:  52
escrow balance: 0
payee balance:  48
```

For the default smoke object:

```text
issue amount = 100
hold amount = 70
capture amount = 48
release amount = 22

payer = 100 - 70 + 22 = 52
escrow = 70 - 48 - 22 = 0
payee = 48
```

This proves conservation through the wallet mutation front-door.

---

# 11. Current Acceptance Gates Passed

Latest verified commands:

```bash
chmod +x scripts/web3_paid_storage_live_smoke.sh
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_settlement
cargo test -p svc-storage --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
```

Latest observed results:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings:
  passed

cargo test -p svc-storage --test paid_write_settlement:
  1 passed

cargo test -p svc-storage --all-targets:
  config tests:                 6 passed
  http_blackbox.rs:             1 passed
  paid_write_http_clients.rs:   7 passed
  paid_write_policy.rs:         5 passed
  paid_write_settlement.rs:     1 passed
  paid_write_verifier.rs:       19 passed
  paid_write_wallet_mode.rs:    1 passed
  web3_paid_storage_loop.rs:    1 passed
  bench targets:                compile as test targets

live smoke:
  svc-wallet healthy
  svc-storage healthy
  issue succeeded
  hold succeeded
  paid write succeeded
  capture receipt returned
  release receipt returned
  object readback succeeded
  final balances conserved
  replay with different bytes rejected
```

---

# 12. Key Invariants Now Proven

## 12.1 Paid write fail-closed

Proven:

```text
missing proof does not store
malformed proof does not store
wallet mismatch does not store
context mismatch does not store
disabled mode rejects paid writes
```

## 12.2 Wallet is mutation front-door

Proven:

```text
storage calls wallet endpoints
storage receives wallet receipts
storage does not directly mutate ledger
capture/release are wallet operations
receipts include ledger sequence/root fields
```

## 12.3 CID binding

Proven:

```text
request body determines b3 CID
hold context idem includes body CID
reusing same hold for different body computes different expected context
mismatch rejects with payment_required
```

## 12.4 Settlement conservation

Proven by live balances:

```text
payer decreases by actual captured cost
escrow returns to zero
payee receives actual captured cost
unused hold remainder returns to payer
```

## 12.5 Accounting event compatibility

Proven:

```text
paid write returns usage_events
events include bytes_stored
events include request_ok
events include pin_seconds when header is present
events include tenant/subject/region/route/source_service
web3_paid_storage_loop ingests compatible usage events in-process
```

---

# 13. Known Current Limitations

These are not blockers for beta proof, but they are important before production hardening.

## 13.1 Accounting export is not yet production-live

Current:

```text
usage_events are returned in /paid/o response
tests prove shape and in-process ingestion
```

Needed:

```text
real export path to ron-accounting
HTTP exporter or bus publisher or WAL/batch forwarder
retry/backoff
idempotency key per usage event
dead-letter/quarantine behavior
metrics for export success/failure
```

## 13.2 Settlement failure recovery is basic

Current:

```text
write failure attempts release
settlement failure returns settlement_failed
```

Needed:

```text
persistent failed-settlement event
retryable reconciliation
admin/metrics visibility
idempotent recovery job
clear policy for object stored but capture failed
dead-letter/quarantine after bounded attempts
```

## 13.3 Pricing is intentionally primitive

Current beta model:

```text
capture amount = bytes_stored minor units, minimum 1
```

Needed later:

```text
configs/roc-economics.toml integration
price per byte
pin-duration pricing
provider split
creator split
network reserve split
curator split
basis points only
integer minor units only
no floating point
```

## 13.4 Dev-header mode still exists

This is useful, but dangerous if accidentally enabled in production.

Needed:

```text
deployment guardrail
warning log on startup when dev-header mode is active
possibly require explicit RON_STORAGE_ALLOW_DEV_HEADER=1 in non-debug profile
```

## 13.5 Auth/capability integration is not final

Current:

```text
dev bearer tokens in live smoke
header-based paid route proof
wallet receipt lookup
```

Needed:

```text
real ron-auth / svc-passport capabilities
wallet caveats for account, asset, amount, action, route, CID, expiry
storage:put capability enforcement
short-lived wallet hold/capture/release permissions
```

## 13.6 Storage backend posture

Current:

```text
memory/dev paths are heavily tested
storage is good enough for beta proof
```

Needed depending on beta scope:

```text
durable FS backend confirmation
amnesia mode behavior
crash/restart persistence tests
content integrity verification on read
object count/GC policy
```

## 13.7 b3 asset page integration still belongs upstream

Current:

```text
storage stores bytes by CID
```

Needed in WEB3_2 layer:

```text
manifest-backed asset pages
ownership
payout address
tags
description
curator metadata
provenance
provider/storage receipts
crab://b3/<hash>.image etc.
```

Important boundary:

```text
Do not put asset-page semantic ownership logic inside svc-storage.
```

---

# 14. Recommended Next Work for `svc-storage`

## Priority 1 — Production accounting export path

Add a real usage exporter.

Suggested module:

```text
crates/svc-storage/src/accounting/
  mod.rs
  exporter.rs
  http.rs
  noop.rs
  types.rs
```

Suggested trait:

```text
StorageUsageExporter:
  export(event, idempotency_key) -> Result<(), ExportError>
```

Modes:

```text
noop
http
wal/batch later
```

Suggested env:

```bash
RON_STORAGE_ACCOUNTING_EXPORT_MODE=noop
RON_STORAGE_ACCOUNTING_BASE_URL=http://127.0.0.1:<port>
RON_STORAGE_ACCOUNTING_BEARER=dev
RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000
```

Acceptance:

```text
paid write exports bytes_stored
paid write exports request_ok
paid write exports pin_seconds
duplicate export is idempotent
export failure is observable
export failure does not silently disappear
```

## Priority 2 — Settlement recovery hardening

Add:

```text
failed settlement event type
local retry queue or WAL
structured logs
metrics
admin visibility
idempotent reconciliation command/test
```

Test scenarios:

```text
write succeeds, capture fails
capture succeeds, release fails
wallet timeout during capture
wallet timeout during release
retry same settlement idempotency key
manual reconcile succeeds
```

## Priority 3 — Metrics expansion

Add or verify:

```text
storage_put_total
storage_get_total
storage_paid_put_total
storage_paid_put_rejected_total
storage_wallet_receipt_lookup_total
storage_wallet_receipt_lookup_failed_total
storage_bytes_written_total
storage_bytes_read_total
storage_put_latency_seconds
storage_get_latency_seconds
storage_objects_total
storage_fs_errors_total
storage_accounting_export_total
storage_accounting_export_failed_total
storage_capture_total
storage_release_total
storage_settlement_failed_total
```

Acceptance:

```text
/metrics includes names
paid success increments success metrics
paid rejection increments rejection metrics
wallet lookup failure increments failure metric
capture/release increments settlement metrics
accounting export failure increments export failure metric
```

## Priority 4 — Startup guardrails

Add warnings/fail-closed checks:

```text
warn if dev-header mode active
warn if wallet-capture active without wallet-receipt verifier
fail if wallet-capture payee is empty
fail if wallet base URL malformed
expose mode in /readyz or /version/admin status
```

## Priority 5 — Strict CID write route, if desired

Potential future route:

```text
PUT /o/:cid
```

Behavior:

```text
hash body
compare to path CID
store only if exact match
reject mismatch
```

This is optional, because current `/paid/o` already computes CID itself and returns it.

## Priority 6 — WEB3_2 asset integration hooks

Keep storage simple:

```text
storage returns CID/metadata/availability
index/gateway/omnigate builds asset pages
storage may expose provider receipt/reference only
```

Do not add:

```text
ownership logic
name resolution
payout split logic
curator logic
site/app access policy
```

Those belong upstream.

---

# 15. Commands Reference

## Full crate gate

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

## Settlement-specific gate

```bash
cargo test -p svc-storage --test paid_write_settlement
```

## Wallet receipt route gate

```bash
cargo test -p svc-storage --test paid_write_wallet_mode
```

## HTTP client gate

```bash
cargo test -p svc-storage --test paid_write_http_clients
```

## Full live paid storage smoke

```bash
chmod +x scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_storage_live_smoke.sh
```

## Manual helper commands

```bash
cargo run -q -p svc-storage --bin roc_b3_tool -- cid <path>

cargo run -q -p svc-storage --bin roc_b3_tool -- context-idem \
  <cid> <payer> <escrow> <asset> <amount_minor>
```

---

# 16. Guardrails / Do Not Regress

Do not regress these:

```text
Do not remove b3:<64 lowercase hex> as canonical CID.
Do not add name resolution inside svc-storage.
Do not put manifest ownership logic inside svc-storage.
Do not let svc-storage mutate ledger directly.
Do not let svc-storage become a wallet.
Do not let svc-storage become ron-accounting.
Do not use floating-point money math.
Do not bypass wallet for capture/release.
Do not store paid object after failed proof.
Do not allow receipt replay for different body/CID.
Do not silently ignore settlement failure.
Do not silently ignore accounting export failure once exporter exists.
Do not rely on dev-header mode for production.
Do not introduce ROX/Solana/external settlement logic.
Do not hold locks across .await.
Do not add unbounded queues.
Do not persist secret/private state in amnesia mode.
```

Current safest beta posture:

```text
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
RON_STORAGE_WALLET_BASE_URL=http://<svc-wallet>
RON_STORAGE_WALLET_BEARER=<capability/dev token>
RON_STORAGE_PAID_SETTLEMENT_PAYEE=svc_storage
```

Current safest production-off posture:

```text
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=disabled
RON_STORAGE_PAID_SETTLEMENT_MODE=disabled
```

---

# 17. Short Commit Summary

Suggested commit message:

```text
svc-storage: add wallet-backed paid write settlement

- add wallet receipt context binding for paid storage CID replay protection
- add wallet-capture settlement mode for /paid/o
- capture actual storage cost through svc-wallet
- release unused hold remainder through svc-wallet
- return settlement receipts in paid write response
- add roc_b3_tool helper for CID/context-idem smoke scripts
- add paid_write_settlement route tests
- update live paid-storage smoke to prove issue→hold→store→capture→release
- preserve dev-header, wallet-receipt, and disabled verifier modes
```

Suggested longer body:

```text
This completes the beta-shaped paid-storage lifecycle for RustyOnions ROC:
wallet issue, hold, receipt lookup, CID-bound paid storage write, wallet capture,
wallet release, object readback, balance conservation, and replay rejection.

svc-storage remains within its boundary: it stores bytes and calls svc-wallet
for economic mutation. It does not directly mutate ledger state.
```

---

# 18. Current Status in One Paragraph

`svc-storage` is now beta-strong for the paid storage use case. It can store and serve BLAKE3-addressed objects, enforce paid writes through `/paid/o`, verify wallet hold receipts over HTTP, bind a hold to the exact storage operation and CID, optionally call `svc-wallet` to capture actual cost and release the unused remainder, return stable receipts, emit accounting-compatible usage events, and prove the full live loop with `scripts/web3_paid_storage_live_smoke.sh`. The remaining work is mostly production hardening: real accounting export, metrics expansion, settlement retry/quarantine, capability/passport integration, durable/amnesia storage posture, and upstream WEB3_2 asset-page integration.

---


```
```


### END NOTE - APRIL 28 2026 - 19:30 CST



### BEGIN NOTE - APRIL 28 2026 - 20:10 CST


```text
crates/svc-storage/NOTES.MD
```

````md
---

### BEGIN NOTE - APRIL 28/29 2026 - WEB3 PAID STORAGE + SETTLEMENT + REAL ACCOUNTING EXPORT GREEN

# svc-storage NOTES.MD — WEB3 Paid Storage, Wallet Settlement, and Accounting Export Plateau

Date: 2026-04-28 / 2026-04-29  
Crate: `svc-storage`  
Workspace path: `crates/svc-storage`  
Project: RustyOnions / RON-CORE / WEB3 / ROC internal value plane  
Status: **Paid storage WEB3 loop is live-smoke green with wallet settlement and real ron-accounting export**  

Current estimated completion for this crate:

```text
svc-storage core CAS baseline:                   ~90–94%
svc-storage paid-write admission path:           ~98–99%
svc-storage wallet settlement path:              ~97–99%
svc-storage accounting export path:              ~92–96%
svc-storage WEB3 beta readiness overall:         ~98–99%
svc-storage production-hardening readiness:      ~88–93%
````

Current project-level significance:

```text
svc-storage is now the first RustyOnions service proving a full real ROC paid-use loop:

wallet issue
→ wallet hold
→ paid service enforcement
→ content-addressed storage
→ wallet capture
→ wallet release
→ accounting usage export
→ accounting snapshot
→ object readback
→ replay rejection
```

This note documents the current `svc-storage` plateau, how to run the new flows, what has been proven, and what remains.

---

# 0. Executive Summary

`svc-storage` has reached a major WEB3 beta milestone.

It now supports a live, multi-process internal ROC paid-storage circuit:

```text
ron-accounting ingest process starts
svc-wallet starts
svc-storage starts

svc-wallet issues ROC to payer
svc-wallet creates a hold into escrow
svc-storage verifies the wallet hold receipt over HTTP
svc-storage binds the hold to the exact paid-storage context and body-derived b3 CID
svc-storage writes the object by canonical b3:<64 lowercase hex>
svc-storage calls svc-wallet capture for actual storage cost
svc-storage calls svc-wallet release for unused hold remainder
svc-storage exports usage events to a real ron-accounting ingest process
ron-accounting records usage snapshot rows
svc-storage serves the object back by CID
wallet balances prove conservation
receipt replay for different bytes is rejected
```

The latest green smoke proved:

```text
issue amount:    100 ROC
hold amount:      70 ROC
object size:      48 bytes
capture amount:   48 ROC
release amount:   22 ROC

final balances:
  acct_user           = 52
  escrow_paid_write   = 0
  svc_storage         = 48

accounting snapshot:
  row_count = 3
  bytes PUT /paid/o = 48
  request OK /paid/o = 1
  pin seconds /paid/o = 60
```

This proves the current target loop:

```text
wallet/ledger receipts
→ storage enforcement
→ storage settlement
→ accounting usage metering
```

`svc-storage` remains within its correct boundary:

```text
It stores bytes.
It enforces paid storage admission.
It calls svc-wallet for wallet/ledger mutation.
It exports usage to ron-accounting.
It does not mutate the ledger directly.
It does not become a wallet.
It does not become accounting truth.
It does not resolve names or own asset manifests.
```

---

# 1. Strategic Role of `svc-storage`

`svc-storage` is the RustyOnions content-addressed byte storage service.

Its WEB3 role is now:

```text
store immutable object bytes by BLAKE3 CID
return canonical b3:<64 lowercase hex> content IDs
serve objects by CID
enforce paid storage writes through /paid/o
validate wallet hold receipts
bind holds to exact storage context and CID
settle paid writes through wallet capture/release
emit and optionally export accounting usage events
surface paid-write and accounting-export metrics
support live paid storage smoke proofs
```

Its non-role remains equally important:

```text
svc-storage is not ron-ledger
svc-storage is not svc-wallet
svc-storage is not ron-accounting
svc-storage is not svc-rewarder
svc-storage is not ron-naming
svc-storage is not svc-index
svc-storage is not omnigate
svc-storage is not the browser extension
svc-storage is not a b3 asset-page ownership registry
svc-storage is not a ROX/Solana bridge
```

Correct architecture boundary:

```text
svc-storage:
  raw bytes
  BLAKE3/CID verification
  paid storage admission
  wallet receipt verification
  wallet settlement calls
  usage event creation/export
  storage-specific metrics

svc-wallet:
  balance mutation
  issue/transfer/burn/hold/capture/release
  nonce/idempotency
  committed receipts
  ledger front-door

ron-ledger:
  durable append-only economic truth
  deterministic replay
  conservation/non-negativity

ron-accounting:
  transient usage counters
  usage snapshot rows
  reward projection inputs
  not balance truth

svc-rewarder:
  deterministic reward planning
  reward/payout intents
  no direct ledger mutation

svc-index / ron-naming / gateway / omnigate:
  names
  manifests
  b3 asset pages
  owner/payout metadata
  site/app access
```

---

# 2. Current Feature Inventory

## 2.1 Core CAS storage

`svc-storage` continues to support the baseline object storage behavior:

```text
object write
object read
HEAD metadata
range read behavior
BLAKE3 content addressing
b3:<64 lowercase hex> canonical CIDs
memory storage backend for tests/dev
```

Important rule:

```text
CID = b3:<BLAKE3-256 digest of raw object bytes>
```

Storage should continue to store **bytes by content ID only**.

It should not own:

```text
name resolution
ownership metadata
payout metadata
curator metadata
asset descriptions/tags
crab:// route semantics
human-readable names
```

Those belong upstream in `ron-naming`, `svc-index`, `svc-gateway`, `omnigate`, and WEB3_2 product-layer work.

---

## 2.2 Paid storage route: `/paid/o`

`/paid/o` is the beta paid-write enforcement seam.

Current behavior:

```text
POST /paid/o
hashes request body
computes canonical b3 CID
verifies paid proof
optionally verifies proof against svc-wallet receipt over HTTP
optionally binds wallet receipt idem to body CID/context
stores object
optionally captures actual cost through wallet
optionally releases remainder through wallet
optionally exports usage to accounting
returns object CID, receipt metadata, settlement info, accounting export info, and usage events
```

Current success response shape includes:

```text
cid
paid
payer
escrow
wallet_txid
wallet_receipt_hash
wallet_idem
paid_context_idem
verifier
estimate_minor
settlement
accounting_export
usage_events
```

Example success fields from latest green live smoke:

```json
{
  "paid": true,
  "verifier": "wallet-http",
  "estimate_minor": "70",
  "settlement": {
    "mode": "wallet-capture",
    "capture_amount_minor": "48",
    "release_amount_minor": "22"
  },
  "accounting_export": {
    "mode": "http",
    "status": "exported",
    "event_count": 3,
    "http_status": 202
  }
}
```

---

# 3. Paid Write Verification

## 3.1 Verifier modes

Current paid verifier modes:

```text
dev-header
wallet-receipt
disabled
```

Controlled by:

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=dev-header
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=disabled
```

### `dev-header`

Purpose:

```text
local development
route-level tests
simple manual testing without svc-wallet
```

Behavior:

```text
trusts paid proof headers
validates header shape
does not call svc-wallet
does not require wallet-backed context binding
```

Warning:

```text
dev-header is not production proof.
Keep it for local/test ergonomics.
Do not accidentally deploy production paid storage in dev-header mode.
```

### `wallet-receipt`

Purpose:

```text
production-shaped beta paid-write mode
real svc-wallet receipt lookup
fail-closed proof validation
```

Behavior:

```text
reads expected proof metadata from request headers
calls svc-wallet GET /v1/tx/{txid}
validates actual wallet receipt against expected request fields
requires op=hold
requires asset=roc
requires positive integer amount
requires payer/from
requires escrow/to
requires b3 receipt hash
requires context idem for paid storage binding
rejects mismatches
```

The latest live smoke used:

```text
"verifier": "wallet-http"
```

This proves storage did not merely trust headers; it fetched and verified wallet receipt data over HTTP.

### `disabled`

Purpose:

```text
fail-closed production/off switch
```

Behavior:

```text
rejects paid writes
does not store object
```

---

## 3.2 Paid proof headers

Current `/paid/o` headers:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <integer>
x-ron-wallet-txid: <wallet txid>
x-ron-wallet-receipt-hash: b3:<64 lowercase hex>
x-ron-wallet-from: <payer account>
x-ron-wallet-to: <escrow account>
x-ron-wallet-idem: <storage_put context idem>
```

Optional accounting context headers:

```text
x-ron-tenant: <integer>
x-ron-accounting-subject: <subject/service/provider>
x-ron-region: <region>
x-ron-pin-seconds: <integer>
```

Default accounting context if omitted:

```text
tenant = 1
subject = svc_storage
region = local
pin_seconds = 0
```

---

# 4. Paid Storage Context Binding

A major anti-replay hardening feature is now implemented and green.

For wallet-backed paid writes, `svc-storage` computes a deterministic idempotency/context key from:

```text
version
service = svc-storage
route = /paid/o
operation = paid_storage_put
cid = b3:<BLAKE3 hash of body>
payer
escrow
asset
amount_minor
```

The resulting key has this shape:

```text
storage_put:<32 lowercase hex chars>
```

The wallet hold must have this exact idempotency key.

Latest green smoke example:

```text
object CID:
  b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57

context idem:
  storage_put:c94182f22c2d8cfb5ae73d930c611cf7
```

The paid write returned:

```text
wallet_idem       = storage_put:c94182f22c2d8cfb5ae73d930c611cf7
paid_context_idem = storage_put:c94182f22c2d8cfb5ae73d930c611cf7
```

Replay with different bytes was rejected:

```text
payment_required:
wallet receipt paid storage context idem mismatch
```

This proves:

```text
A valid hold for one exact object/CID cannot be reused to store different bytes.
```

---

# 5. Wallet Settlement

## 5.1 Settlement modes

Current modes:

```text
disabled
wallet-capture
```

Controlled by:

```bash
RON_STORAGE_PAID_SETTLEMENT_MODE=disabled
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
```

Default:

```text
disabled
```

Reason:

```text
Settlement mutates wallet/ledger through svc-wallet, so it must be explicit.
```

---

## 5.2 `wallet-capture` lifecycle

When enabled:

```text
1. Verify wallet hold receipt.
2. Bind hold to CID/context.
3. Store object bytes.
4. Calculate actual cost.
5. POST /v1/capture to svc-wallet:
     escrow_paid_write → svc_storage
6. POST /v1/release to svc-wallet:
     escrow_paid_write → payer
7. Return capture and release receipts.
```

Current beta pricing model:

```text
actual_capture_amount_minor = max(bytes_stored, 1)
```

Latest smoke example:

```text
object bytes = 48
hold amount = 70
capture = 48
release = 22
```

---

## 5.3 Settlement response

Returned under:

```text
settlement
```

Shape:

```json
{
  "mode": "wallet-capture",
  "capture_amount_minor": "48",
  "release_amount_minor": "22",
  "capture_receipt": {
    "op": "capture",
    "from": "escrow_paid_write",
    "to": "svc_storage",
    "asset": "roc",
    "amount_minor": "48",
    "idem": "storage_cap:<hash>",
    "receipt_hash": "b3:<hash>"
  },
  "release_receipt": {
    "op": "release",
    "from": "escrow_paid_write",
    "to": "acct_user",
    "asset": "roc",
    "amount_minor": "22",
    "idem": "storage_rel:<hash>",
    "receipt_hash": "b3:<hash>"
  }
}
```

Settlement idempotency key shapes:

```text
capture:
  storage_cap:<32 lowercase hex>

release:
  storage_rel:<32 lowercase hex>

failed write release:
  storage_failrel:<32 lowercase hex>
```

---

## 5.4 Balance conservation proof

Latest smoke final balances:

```text
payer balance:  52
escrow balance: 0
payee balance:  48
```

Calculation:

```text
payer:
  100 issued
  -70 held
  +22 released
  = 52

escrow:
  +70 held
  -48 captured
  -22 released
  = 0

payee:
  +48 captured
  = 48
```

This proves settlement conservation through `svc-wallet`.

---

# 6. Accounting Export

`svc-storage` now has an opt-in accounting export path.

## 6.1 Export modes

Current modes:

```text
disabled
http
```

Controlled by:

```bash
RON_STORAGE_ACCOUNTING_EXPORT_MODE=disabled
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
```

Default:

```text
disabled
```

Reason:

```text
Accounting export should not silently become a production dependency unless explicitly configured.
```

---

## 6.2 HTTP export config

Environment variables:

```bash
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
RON_STORAGE_ACCOUNTING_BASE_URL=http://127.0.0.1:19600
RON_STORAGE_ACCOUNTING_BEARER=dev
RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000
```

Export endpoint:

```text
POST /v1/usage-events
```

Export schema:

```text
svc-storage.usage-events.v1
```

Export payload includes:

```text
schema
cid
wallet_txid
source_service
events[]
```

Export uses deterministic idempotency keys:

```text
storage_acct:<32 lowercase hex>
```

Latest green smoke example:

```json
"accounting_export": {
  "mode": "http",
  "status": "exported",
  "event_count": 3,
  "idempotency_key": "storage_acct:5b5e91f3af58126c7cd9a24a66e31d62",
  "http_status": 202,
  "reason": null
}
```

---

## 6.3 Usage event DTO

`svc-storage` usage events currently include:

```text
timestamp_ms
tenant
subject
metric_kind
value
source_service
region
route
```

Current metric kinds emitted by paid storage:

```text
bytes_stored
request_ok
pin_seconds
```

Example event payload:

```json
{
  "timestamp_ms": 1777424552047,
  "tenant": 7,
  "subject": "svc_storage_provider",
  "metric_kind": "bytes_stored",
  "value": 48,
  "source_service": "svc-storage",
  "region": "local",
  "route": "/paid/o"
}
```

---

## 6.4 Export behavior

Important behavior:

```text
successful export:
  accounting_export.status = exported
  http_status = 202

disabled export:
  accounting_export.status = skipped

failed export:
  accounting_export.status = failed
  paid storage write still succeeds if storage and wallet settlement succeeded
```

Design reason:

```text
ron-accounting is usage/metering, not ledger truth.
A transient accounting export failure should be visible and recoverable,
but should not directly undo a successful wallet-settled storage write.
```

Remaining hardening:

```text
persistent export retry queue
dead-letter/quarantine
admin view of failed export reports
manual reconciliation
batch exporter
dedupe across restarts
```

---

# 7. Real `ron-accounting` Ingest Process

A lightweight std-only `ron-accounting` HTTP ingest binary was added so the live smoke can prove real process-to-process storage → accounting export without adding new dependencies.

Added binary:

```text
crates/ron-accounting/src/bin/ron_accounting_ingest.rs
```

Added module:

```text
crates/ron-accounting/src/http_ingest.rs
```

Exposed through:

```text
crates/ron-accounting/src/lib.rs
```

Supported endpoints:

```text
GET  /healthz
GET  /readyz
GET  /v1/snapshot
POST /v1/usage-events
```

Default bind:

```text
127.0.0.1:19600
```

Environment:

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600
RON_ACC_BEARER=dev
```

Purpose:

```text
live beta proof
local integration testing
storage→accounting export validation
```

Important note:

```text
This is a lightweight ingest adapter and smoke-proof service.
It is good enough for current WEB3 beta validation, but may later be replaced or absorbed into the canonical ron-accounting service/admin plane.
```

---

# 8. Accounting Snapshot Proof

The latest live smoke calls:

```text
GET /v1/snapshot
```

Returned:

```json
{
  "schema": "ron-accounting.snapshot.v1",
  "row_count": 3,
  "rows": [
    {
      "key": {
        "dimension": "requests",
        "labels": {
          "method": "PIN_SECONDS",
          "region": "local",
          "route": "/paid/o",
          "service": "svc_storage_provider",
          "tenant": 7
        }
      },
      "value": 60
    },
    {
      "key": {
        "dimension": "bytes",
        "labels": {
          "method": "PUT",
          "region": "local",
          "route": "/paid/o",
          "service": "svc_storage_provider",
          "tenant": 7
        }
      },
      "value": 48
    },
    {
      "key": {
        "dimension": "requests",
        "labels": {
          "method": "REQ_OK",
          "region": "local",
          "route": "/paid/o",
          "service": "svc_storage_provider",
          "tenant": 7
        }
      },
      "value": 1
    }
  ]
}
```

This proves:

```text
svc-storage usage events reached a real ron-accounting process
ron-accounting accepted the HTTP export
ron-accounting recorded the rows
snapshot inspection confirms the expected counters
```

---

# 9. Metrics

`svc-storage` metrics now include paid-write and accounting-export metric families.

Important metrics:

```text
storage_paid_write_total{status}
storage_paid_write_bytes_total
storage_accounting_export_total{status}
storage_accounting_export_events_total
```

Paid write statuses include:

```text
accepted
payment_required
bad_accounting_context
storage_error
disabled
config_error
settlement_error
```

Accounting export statuses include:

```text
skipped
exported
failed
```

The metrics route keeps compatibility with the existing initializer:

```text
register_paid_write_metrics()
```

This initializes both old paid-write and new accounting-export metric families so `/metrics` has expected series before traffic.

Important metric guardrail:

```text
Do not put account IDs, CIDs, txids, receipt hashes, or object metadata into Prometheus labels.
```

Labels should stay low-cardinality machine statuses only.

---

# 10. Helper Binary

Added helper:

```text
crates/svc-storage/src/bin/roc_b3_tool.rs
```

Purpose:

```text
compute b3 CID without requiring external b3sum
compute paid-storage context idem using the exact same code path as svc-storage
```

Commands:

```bash
cargo run -q -p svc-storage --bin roc_b3_tool -- cid <path>

cargo run -q -p svc-storage --bin roc_b3_tool -- context-idem \
  <cid> <payer> <escrow> <asset> <amount_minor>
```

Used by:

```text
scripts/web3_paid_storage_live_smoke.sh
```

---

# 11. Files Added / Updated

## Added during this plateau

```text
crates/svc-storage/src/bin/roc_b3_tool.rs
crates/svc-storage/src/accounting/mod.rs
crates/svc-storage/src/accounting/exporter.rs
crates/svc-storage/src/policy/settlement.rs
crates/svc-storage/tests/paid_write_settlement.rs
crates/svc-storage/tests/paid_write_accounting_export.rs
crates/ron-accounting/src/http_ingest.rs
crates/ron-accounting/src/bin/ron_accounting_ingest.rs
scripts/web3_paid_storage_live_smoke.sh
```

## Updated during this plateau

```text
crates/svc-storage/src/lib.rs
crates/svc-storage/src/config.rs
crates/svc-storage/src/metrics.rs
crates/svc-storage/src/policy/mod.rs
crates/svc-storage/src/policy/paid_write.rs
crates/svc-storage/src/http/routes/paid_object.rs
crates/svc-storage/tests/paid_write_wallet_mode.rs
crates/ron-accounting/src/lib.rs
```

---

# 12. Test Inventory

## `http_blackbox.rs`

Proves:

```text
basic storage write/read behavior
HEAD object checks
GET object bytes
range read behavior
```

## `paid_write_policy.rs`

Proves:

```text
missing paid proof rejects
malformed proof rejects
malformed accounting context rejects
valid minimal proof stores object
paid-write metrics expose statuses
failed admission does not store object
```

## `paid_write_verifier.rs`

Proves:

```text
WalletReceipt validates hold/roc/positive amount
bad receipt hash rejects
wrong operation rejects
wrong asset rejects
zero or non-integer amount rejects
missing payer/escrow rejects
DevHeaderVerifier accepts valid proof
DevHeaderVerifier rejects invalid proof
WalletReceiptVerifier fails closed until lookup wired
WalletReceiptVerifier accepts matching lookup receipt
WalletReceiptVerifier rejects wrong payer/escrow/asset/amount/hash/op
```

## `paid_write_http_clients.rs`

Proves:

```text
HTTP wallet receipt client fetches with bearer
matching HTTP wallet receipt verifies
bad JSON fails closed
missing receipt fails closed
missing bearer fails closed
unsafe txid rejected before network
invalid base URL rejected
```

## `paid_write_wallet_mode.rs`

Proves:

```text
/payed/o in wallet-receipt mode calls wallet
matching wallet receipt stores object
mismatched wallet receipt rejects
same valid hold cannot be reused for different body/CID
usage_events include bytes_stored/request_ok/pin_seconds
```

Typo note:

```text
The actual route is /paid/o, not /payed/o.
```

## `paid_write_settlement.rs`

Proves:

```text
/payed/o in wallet-receipt + wallet-capture mode works
capture actual cost through wallet
release remainder through wallet
settlement receipts are returned
object reads back after settlement
```

Typo note:

```text
The actual route is /paid/o, not /payed/o.
```

## `paid_write_accounting_export.rs`

Proves:

```text
/payed/o can export usage events to HTTP accounting adapter
export request has schema svc-storage.usage-events.v1
export uses Idempotency-Key storage_acct:<hash>
export sends wallet_txid, source_service, CID, and events
accounting_export report says exported
event shape includes bytes_stored/request_ok/pin_seconds
```

Typo note:

```text
The actual route is /paid/o, not /payed/o.
```

## `web3_paid_storage_loop.rs`

Proves in-process WEB3 logic:

```text
unfunded paid storage rejects
wallet issue + hold enables paid storage
usage event shape remains accounting-compatible
wallet capture/release conservation works in local model
```

## Live smoke

Script:

```text
scripts/web3_paid_storage_live_smoke.sh
```

Proves full multi-process path:

```text
ron-accounting process
svc-wallet process
svc-storage process
wallet issue
wallet hold
storage wallet receipt lookup
storage CID/context binding
storage object write
wallet capture
wallet release
storage accounting export
ron-accounting snapshot rows
object readback
wallet balance conservation
replay rejection
```

---

# 13. Commands

## Full ron-accounting gate

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets
```

Latest known result:

```text
clippy clean
all ron-accounting tests passed
ron_accounting_ingest binary compiled
```

## Full svc-storage gate

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

Latest known result:

```text
clippy clean
all svc-storage tests passed
```

## Focused svc-storage tests

```bash
cargo test -p svc-storage --test paid_write_policy
cargo test -p svc-storage --test paid_write_verifier
cargo test -p svc-storage --test paid_write_http_clients
cargo test -p svc-storage --test paid_write_wallet_mode
cargo test -p svc-storage --test paid_write_settlement
cargo test -p svc-storage --test paid_write_accounting_export
cargo test -p svc-storage --test web3_paid_storage_loop
```

## Live three-process WEB3 smoke

```bash
chmod +x scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_storage_live_smoke.sh
```

Expected final line:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```

---

# 14. Live Smoke Service Ports

Defaults:

```text
ron-accounting:
  127.0.0.1:19600

svc-wallet:
  127.0.0.1:18088

svc-storage:
  127.0.0.1:15303
```

Override example:

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600 \
SVC_WALLET_ADDR=127.0.0.1:18088 \
ADDR=127.0.0.1:15303 \
bash scripts/web3_paid_storage_live_smoke.sh
```

Logs:

```text
target/web3-smoke/ron-accounting.log
target/web3-smoke/svc-wallet.log
target/web3-smoke/svc-storage.log
```

Generated smoke files:

```text
target/web3-smoke/paid-object.bin
target/web3-smoke/paid-object.get.bin
target/web3-smoke/bad-object.bin
target/web3-smoke/bad-response.json
```

---

# 15. Environment Variables

## svc-storage paid write

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=dev-header
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=disabled
```

Recommended beta smoke:

```bash
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
```

## svc-storage wallet config

```bash
RON_STORAGE_WALLET_BASE_URL=http://127.0.0.1:18088
RON_STORAGE_WALLET_BEARER=dev
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS=2000
```

## svc-storage settlement

```bash
RON_STORAGE_PAID_SETTLEMENT_MODE=disabled
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
RON_STORAGE_PAID_SETTLEMENT_PAYEE=svc_storage
```

Recommended beta smoke:

```bash
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
RON_STORAGE_PAID_SETTLEMENT_PAYEE=svc_storage
```

## svc-storage accounting export

```bash
RON_STORAGE_ACCOUNTING_EXPORT_MODE=disabled
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
RON_STORAGE_ACCOUNTING_BASE_URL=http://127.0.0.1:19600
RON_STORAGE_ACCOUNTING_BEARER=dev
RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000
```

Recommended beta smoke:

```bash
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
```

## ron-accounting ingest

```bash
RON_ACCOUNTING_ADDR=127.0.0.1:19600
RON_ACC_BEARER=dev
```

## svc-storage bind

```bash
ADDR=127.0.0.1:15303
```

Note:

```text
The live script currently uses ADDR for svc-storage bind because that matches current binary behavior.
```

---

# 16. Latest Green Proof Summary

Latest validated command group:

```bash
cargo fmt
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo test -p ron-accounting --all-targets

cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets

chmod +x scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_storage_live_smoke.sh
```

Latest green proofs:

```text
ron-accounting clippy passed
ron-accounting tests passed
svc-storage clippy passed
svc-storage tests passed
ron-accounting ingest started and healthz passed
svc-wallet started and healthz passed
svc-storage started and healthz passed
wallet issue succeeded
wallet hold succeeded
paid write succeeded
capture receipt returned
release receipt returned
accounting_export.status = exported
accounting_export.http_status = 202
accounting_export.event_count = 3
object readback succeeded
wallet balances conserved
ron-accounting snapshot row_count = 3
replay with different bytes rejected
```

---

# 17. Current Invariants Proven

## 17.1 Paid write fail-closed

Proven:

```text
missing proof does not store
bad proof does not store
wallet mismatch does not store
context mismatch does not store
disabled mode rejects
```

## 17.2 Wallet-only mutation

Proven:

```text
storage calls wallet for capture/release
storage does not mutate ledger directly
wallet receipts include ledger sequence/root fields
wallet balance conservation holds
```

## 17.3 CID/context binding

Proven:

```text
body determines b3 CID
wallet hold idem includes CID
different body computes different expected context idem
replay with old hold rejects
```

## 17.4 Settlement conservation

Proven:

```text
capture amount equals actual cost
release amount equals held remainder
escrow returns to 0
payer and payee balances match expected math
```

## 17.5 Accounting export

Proven:

```text
usage_events are emitted
usage_events export to HTTP accounting endpoint
Idempotency-Key is deterministic storage_acct:<hash>
accounting process accepts export with 202
accounting snapshot records expected rows
```

---

# 18. Known Limitations / Remaining Work

`svc-storage` is extremely strong for WEB3 beta now, but not fully production hardened.

## 18.1 Accounting export durability

Current:

```text
single HTTP export attempt
response reports exported/skipped/failed
successful paid write not rolled back on export failure
```

Needed:

```text
retry queue
WAL-backed export outbox
dedupe across restarts
dead-letter/quarantine
manual reconciliation
admin view of failed exports
```

## 18.2 Settlement failure recovery

Current:

```text
failed storage write attempts release
settlement failure returns settlement_failed
```

Needed:

```text
persistent settlement recovery event
retry/reconciliation worker
admin status for unresolved settlement failure
separate handling for:
  capture failed
  capture succeeded but release failed
  wallet timeout
  wallet conflict/idempotency replay
```

## 18.3 Pricing config

Current:

```text
actual_capture_amount_minor = max(bytes_stored, 1)
```

Needed:

```text
configs/roc-economics.toml
price per byte
price per pin-second
minimum charge
provider split
creator split
reserve split
basis points only
integer minor units only
no floating-point math
```

## 18.4 Canonical accounting service

Current:

```text
std-only ron_accounting_ingest binary
good for smoke/live proof
not necessarily final service architecture
```

Needed:

```text
canonical ron-accounting service/admin plane
/healthz /readyz /metrics
possibly axum-based when Cargo context is confirmed
durable WAL/in-memory modes
batch ingest
service metrics
```

## 18.5 Auth/capability integration

Current:

```text
dev bearer tokens
header-based paid proof
wallet receipt lookup
```

Needed:

```text
ron-auth integration
svc-passport capabilities
wallet caveats:
  account
  asset
  amount
  action
  route
  CID
  expiry
storage capability:
  storage:put
short-lived runtime capabilities
```

## 18.6 Metrics hardening

Current:

```text
paid write metrics
accounting export metrics
```

Needed:

```text
wallet receipt lookup metrics
settlement capture/release metrics
settlement error metrics
accounting export retry metrics
storage object count/bytes metrics
latency histograms
readyz degraded conditions
```

## 18.7 Gateway/omnigate paid path

Current:

```text
svc-storage direct /paid/o works
```

Needed:

```text
svc-gateway paid storage route
omnigate policy/enforcement path
passport/capability integration
paid content view / site access extensions
```

## 18.8 b3 asset page layer

Current:

```text
raw byte storage by b3 CID
```

Needed upstream:

```text
manifest-backed b3 asset pages
crab://b3/<hash>.image
crab://b3/<hash>.song
crab://b3/<hash>.article
ownership
payout address
tags
description
curator metadata
provenance
storage/provider data
receipts
```

Do not add this semantic ownership layer inside `svc-storage`.

---

# 19. Recommended Next Work

## Priority 1 — Rewarder integration proof

Now that storage exports usage into real accounting, the next best project-level proof is:

```text
ron-accounting snapshot
→ svc-rewarder deterministic reward plan
→ svc-wallet issue/payout
→ ledger receipt
```

This would close:

```text
usage → accounting → rewarder → wallet/ledger
```

That is the next big WEB3 loop.

## Priority 2 — Make accounting ingest canonical

The std-only ingest adapter is a great live proof. Next production shape should decide:

```text
keep it as minimal HTTP binary
or replace with canonical axum service/admin plane
or expose through existing ron-accounting service scaffold
```

Acceptance for canonical version:

```text
/healthz
/readyz
/metrics
/v1/usage-events
/v1/snapshot
idempotency dedupe
bounded request body
auth gate
clear error taxonomy
tests
live smoke still green
```

## Priority 3 — Outbox/retry for accounting export

Add to `svc-storage`:

```text
accounting export outbox
retry/backoff
dead-letter
admin/debug status
```

This makes accounting export production-safe under transient outages.

## Priority 4 — Settlement recovery hardening

Add to `svc-storage`:

```text
settlement reconciliation events
failed capture/release retry
idempotent replay
manual reconcile test
```

## Priority 5 — Gateway/omnigate integration

Expose the proven storage paid-write path through:

```text
svc-gateway
omnigate
future passport capability UX
```

---

# 20. Suggested Commit Summary

```text
svc-storage: complete paid storage settlement and accounting export loop

- add wallet-backed paid-write receipt verification and context binding
- bind wallet holds to exact b3 CID and paid-storage operation context
- add wallet-capture settlement mode for /paid/o
- capture actual storage cost through svc-wallet
- release unused hold remainder through svc-wallet
- return capture/release receipts in paid write response
- add usage event DTO and optional HTTP accounting exporter
- add accounting_export report to paid write response
- add metrics for accounting export outcomes
- add roc_b3_tool helper for CID/context-idem smoke scripts
- add paid_write_settlement and paid_write_accounting_export tests
- update live WEB3 smoke to run ron-accounting + svc-wallet + svc-storage
- prove issue→hold→store→capture→release→accounting snapshot
```

Longer commit body:

```text
This completes the first live RustyOnions ROC paid-use loop for storage.

The smoke now proves:
svc-wallet issue/hold, svc-storage wallet receipt verification,
CID-bound paid write, wallet capture/release settlement, real
ron-accounting usage export, accounting snapshot rows, object readback,
wallet balance conservation, and replay rejection for a different body.

svc-storage remains within its boundary: it stores bytes, enforces
paid-write admission, calls svc-wallet for economic mutation, and exports
usage to accounting. It does not directly mutate ledger state.
```

---

# 21. Current Status in One Paragraph

`svc-storage` is now beta-complete for the core paid-storage WEB3 use case. It stores and serves BLAKE3-addressed objects, enforces paid writes through `/paid/o`, verifies wallet hold receipts over HTTP, binds paid holds to the exact object CID/context, settles successful writes through `svc-wallet` capture/release, returns settlement receipts, exports usage events to a real `ron-accounting` ingest process, verifies accounting snapshot rows, preserves balance conservation, and rejects replay of the same hold for different bytes. Remaining work is production hardening: durable accounting outbox/retry, settlement reconciliation, canonical accounting service shape, richer metrics/readiness, gateway/omnigate integration, passport/capability integration, and upstream b3 asset-page/product-layer work.

---
```

### END NOTE - APRIL 28 2026 - 20:10 CST



### BEGIN NOTE - APRIL 28 2026 - 23:10 CST

# svc-storage NOTES.MD — WEB3 Paid Storage, Wallet Settlement, Accounting Export, and ROC Economics Carryover Notes

Date: 2026-04-29
Crate: `svc-storage`
Workspace path: `crates/svc-storage`
Project: RustyOnions / RON-CORE / WEB3 / ROC internal value plane
Session status: **Paid storage is live-smoke green with wallet settlement, real accounting export, and optional ROC economics pricing**
Estimated current completion: **98–99% for WEB3 beta paid-storage path**, **90–94% for production-hardening readiness**

---

## 0. Executive Summary

`svc-storage` is now the strongest concrete WEB3 use-case crate in RustyOnions. It proves the first real in-network paid ROC operation: paying ROC to store content by canonical BLAKE3 content ID.

Before this session, `svc-storage` had already reached a major plateau: free CAS writes/reads, HEAD/range reads, paid `/paid/o` writes, wallet hold receipt validation, context binding, wallet-capture settlement, accounting usage export, and live multi-service smoke with `svc-wallet` and `ron-accounting`. Prior notes summarized the paid storage loop as:

```text
wallet issue
→ wallet hold
→ paid service enforcement
→ content-addressed storage
→ wallet capture
→ wallet release
→ accounting usage export
→ accounting snapshot
→ object readback
→ replay rejection
```

That plateau was already documented as a full real ROC paid-use loop. 

This session added the next important improvement: **`svc-storage` now has the first live consumer path for the new `configs/roc-economics.toml` policy**. The old beta price model, `capture_amount_minor = max(bytes_stored, 1)`, remains the fallback when no economics config is explicitly set. When `RON_STORAGE_ROC_ECONOMICS_PATH` is set, storage loads the validated economics policy through `ron-policy` and prices the `paid_storage_put` action from that config.

Final observed gate was fully green:

```text
cargo fmt                                      ✅
cargo clippy -p svc-storage --all-targets     ✅
cargo test -p svc-storage --test paid_write_economics ✅
cargo test -p svc-storage --all-targets       ✅
scripts/web3_paid_storage_live_smoke.sh       ✅
```

The final terminal output confirms the new `paid_write_economics` tests passed 6/6, the full `svc-storage` test suite passed, and the live smoke ended with:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```



---

## 1. Strategic Role in WEB3 / ROC

`svc-storage` is the raw content-addressed storage service and the first paid-use enforcement point in the internal ROC economy.

Its correct role:

```text
store immutable bytes by canonical b3 CID
serve objects by CID
support free/dev CAS writes and reads
support HEAD metadata checks
support range reads
enforce paid object writes through /paid/o
verify wallet hold receipts
bind paid holds to exact storage context and body-derived CID
call svc-wallet for capture/release when settlement is enabled
export usage events to ron-accounting
emit low-cardinality paid-write/accounting metrics
optionally price paid_storage_put from ron-policy economics config
```

Its incorrect role — do not regress into this:

```text
not the ledger
not the wallet
not the accounting truth engine
not the reward calculator
not the name resolver
not the passport authority
not the asset manifest owner registry
not the b3 asset page layer
not the external-chain bridge
not the ROX/Solana layer
```

Correct service boundaries remain:

```text
svc-storage:
  bytes, CIDs, paid-write admission, settlement calls, usage events

svc-wallet:
  issue, hold, capture, release, balances, receipts, idempotency, ledger commit

ron-ledger:
  durable append-only economic truth

ron-accounting:
  transient usage counters and snapshots

svc-rewarder:
  deterministic reward plans and payout issue requests

ron-policy:
  economics/pricing/split/cap policy

svc-index / ron-naming / svc-gateway / omnigate:
  names, manifests, owner/payout metadata, asset pages, crab links, product UX
```

This boundary is especially important for WEB3_2: storage stores bytes by content ID only. Ownership, manifests, crab links, payout routing, b3 asset pages, and semantic asset metadata belong upstream, not inside raw storage.

---

## 2. What Was Already Working Before This Session

Before this session, `svc-storage` already supported:

```text
free/dev CAS object writes
free/dev CAS object reads
HEAD metadata checks
range reads
BLAKE3 b3:<64 lowercase hex> addressing
POST /paid/o paid writes
dev-header verifier mode
wallet-receipt verifier mode
disabled verifier mode
HTTP wallet receipt lookup
wallet hold receipt contract validation
paid storage context binding
wallet-capture settlement mode
capture actual cost through svc-wallet
release unused hold remainder through svc-wallet
return capture/release receipts in /paid/o response
accounting-compatible usage event DTOs
HTTP accounting export mode
paid-write Prometheus metrics
accounting export metrics
roc_b3_tool helper for smoke scripts
live wallet + storage + accounting smoke
replay rejection for mismatched bytes/context
```

Prior notes emphasized that `svc-storage` had become the first fully live paid-use enforcement point for ROC, proving hold → capture → release while never mutating the ledger directly. 

Before this session, the main missing piece was that paid storage pricing still used the hardcoded beta formula:

```text
capture_amount_minor = max(bytes_stored, 1)
```

That was safe and deterministic, but not yet policy-configurable.

---

## 3. What We Accomplished in This Session

### 3.1 Added `ron-policy` as a production dependency

We updated:

```text
crates/svc-storage/Cargo.toml
```

Added under RON crates:

```toml
ron-policy = { path = "../ron-policy" }
```

This was necessary because `svc-storage` now calls `ron-policy` to load and validate `configs/roc-economics.toml`.

Important architectural point:

```text
svc-storage depends on ron-policy for read-only economics policy.
svc-storage still does not mutate wallet/ledger directly.
svc-storage still uses svc-wallet for economic mutation.
```

### 3.2 Added optional ROC economics adapter

New/updated file:

```text
crates/svc-storage/src/policy/economics.rs
```

This module defines the optional pricing adapter for paid storage.

Key env vars:

```text
RON_STORAGE_ROC_ECONOMICS_PATH
RON_STORAGE_ROC_ECONOMICS_ACTION
```

Default action:

```text
paid_storage_put
```

Important behavior:

```text
If RON_STORAGE_ROC_ECONOMICS_PATH is unset:
  preserve legacy beta pricing: max(bytes_stored, 1)

If RON_STORAGE_ROC_ECONOMICS_PATH is set:
  read the TOML file
  parse/validate through ron-policy
  price configured action
  fail closed if file missing, invalid, unknown action, disabled action, or cap exceeded
```

This is the correct beta-safe design because it preserves existing smokes by default while making the economics path explicitly opt-in.

### 3.3 Added paid storage economics tests

New test file:

```text
crates/svc-storage/tests/paid_write_economics.rs
```

Final passing tests:

```text
legacy_pricing_is_preserved_when_policy_env_is_unset
checked_in_roc_economics_prices_paid_storage_put
configured_action_can_be_overridden
explicit_bad_policy_path_fails_closed
explicit_empty_policy_path_fails_closed
explicit_unknown_action_fails_closed
```

Final output:

```text
running 6 tests
6 passed
0 failed
```



What this proves:

```text
legacy fallback remains stable
checked-in configs/roc-economics.toml can price paid_storage_put
48-byte paid_storage_put prices to 84 minor units under economics config
action override works
explicit missing config file fails closed
explicit empty config path fails closed
explicit unknown action fails closed
```

### 3.4 Wired route pricing to the economics adapter

Updated:

```text
crates/svc-storage/src/http/routes/paid_object.rs
```

The route now builds the settlement plan using:

```text
paid_storage_capture_amount_from_env(bytes_stored)
```

instead of directly using the hardcoded legacy amount.

Current behavior:

```text
legacy path:
  no RON_STORAGE_ROC_ECONOMICS_PATH
  capture 48 for 48 bytes

economics path:
  RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml
  paid_storage_put prices 48 bytes as 84 minor units
```

This keeps current live smoke green while enabling the next economics-enabled smoke.

### 3.5 Updated settlement planning for externally computed capture amount

Updated:

```text
crates/svc-storage/src/policy/settlement.rs
```

The settlement plan now supports:

```text
PaidStorageSettlementPlan::from_paid_write(...)
```

for legacy pricing, and:

```text
PaidStorageSettlementPlan::from_paid_write_with_capture_amount(...)
```

for economics-configured pricing.

This lets the paid route calculate the capture amount first, then hand the exact amount into settlement validation.

Settlement still enforces:

```text
capture amount > 0
capture amount <= hold estimate
payee cannot be escrow
paid context idem must match CID/payer/escrow/asset/estimate
release amount = held amount - capture amount
```

### 3.6 Fixed helper visibility issue in settlement module

The first settlement replacement referenced helper functions that existed elsewhere but were private:

```text
normalize_base_url
is_b3_cid
```

Compiler correctly rejected them as inaccessible. 

Fix:

```text
defined local normalize_base_url()
defined local is_b3_cid()
```

After that, `svc-storage` compiled and all tests passed.

### 3.7 Proved all existing behavior remained intact

After the economics wiring, the full `svc-storage` suite still passed:

```text
config tests: 9 passed
http_blackbox: 1 passed
paid_write_accounting_export: 1 passed
paid_write_economics: 6 passed
paid_write_http_clients: 7 passed
paid_write_policy: 5 passed
paid_write_settlement: 1 passed
paid_write_verifier: 19 passed
paid_write_wallet_mode: 1 passed
web3_paid_storage_loop: 1 passed
bench targets compile as test targets
```



This is important because the new economics feature did not regress the existing wallet-receipt, settlement, accounting export, or free CAS behavior.

### 3.8 Proved live paid-storage smoke still green

The old live smoke still uses legacy fallback pricing because `RON_STORAGE_ROC_ECONOMICS_PATH` is unset.

Final live smoke proved:

```text
ron-accounting starts healthy
svc-wallet starts healthy
svc-storage starts healthy
wallet issues 100 ROC
wallet holds 70 ROC
svc-storage verifies wallet hold receipt
svc-storage writes object
svc-storage captures 48
svc-storage releases 22
svc-storage exports 3 accounting events
object reads back
payer balance = 52
escrow balance = 0
payee balance = 48
ron-accounting snapshot row_count = 3
same hold cannot be replayed for different bytes
```

The final output includes the paid object response with capture/release receipts and the accounting export report. 

---

## 4. Current Feature Inventory

### 4.1 Free CAS path

Current capabilities:

```text
PUT/POST object bytes
derive BLAKE3 digest
return canonical b3:<64 lowercase hex> CID
GET object by CID
HEAD metadata
range read behavior
HTTP black-box test coverage
```

Known test:

```text
http_blackbox.rs
```

Final pass:

```text
test blackbox_put_head_get_range ... ok
```

### 4.2 Paid object write path

Endpoint:

```text
POST /paid/o
```

Paid route behavior:

```text
hash body
derive b3 CID
verify paid write proof
validate wallet receipt if in wallet-receipt mode
validate context binding
optionally build wallet settlement client
compute capture amount
store bytes
capture/release if settlement enabled
emit usage events
export accounting if configured
return paid response
```

### 4.3 Paid verifier modes

Current modes:

```text
dev-header
wallet-receipt
disabled
```

Environment:

```text
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=dev-header
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=disabled
```

Expected behavior:

```text
dev-header:
  local/testing mode, explicit only

wallet-receipt:
  production-shaped beta mode, calls wallet receipt lookup

disabled:
  fail closed for paid route
```

### 4.4 Wallet receipt verification

Storage verifies:

```text
op = hold
asset = roc
positive amount
payer/from is present
escrow/to is present
receipt hash is b3:<64 lowercase hex>
txid and receipt hash match headers
lookup receipt matches expected values
context idem matches exact storage context
```

Prior notes emphasize the key proof:

```text
"verifier": "wallet-http"
```

meaning storage is no longer simply trusting dev headers; it calls `svc-wallet` for receipt verification. 

### 4.5 Paid storage context binding

The wallet hold is bound to:

```text
service = svc-storage
route = /paid/o
operation = paid_storage_put
cid = b3:<body hash>
payer
escrow
asset
amount_minor
```

Context idempotency shape:

```text
storage_put:<32 lowercase hex>
```

Replay with different bytes rejects. Final smoke shows:

```text
payment_required:
wallet receipt paid storage context idem mismatch
```

This is a critical anti-replay property.

### 4.6 Wallet settlement

Settlement mode:

```text
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
```

Current settlement flow:

```text
verified hold
→ storage write succeeds
→ capture actual storage cost through svc-wallet
→ release remainder through svc-wallet
→ return capture/release receipts
```

Settlement response shape includes:

```text
mode = wallet-capture
capture_amount_minor
release_amount_minor
capture_receipt
release_receipt
```

Existing notes document the expected receipt shapes and idempotency key formats:

```text
capture: storage_cap:<32 lowercase hex>
release: storage_rel:<32 lowercase hex>
failed write release: storage_failrel:<32 lowercase hex>
```



### 4.7 Accounting export

Current modes:

```text
disabled
http
```

Environment:

```text
RON_STORAGE_ACCOUNTING_EXPORT_MODE=disabled
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
RON_STORAGE_ACCOUNTING_BASE_URL=http://127.0.0.1:19600
RON_STORAGE_ACCOUNTING_BEARER=dev
RON_STORAGE_ACCOUNTING_TIMEOUT_MS=2000
```

Export endpoint:

```text
POST /v1/usage-events
```

Export schema:

```text
svc-storage.usage-events.v1
```

Current usage event kinds:

```text
bytes_stored
request_ok
pin_seconds
```

Export report included in `/paid/o` response:

```json
{
  "mode": "http",
  "status": "exported",
  "event_count": 3,
  "idempotency_key": "storage_acct:<hash>",
  "http_status": 202,
  "reason": null
}
```

Prior notes describe this as the strongest proof that accounting is no longer isolated: storage exports usage into a real `ron-accounting` process and snapshot rows verify the counters. 

### 4.8 Optional ROC economics pricing

New in this session.

Environment:

```text
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml
RON_STORAGE_ROC_ECONOMICS_ACTION=paid_storage_put
```

Current behavior:

```text
unset env:
  legacy capture = max(bytes_stored, 1)

set env:
  load policy through ron-policy
  price configured action
  fail closed on explicit config errors
```

Current tested examples:

```text
legacy 48-byte object:
  capture = 48

economics-configured paid_storage_put for 48 bytes:
  capture = 84
```

This sets up the next smoke script: economics-enabled paid storage.

---

## 5. Current Live Smoke Proof

Script:

```text
scripts/web3_paid_storage_live_smoke.sh
```

Current live process graph:

```text
ron-accounting ingest: 127.0.0.1:19600
svc-wallet:            127.0.0.1:18088
svc-storage:           127.0.0.1:15303
```

Final observed flow:

```text
building svc-wallet, svc-storage, ron-accounting ingest, local BLAKE3 helper
starting ron-accounting ingest
ron-accounting is healthy
starting svc-wallet
svc-wallet is healthy
starting svc-storage in wallet-receipt + wallet-capture + accounting-http mode
svc-storage is healthy
object CID computed
context idem computed
issue ROC to payer
create wallet hold bound to context
write paid object through svc-storage
storage captures/releases and exports usage
read object back
check final balances
check ron-accounting snapshot
prove same hold cannot be replayed for different bytes
WEB3 paid-storage live smoke with settlement + real accounting export green
```

Final balance proof:

```text
payer balance:  52
escrow balance: 0
payee balance:  48
```

Final accounting snapshot proof:

```text
row_count = 3

PIN_SECONDS / requests = 60
PUT / bytes = 48
REQ_OK / requests = 1
```

Final live output confirms all of this. 

---

## 6. Current Test Status

Final command:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
```

Final results:

```text
clippy: clean
paid_write_economics: 6 passed
full svc-storage tests: all passed
live paid-storage smoke: green
```

Detailed observed test files:

```text
http_blackbox.rs
paid_write_accounting_export.rs
paid_write_economics.rs
paid_write_http_clients.rs
paid_write_policy.rs
paid_write_settlement.rs
paid_write_verifier.rs
paid_write_wallet_mode.rs
web3_paid_storage_loop.rs
bench targets: etag_range, read_path, write_path
```

The final terminal output confirms all these gates. 

Standing full gate:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
```

Economics-focused gate:

```bash
cargo test -p svc-storage --test paid_write_economics
```

Accounting-focused gate:

```bash
cargo test -p svc-storage --test paid_write_accounting_export
```

Settlement-focused gate:

```bash
cargo test -p svc-storage --test paid_write_settlement
```

Wallet receipt-focused gate:

```bash
cargo test -p svc-storage --test paid_write_wallet_mode
cargo test -p svc-storage --test paid_write_http_clients
cargo test -p svc-storage --test paid_write_verifier
```

---

## 7. Current Completion Estimate

Updated after this session:

```text
svc-storage core CAS baseline:                   90–94%
free object read/write path:                     92–96%
HEAD/range metadata path:                        88–94%
paid-write admission path:                       98–99%
wallet receipt verification path:                97–99%
context binding / replay protection:             97–99%
wallet capture/release settlement path:          97–99%
accounting usage export path:                    93–97%
ron-accounting live export proof:                97–99%
ROC economics optional pricing consumer:         86–92%
paid storage WEB3 beta readiness:                98–99%
svc-storage production-hardening readiness:      90–94%
```

Interpretation:

```text
For WEB3 beta:
  svc-storage is effectively complete for paid storage.

For production:
  remaining work is mostly durability, retry/recovery, auth/capabilities,
  gateway/omnigate exposure, economics-enabled smoke, and product-layer integration.
```

---

## 8. Important Invariants to Preserve

Do not regress:

```text
storage stores bytes by canonical b3 CID
content ID is b3:<64 lowercase hex>
no name/manifest/ownership logic in raw storage
no direct ledger mutation from svc-storage
all wallet mutation goes through svc-wallet
paid writes fail closed without valid proof
wallet-receipt mode verifies receipt over HTTP
dev-header mode stays explicit/local
disabled mode fails closed
wallet hold must be bound to exact CID/context
same hold cannot be replayed for different bytes
paid operation uses hold → capture → release
capture amount cannot exceed held amount
release amount equals held amount minus capture amount
settlement idempotency keys remain deterministic
usage export is observable
accounting export failure is visible, not silent
ron-accounting remains usage/metering, not truth
no floating-point money math
all ROC amounts are integer minor units
economics config is opt-in and fail-closed when explicitly set
high-cardinality values do not enter Prometheus labels
no ROX/Solana/external settlement logic
```

Do not put these into `svc-storage`:

```text
asset ownership registry
payout address lookup
crab:// link resolution
manifest-backed asset pages
passport key custody
reward calculation
ledger replay
wallet balance truth
```

---

## 9. What Remains for svc-storage

### Priority A — Economics-enabled live smoke

Current live smoke proves fallback legacy pricing. Next smoke should prove economics pricing.

Needed script or script mode:

```text
scripts/web3_paid_storage_live_smoke.sh --economics
```

or env-based:

```bash
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml \
bash scripts/web3_paid_storage_live_smoke.sh
```

Changes needed:

```text
increase hold amount to cover configured price
assert capture_amount_minor = 84 for 48-byte object
assert release amount reflects configured price
assert final balances conserve under economics pricing
assert old fallback smoke still passes with env unset
```

Expected economics-enabled balance example if issue = 120 and hold = 100:

```text
capture = 84
release = 16
payer final = 36
escrow final = 0
payee final = 84
```

This is the next most direct proof that `ron-policy` economics is live in the paid storage path.

### Priority B — Settlement recovery / retry hardening

Current settlement is live and deterministic, but production needs recovery.

Needed:

```text
capture retry with deterministic idempotency
release retry with deterministic idempotency
failed write release retry
write succeeds but capture fails quarantine
write succeeds but release fails quarantine
admin/debug settlement status
manual reconcile command
dead-letter settlement queue
metrics for settlement failure/retry/quarantine
```

Important scenario:

```text
storage write succeeds
wallet capture fails temporarily
object exists but payment not settled
```

This must become recoverable, visible, and safe.

### Priority C — Accounting export outbox

Current HTTP export is live and green, but export failure is reported in response rather than production-hardened.

Needed:

```text
bounded accounting export outbox
retry/backoff
dead-letter queue
idempotent export replay
admin/debug export status
metric for queued/exported/failed/dead-letter events
optional WAL/outbox persistence
```

Prior notes already list this as a recommended next production hardening item. 

### Priority D — Capability/passport integration

Current smokes use dev bearer and service-local modes.

Needed:

```text
ron-auth capability verification
svc-passport identity binding
wallet-linked passport permission checks
payer account caveats
storage action caveats
max bytes caveats
CID/action caveats
expiry/TTL caveats
tenant caveats
anonymous passport spend caps
```

Important: capability enforcement should fail closed.

### Priority E — Gateway/omnigate exposure

`/paid/o` is proven directly on `svc-storage`. Next product step is exposing it through:

```text
svc-gateway
omnigate
future browser extension flow
```

Needed:

```text
gateway route to paid storage
gateway/omnigate preflight price estimate
wallet hold UX handoff
storage receipt proof pass-through
object CID returned to client
accounting export preserved
```

Prior TODO notes identified storage/gateway/omnigate paid enforcement as the correct service boundary. 

### Priority F — Paid storage estimate endpoint

Now that economics can price storage, add a read-only estimate route:

```text
GET /paid/o/estimate?bytes=...
POST /paid/o/estimate
```

Suggested response:

```json
{
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 48,
  "estimate_minor": "84",
  "pricing_mode": "roc-economics",
  "economics_policy": "configs/roc-economics.toml"
}
```

Useful for gateway/browser/passport UX before wallet hold creation.

### Priority G — Pinning semantics

Current smoke includes `pin_seconds` accounting events, but full pinning product behavior is not finished.

Needed:

```text
explicit pin request model
pin duration validation
pin extension
pin expiry
pin accounting
pin pricing through paid_storage_pin
pin status introspection
pin cleanup/GC behavior
```

### Priority H — Storage durability / amnesia posture

Need decide production posture:

```text
what storage backend is beta default?
what survives restart?
what is amnesia mode allowed to persist?
what is cache vs durable object?
what is GC policy?
what happens to paid objects in amnesia mode?
how are providers/accounting rewarded if bytes disappear?
```

Possible modes:

```text
memory/dev
local disk CAS
sled/redb/rocks style index if already available
provider-backed object store
amnesia ephemeral mode
```

Storage durability must be honest in `/readyz` and dashboard.

### Priority I — Metrics expansion

Current metrics exist for paid writes and accounting export. Expand:

```text
storage_paid_write_total{status}
storage_paid_write_bytes_total
storage_accounting_export_total{status}
storage_accounting_export_events_total
storage_settlement_total{status}
storage_settlement_capture_total{status}
storage_settlement_release_total{status}
storage_economics_price_total{status,action}
storage_economics_config_loaded
storage_wallet_receipt_lookup_seconds
storage_paid_context_mismatch_total
storage_object_bytes_total
storage_cas_get_total{status}
storage_cas_put_total{status}
```

Avoid high-cardinality labels:

```text
CID
wallet txid
account ID
receipt hash
raw user ID
raw route variants
```

### Priority J — Dashboard/admin visibility

Potential safe endpoints:

```text
GET /admin/storage/status
GET /admin/storage/paid-summary
GET /admin/storage/accounting-export
GET /admin/storage/settlement-queue
GET /admin/storage/economics
```

Do not expose sensitive wallet tokens or unbounded CIDs/account IDs in public metrics.

### Priority K — B3 asset product layer integration

Current storage returns raw CID. WEB3_2 wants asset pages:

```text
crab://b3/<hash>.image
crab://b3/<hash>.song
crab://b3/<hash>.article
crab://b3/<hash>.video
crab://b3/<hash>.comment
```

But this belongs upstream.

Storage should expose enough provider data:

```text
CID exists?
bytes length
content type if known
storage provider/service ID
pin status if implemented
receipts if authorized
```

Ownership, payout address, tags, description, curator metadata, and provenance belong in manifests/index/gateway/omnigate.

### Priority L — Stronger body/context limits

Need:

```text
max paid object size
max free object size
body timeout
body streaming limit
hash streaming rather than all-in-memory where needed
decompression cap if applicable
CID mismatch rejection if client-supplied CID exists
```

### Priority M — Broader paid content integration

Paid storage is the first paid operation. Next paid service paths will need storage output:

```text
paid content view
paid song play
paid video view
site visit
app access
```

`svc-storage` should remain byte storage. Gateway/omnigate should orchestrate content views and wallet holds/captures.

---

## 10. Known Commands

### Full storage gate

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
```

### Economics gate

```bash
cargo test -p svc-storage --test paid_write_economics
```

### Paid storage live smoke

```bash
bash scripts/web3_paid_storage_live_smoke.sh
```

Expected final line:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```

### Future economics-enabled live smoke

```bash
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml \
bash scripts/web3_paid_storage_live_smoke.sh
```

Important: hold amount in the script must be raised because the current 48-byte body prices to `84` minor units under `paid_storage_put`, not legacy `48`.

### Manual storage run

```bash
RON_STORAGE_ADDR=127.0.0.1:15303 \
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt \
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture \
RON_STORAGE_WALLET_BASE_URL=http://127.0.0.1:18088 \
RON_STORAGE_WALLET_BEARER=dev \
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http \
RON_STORAGE_ACCOUNTING_BASE_URL=http://127.0.0.1:19600 \
RON_STORAGE_ACCOUNTING_BEARER=dev \
cargo run -p svc-storage
```

### Manual economics-enabled storage run

```bash
RON_STORAGE_ADDR=127.0.0.1:15303 \
RON_STORAGE_PAID_WRITE_VERIFIER_MODE=wallet-receipt \
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture \
RON_STORAGE_WALLET_BASE_URL=http://127.0.0.1:18088 \
RON_STORAGE_WALLET_BEARER=dev \
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http \
RON_STORAGE_ACCOUNTING_BASE_URL=http://127.0.0.1:19600 \
RON_STORAGE_ACCOUNTING_BEARER=dev \
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml \
cargo run -p svc-storage
```

---

## 11. Suggested NOTES.MD Commit Summary

```text
svc-storage: document ROC paid-storage economics consumer and live loop

- preserve existing paid storage loop: wallet issue, hold, receipt verification,
  CID-bound paid write, wallet capture/release, accounting export, object readback,
  balance conservation, and replay rejection
- add ron-policy dependency for optional ROC economics pricing
- add policy/economics adapter for RON_STORAGE_ROC_ECONOMICS_PATH
- preserve legacy max(bytes,1) pricing when economics env is unset
- fail closed when economics config is explicitly set but invalid/missing/unknown
- wire /paid/o settlement plan to configured economics capture amount
- add paid_write_economics tests and prove 6/6 pass
- keep full svc-storage test suite and live smoke green
```

Longer body:

```text
svc-storage is now the first live consumer of the RustyOnions ROC economics policy.
The paid storage route can still run in legacy beta mode by default, but when
RON_STORAGE_ROC_ECONOMICS_PATH is set it prices paid_storage_put through
ron-policy and configs/roc-economics.toml. The explicit economics path fails
closed on missing config, empty config path, invalid policy, or unknown action.

The existing paid-storage live loop remains green: wallet issue/hold, wallet
receipt verification, CID-bound storage, wallet capture/release, accounting
export, snapshot verification, object readback, and replay rejection.
```

---

## 12. Final Status

`svc-storage` should currently be considered:

```text
Status: WEB3 beta paid-storage path essentially complete
WEB3 beta readiness for paid storage: 98–99%
Production-hardening readiness: 90–94%
Main remaining work: economics-enabled live smoke, settlement recovery,
accounting outbox/retry, capability/passport enforcement, gateway/omnigate
exposure, pinning semantics, storage durability/amnesia policy, and dashboard/admin visibility.
```

Most important current achievement:

```text
RustyOnions now has a real paid in-network use case:

ROC wallet hold
→ paid storage enforcement
→ b3 content-addressed write
→ wallet capture/release
→ accounting export
→ accounting snapshot
→ replay rejection
```

And after this session, the paid storage price can be driven by the canonical `configs/roc-economics.toml` policy instead of being permanently hardcoded.


### END NOTE - APRIL 28 2026 - 23:10 CST



### BEGIN NOTE - APRIL 29 2026 - 00:50 CST

# NOTE — `svc-storage` Session Notes

## Current status

`svc-storage` is now a **beta-grade WEB3 paid storage enforcement point**. In this session, we moved it beyond “paid storage works internally” into a complete product-facing flow where pricing, paid write enforcement, wallet settlement, accounting export, CID readback, and replay rejection are all proven through the full edge path.

The strongest proof from this session is the final live edge smoke:

```text
svc-gateway
→ omnigate
→ svc-storage
→ svc-wallet
→ ron-accounting
```

That smoke returned the expected gateway estimate of **84 ROC minor units** for the 48-byte object, created a wallet hold, wrote the object through gateway/omnigate into `svc-storage`, captured 84, released 16, exported accounting, read the object back by b3 CID, and rejected replay with `402 payment_required`.  

---

# 1. What `svc-storage` is responsible for now

`svc-storage` is still fundamentally the CAS object service, but for WEB3/ROC it now also acts as the first real paid-use enforcement point.

Its current responsibilities are:

```text
1. Free/dev CAS object storage
2. BLAKE3 b3:<64 lowercase hex> CID generation
3. HEAD/GET object readback by CID
4. Paid object write route at /paid/o
5. Paid storage estimate route at /paid/o/estimate
6. Wallet receipt verification for paid writes
7. CID-bound paid context verification
8. Wallet capture/release settlement through svc-wallet
9. Usage event generation for ron-accounting
10. HTTP accounting export
11. Replay rejection for mismatched body/CID/context
12. Economics-backed ROC pricing via configs/roc-economics.toml
```

The crate still preserves the correct modular boundary:

```text
svc-storage does not mutate ledger directly.
svc-storage does not own account balances.
svc-storage does not issue/capture/release by itself.
svc-storage asks svc-wallet to perform economic mutation.
svc-storage exports usage facts to ron-accounting.
svc-storage stores bytes by b3 content ID.
```

That boundary is now live-proven, not just designed.

---

# 2. Files changed or materially involved this session

## Direct `svc-storage` files

```text
crates/svc-storage/src/policy/economics.rs
crates/svc-storage/src/http/routes/paid_estimate.rs
crates/svc-storage/src/http/routes/mod.rs
crates/svc-storage/src/http/server.rs
crates/svc-storage/src/lib.rs
crates/svc-storage/tests/paid_write_estimate.rs
```

## Existing `svc-storage` files that were exercised by the session

```text
crates/svc-storage/src/http/routes/paid_object.rs
crates/svc-storage/src/policy/settlement.rs
crates/svc-storage/src/policy/verifier.rs
crates/svc-storage/src/accounting/*
crates/svc-storage/src/storage/*
crates/svc-storage/src/bin/roc_b3_tool.rs
crates/svc-storage/tests/paid_write_economics.rs
crates/svc-storage/tests/paid_write_accounting_export.rs
crates/svc-storage/tests/paid_write_settlement.rs
crates/svc-storage/tests/paid_write_verifier.rs
crates/svc-storage/tests/paid_write_wallet_mode.rs
crates/svc-storage/tests/web3_paid_storage_loop.rs
```

## Root scripts added/updated that prove `svc-storage`

```text
scripts/web3_paid_storage_live_smoke.sh
scripts/web3_paid_estimate_edge_smoke.sh
scripts/web3_paid_storage_edge_write_smoke.sh
```

The most important final script is:

```text
scripts/web3_paid_storage_edge_write_smoke.sh
```

It proves the real service chain with `svc-storage` acting as the storage/payment enforcement point.

---

# 3. Major accomplishment: `/paid/o/estimate`

## What we added

We added:

```text
GET /paid/o/estimate?bytes=N
```

This endpoint is read-only and side-effect-free.

It does not:

```text
create wallet holds
mutate ledger state
store object bytes
export accounting events
touch manifests
touch passports
touch naming/index ownership
```

It only returns the paid storage quote that the client should use before making a wallet hold.

## Response shape

The endpoint returns:

```json
{
  "schema": "svc-storage.paid-storage-estimate.v1",
  "route": "/paid/o",
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 48,
  "amount_minor": "84",
  "minimum_hold_minor": "84",
  "pricing_mode": "roc-economics",
  "economics_policy_path": "configs/roc-economics.toml"
}
```

The final edge write smoke shows this exact response flowing through gateway → omnigate → storage with `amount_minor = 84` and `pricing_mode = roc-economics`. 

## Why this matters

Before this endpoint, a client had to know the pricing rules or over-hold blindly. Now the product flow is clean:

```text
1. Ask storage/gateway what the write costs.
2. Create a wallet hold for at least that amount.
3. Submit bytes plus wallet receipt.
4. Let storage capture/release the exact amount.
```

This is the bridge between backend ROC economics and browser/dashboard UX.

---

# 4. Economics adapter integration

`svc-storage` now uses the optional ROC economics adapter for paid storage pricing.

The relevant behavior is:

```text
If RON_STORAGE_ROC_ECONOMICS_PATH is unset:
  preserve legacy beta pricing
  capture = max(bytes_stored, 1)

If RON_STORAGE_ROC_ECONOMICS_PATH is set:
  load configs/roc-economics.toml
  parse/validate through ron-policy
  price configured action
  fail closed on explicit config errors
```

The important env vars are:

```text
RON_STORAGE_ROC_ECONOMICS_PATH
RON_STORAGE_ROC_ECONOMICS_ACTION
```

Default action:

```text
paid_storage_put
```

The previous notes already documented this as the correct design: `svc-storage` may depend on `ron-policy` for **read-only economics policy**, but still must not mutate wallet/ledger directly. 

## Current proven pricing

For the session’s canonical test object:

```text
object bytes = 48
action = paid_storage_put
policy path = configs/roc-economics.toml
price = 84 ROC minor units
```

The final live smoke proved the economics-backed quote at the product-facing gateway route. 

---

# 5. Important implementation fix: removed Axum `Query` dependency

During the big edge-write smoke, `svc-storage` failed to compile because `axum::extract::Query` was not available under the crate’s enabled Axum features.

The error was:

```text
unresolved import axum::extract::Query
no Query in extract
the item is gated behind the query feature
```

Instead of changing `Cargo.toml`, we fixed `paid_estimate.rs` by parsing the query string from `Uri`.

That was the correct fix because:

```text
1. It avoided adding a new Axum feature.
2. It kept Cargo.toml unchanged.
3. It kept the endpoint small and explicit.
4. It preserved the existing minimal Axum feature posture.
5. It restored clippy/test green status for svc-storage.
```

After that fix, the focused storage gate passed:

```text
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_estimate
```

with all six `paid_write_estimate` tests green.

---

# 6. Tests added or strengthened

## `paid_write_estimate.rs`

This session added/solidified the dedicated estimate endpoint test coverage.

Current test cases:

```text
legacy_estimate_preserves_beta_bytes_pricing_when_policy_unset
economics_estimate_uses_checked_in_roc_economics_policy
economics_estimate_honors_action_override
estimate_rejects_missing_bytes_query
estimate_rejects_invalid_bytes_query
explicit_bad_policy_path_fails_closed
```

What these prove:

```text
1. Legacy fallback pricing still works.
2. Economics config pricing works.
3. checked-in configs/roc-economics.toml prices 48 bytes at 84.
4. Alternate action pricing works.
5. Missing bytes query fails with 400.
6. Non-numeric bytes query fails with 400.
7. Explicit bad config path fails closed.
```

The final passing output showed all six estimate tests passing after the clippy fixes. 

## Clippy fixes in tests

The first test implementation used a normal `std::sync::Mutex` guard across `.await`, which clippy rejected under `-D warnings` with `await_holding_lock`. 

We fixed that by using an async-aware `tokio::sync::Mutex` for the env lock. That preserved deterministic env-var isolation without violating the “no blocking lock across await” rule.

## Removed unused import

The initial `paid_estimate.rs` implementation had an unused `Response` import. Clippy rejected it under `-D warnings`; we removed it. 

---

# 7. Live smoke proof: economics-enabled direct paid storage

We updated/proved the direct live storage smoke with economics pricing.

The economics-enabled smoke proved:

```text
pricing mode: roc-economics:paid_storage_put
capture amount: 84
release amount: 16
accounting export: exported
accounting event count: 3
payer balance: 16
escrow balance: 0
payee balance: 84
replay with different bytes rejected
```

The smoke output also proved accounting rows for:

```text
bytes_stored = 48
request_ok = 1
pin_seconds = 60
```

and ended with:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
```



This is important because before this session, the earlier live smoke mostly proved legacy pricing. The economics path is now live-proven with the real config.

---

# 8. Live smoke proof: product-facing estimate edge path

We added/proved a read-only edge estimate smoke:

```text
scripts/web3_paid_estimate_edge_smoke.sh
```

That smoke starts:

```text
svc-storage
omnigate
svc-gateway
```

and proves:

```text
svc-gateway /paid/o/estimate?bytes=48
→ omnigate /v1/paid/o/estimate?bytes=48
→ svc-storage /paid/o/estimate?bytes=48
→ configs/roc-economics.toml
→ amount_minor = 84
```

This matters for `svc-storage` because it proves the estimate endpoint is consumable as a product-facing primitive without clients needing to talk directly to the storage service.

---

# 9. Live smoke proof: full edge paid-write path

The biggest proof of the session was:

```text
scripts/web3_paid_storage_edge_write_smoke.sh
```

It starts:

```text
ron-accounting
svc-wallet
svc-storage
omnigate
svc-gateway
```

Then it proves the full paid storage path.

## Step-by-step proof

### 1. Storage object identity

The test object was:

```text
object bytes = 48
CID = b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
```

The final smoke prints that CID and object size before starting the service chain. 

### 2. Product-facing estimate

The gateway estimate returned:

```text
action = paid_storage_put
asset = roc
bytes = 48
amount_minor = 84
minimum_hold_minor = 84
pricing_mode = roc-economics
economics_policy_path = configs/roc-economics.toml
```



### 3. Wallet funding

The script issued:

```text
100 ROC minor units
to acct_user
```

Then created a hold:

```text
from = acct_user
to = escrow_paid_write
asset = roc
amount_minor = 100
idem = storage_put:3519a6eed7dfb69a39e437b92a16e881
```

That hold was intentionally higher than the 84 quote so storage could capture 84 and release 16. 

### 4. Paid write through edge

The write went through:

```text
svc-gateway /paid/o
→ omnigate /v1/paid/o
→ svc-storage /paid/o
```

The storage response proved:

```text
cid = b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
paid = true
payer = acct_user
escrow = escrow_paid_write
verifier = wallet-http
estimate_minor = 100
settlement.mode = wallet-capture
capture_amount_minor = 84
release_amount_minor = 16
```



### 5. Wallet capture/release

The settlement response included both receipts:

```text
capture:
  from escrow_paid_write
  to svc_storage
  amount 84
  op capture

release:
  from escrow_paid_write
  to acct_user
  amount 16
  op release
```

This proves `svc-storage` correctly delegates economic mutation to `svc-wallet` and receives committed receipts back. 

### 6. Accounting export

The storage response included:

```text
accounting_export.mode = http
accounting_export.status = exported
accounting_export.event_count = 3
accounting_export.http_status = 202
```

Usage events were:

```text
bytes_stored = 48
request_ok = 1
pin_seconds = 60
```



### 7. Object readback

The script read the object back from storage:

```text
GET /o/<cid>
```

and byte-compared it against the original object with `cmp`.

That proves the paid route stored the same bytes under the expected b3 CID.

### 8. Final balance conservation

The final wallet balances were:

```text
payer balance = 16
escrow balance = 0
payee balance = 84
```

This exactly matches:

```text
issue 100
hold 100
capture 84
release 16
```



### 9. Accounting snapshot

The accounting snapshot reported:

```text
row_count = 3
PIN_SECONDS = 60
PUT bytes = 48
REQ_OK = 1
```



### 10. Replay rejection

The script tried to reuse the same wallet hold for different bytes through the full edge path.

Storage rejected it with:

```text
error = payment_required
reason = wallet receipt paid storage context idem mismatch
```

The final summary confirms:

```text
replay_rejection: 402 payment_required
```



---

# 10. Current `svc-storage` feature inventory

## Free/dev CAS path

Still supported:

```text
POST /o
PUT /o
HEAD /o/:cid
GET /o/:cid
Range GET
ETag
Content-Length
404 unknown CID
404/validation behavior for bad CID paths
```

## Paid storage path

Now supported:

```text
POST /paid/o
PUT /paid/o
```

The paid path performs:

```text
1. Body hash → b3 CID
2. Paid proof header parse
3. Wallet receipt lookup/verification
4. Paid context idem recomputation
5. CID/payer/escrow/asset/amount binding
6. CAS write
7. Wallet capture
8. Wallet release
9. Usage event generation
10. Accounting export
11. JSON response with settlement/accounting details
```

## Estimate path

Now supported:

```text
GET /paid/o/estimate?bytes=N
```

The estimate path performs:

```text
1. Query parse from Uri
2. Legacy/economics pricing selection
3. Integer-only minor-unit price calculation
4. Fail-closed config handling
5. Read-only JSON response
```

## Economics modes

Current modes:

```text
legacy:
  RON_STORAGE_ROC_ECONOMICS_PATH unset
  capture = max(bytes_stored, 1)

roc-economics:
  RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml
  RON_STORAGE_ROC_ECONOMICS_ACTION=paid_storage_put
  48 bytes = 84 minor units
```

## Settlement modes

Current relevant mode:

```text
RON_STORAGE_PAID_SETTLEMENT_MODE=wallet-capture
```

Current settlement result:

```text
capture amount <= held amount
release amount = held amount - capture amount
capture receipt returned
release receipt returned
```

## Accounting export modes

Current relevant mode:

```text
RON_STORAGE_ACCOUNTING_EXPORT_MODE=http
```

Current export result:

```text
POST /v1/usage-events
status exported
event_count 3
http_status 202
```

---

# 11. Current green gates from this session

## Focused `svc-storage` gate

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_estimate
```

Result:

```text
clippy clean
paid_write_estimate: 6 passed
```

## Broader `svc-storage` gate from the estimate endpoint batch

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_estimate
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --all-targets
```

Result:

```text
paid_write_estimate: 6 passed
paid_write_economics: 6 passed
config tests: 9 passed
http_blackbox: 1 passed
paid_write_accounting_export: 1 passed
paid_write_http_clients: 7 passed
paid_write_policy: 5 passed
paid_write_settlement: 1 passed
paid_write_verifier: 19 passed
paid_write_wallet_mode: 1 passed
web3_paid_storage_loop: 1 passed
bench targets compiled as test targets
```

The terminal output confirms the estimate and economics tests both passed, and the broader storage suite continued through all the paid-write/verifier/wallet-mode/web3-loop tests successfully. 

## Direct economics-enabled live smoke

```bash
bash scripts/web3_paid_storage_live_smoke.sh
```

Result:

```text
WEB3 paid-storage live smoke with settlement + real accounting export green
capture amount: 84
release amount: 16
```



## Full edge paid-write live smoke

```bash
bash scripts/web3_paid_storage_edge_write_smoke.sh
```

Result:

```text
WEB3 edge paid storage write smoke green
path: svc-gateway → omnigate → svc-storage → wallet/accounting
capture_minor: 84
release_minor: 16
accounting_rows: 3
replay_rejection: 402 payment_required
```



---

# 12. What is complete for `svc-storage`

## Complete or beta-complete

```text
1. Free CAS object write/read path.
2. b3 CID addressing for stored bytes.
3. Paid write route /paid/o.
4. Paid estimate route /paid/o/estimate.
5. Optional ROC economics pricing.
6. Legacy fallback pricing.
7. Wallet receipt verification mode.
8. Wallet capture/release settlement mode.
9. CID-bound context-idem anti-replay.
10. HTTP accounting export.
11. Usage event generation.
12. Direct live paid-storage smoke.
13. Edge live paid-storage smoke through gateway/omnigate.
14. Readback proof by CID.
15. Rejected replay does not add accounting rows.
16. Clippy/test green for touched storage paths.
```

## Current readiness estimate

For `svc-storage` specifically:

```text
Paid storage WEB3 beta readiness: 96–98%
General svc-storage beta readiness: 93–96%
Production-hardening readiness: 85–90%
```

Why not 100%? Because the happy path and safety checks are strong, but full production recovery/outbox/admin reconciliation is still pending.

---

# 13. What remains for `svc-storage`

## 13.1 Settlement recovery hardening

The current flow is strong for the normal path, but production needs better recovery around partial failures.

Remaining cases:

```text
1. Storage write succeeds but wallet capture fails.
2. Capture succeeds but release fails.
3. Write fails after hold verification and needs release/refund.
4. Accounting export fails after settlement succeeds.
5. Process crashes between write/capture/release/export.
6. Duplicate retry arrives after partial settlement.
```

Needed mechanisms:

```text
settlement outbox
failed-write release retry
capture/release retry with idempotency
dead-letter/quarantine queue
admin reconcile endpoint or CLI
durable settlement journal
metrics for stuck settlement states
```

## 13.2 Accounting export outbox

Current HTTP export works and is live-proven, but production needs durable retry semantics.

Remaining work:

```text
1. Durable export outbox.
2. Retry with exponential backoff.
3. Idempotent replay against ron-accounting.
4. Dead-letter state after repeated failures.
5. Metrics:
   accounting_export_attempts_total
   accounting_export_failures_total
   accounting_export_retries_total
   accounting_export_deadletter_total
   accounting_export_latency_seconds
```

## 13.3 Better product response shaping

The current `/paid/o` response is rich but very internal.

For dashboard/browser-extension UX, it should eventually provide a cleaner stable product envelope.

Possible product response shape:

```json
{
  "schema": "svc-storage.paid-object-write.v1",
  "cid": "b3:<hash>",
  "paid": true,
  "asset": "roc",
  "charged_minor": "84",
  "released_minor": "16",
  "payer": "acct_user",
  "provider": "svc_storage",
  "settlement": {
    "capture_receipt_hash": "b3:<hash>",
    "release_receipt_hash": "b3:<hash>"
  },
  "accounting": {
    "status": "exported",
    "event_count": 3
  },
  "next": {
    "read_url": "/o/b3:<hash>",
    "asset_page": "crab://b3/<hash>.image"
  }
}
```

Important: this should be additive or versioned; do not break current tests or internal receipt details until product contracts are formalized.

## 13.4 Capability/passport integration

WEB3_2 requires passport/wallet-linked permissions later.

For storage, likely caveats include:

```text
payer account
wallet/passport binding
allowed action = paid_storage_put
max bytes
max ROC minor units
CID caveat
tenant caveat
region caveat
expiry/TTL
anonymous passport limits
per-site spend limits
```

The current storage route validates wallet receipt/context but does not yet enforce passport-level caveats.

## 13.5 Typed b3 asset page integration

Storage correctly stores bytes by b3 CID only. It should not become the owner of manifests or typed asset pages.

Remaining integration points:

```text
ron-naming / svc-index / omnigate:
  typed asset manifests
  ownership metadata
  payout address
  tags/description
  curator metadata
  provenance
  storage/provider data
  receipts

svc-storage:
  byte storage by CID
  object existence/readback
  maybe provider proof/status
```

For WEB3_2, storage should remain the CAS substrate and avoid app-specific logic.

## 13.6 Pinning semantics

The session proved `x-ron-pin-seconds: 60` flows into usage events, but true paid pinning semantics still need definition.

Remaining work:

```text
paid_storage_pin action
pin duration
pin extension
pin expiry
pin status endpoint
pin cleanup/GC
pin pricing through roc-economics.toml
provider reward implications
```

## 13.7 Storage durability/amnesia posture

Need final policy decisions for paid objects:

```text
memory-only vs disk-backed modes
paid object durability guarantee
amnesia mode behavior
paid object TTL / pin requirements
GC policy
provider reward eligibility
what receipts survive in amnesia mode
what logs are suppressed
```

Storage must keep amnesia-mode rules truthful: do not claim durable retention in an ephemeral mode.

## 13.8 Metrics expansion

Useful next metrics:

```text
paid_estimate_requests_total
paid_estimate_failures_total
paid_write_requests_total
paid_write_rejections_total
paid_context_mismatch_total
wallet_receipt_lookup_latency_seconds
settlement_capture_total
settlement_release_total
settlement_capture_failures_total
settlement_release_failures_total
economics_price_total
economics_price_failures_total
accounting_export_total
accounting_export_failures_total
```

## 13.9 End-to-end CI gate

The smoke scripts are now very valuable. Eventually they should become a stable CI/nightly or devnet gate.

Candidate gates:

```bash
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --all-targets
bash scripts/web3_paid_storage_live_smoke.sh
bash scripts/web3_paid_estimate_edge_smoke.sh
bash scripts/web3_paid_storage_edge_write_smoke.sh
```

Some may remain local/devnet because they spawn multiple services and bind ports.

---

# 14. Developer notes for next session

## Do not regress these invariants

```text
1. Storage stores bytes by b3 CID.
2. Names/manifests/ownership do not belong in svc-storage.
3. svc-storage does not directly mutate ledger.
4. Wallet mutation goes through svc-wallet.
5. Accounting is usage export, not ledger truth.
6. Money math remains integer minor units only.
7. Economics config must fail closed when explicitly set.
8. Legacy mode must remain stable when economics env is unset.
9. Paid context idem must bind CID/body hash.
10. Reusing a hold for different bytes must reject before write/settlement/accounting.
11. Gateway/omnigate are proxies; storage remains enforcement point.
```

## Important env vars

```text
ADDR
RON_STORAGE_PAID_WRITE_VERIFIER_MODE
RON_STORAGE_PAID_SETTLEMENT_MODE
RON_STORAGE_PAID_SETTLEMENT_PAYEE
RON_STORAGE_WALLET_BASE_URL
RON_STORAGE_WALLET_BEARER
RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS
RON_STORAGE_ACCOUNTING_EXPORT_MODE
RON_STORAGE_ACCOUNTING_BASE_URL
RON_STORAGE_ACCOUNTING_BEARER
RON_STORAGE_ACCOUNTING_TIMEOUT_MS
RON_STORAGE_ROC_ECONOMICS_PATH
RON_STORAGE_ROC_ECONOMICS_ACTION
```

## Important live values from this session

```text
object bytes: 48
object CID: b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
economics action: paid_storage_put
economics path: configs/roc-economics.toml
estimate amount: 84
hold amount: 100
capture amount: 84
release amount: 16
payer final balance: 16
escrow final balance: 0
payee final balance: 84
accounting row_count: 3
replay rejection: 402 payment_required
```

## Exact proof path

```text
GET  svc-gateway /paid/o/estimate?bytes=48
POST svc-wallet /v1/issue
POST svc-wallet /v1/hold
POST svc-gateway /paid/o
GET  svc-storage /o/<cid>
GET  ron-accounting /v1/snapshot
POST svc-gateway /paid/o with same hold and different bytes
```

---

# 15. Suggested next work for `svc-storage`

The best next `svc-storage`-specific batch would be one of these:

## Option A — Product response envelope

Make the `/paid/o` response easier for gateway/dashboard/browser-extension UX.

Goal:

```text
Keep existing full internal response fields,
but add a stable product-facing envelope:
  schema
  cid
  charged_minor
  released_minor
  asset
  payment_status
  accounting_status
  receipt_hashes
  read path
```

This is probably the highest-value next polish.

## Option B — Settlement/accounting outbox design

Add a durable outbox for settlement/export recovery.

Goal:

```text
make crash/retry behavior production-safe
```

This is more production-hardening than beta-demo work.

## Option C — Pinning model

Turn `pin_seconds` from a usage event into a true pinning contract.

Goal:

```text
paid_storage_pin
pin status
pin expiry
pin extension
pin cleanup
```

This is needed for a real storage marketplace.

## Option D — Capability/passport caveats

Prepare storage enforcement for RON Passports.

Goal:

```text
validate passport caveats before paid write:
  action
  max bytes
  max spend
  tenant
  expiry
  wallet-linked account
```

This belongs to WEB3_2 integration.

---

# 16. Bottom line

`svc-storage` is now one of the strongest crates in the WEB3 stack.

It has:

```text
working CAS storage
b3 CID identity
paid write enforcement
economics-backed estimate
wallet receipt verification
wallet capture/release settlement
accounting export
gateway/omnigate product path proof
object readback proof
anti-replay proof
clippy/test green touched paths
live multi-service smoke green
```

The crate is no longer just “storage with future payment hooks.” It is now a real internal ROC paid-use enforcement service.


### END NOTE - APRIL 29 2026 - 00:50 CST



### BEGIN NOTE - JUNE 14 2026 - 21:55 CST

Based on the latest visible terminal and bundle, `svc-storage` is green for focused QuickChain preflight, all-targets, and clippy, with the gate marker printed.  The regenerated bundle includes the active preflight script and the added QuickChain test suites through economics, range/media, observability, and docs.  This remains aligned with the QuickChain Phase-0 rule: docs/tests/boundary hardening are allowed, but roots/checkpoints/validators/bridges/external settlement are still forbidden. 

Paste this into the `svc-storage` crate notes:

# svc-storage QuickChain Phase-0 Carryover Notes

## BEGIN NOTE - JUNE 15 2026 - svc-storage QuickChain Phase-0 / Preflight Parking Notes

## 0. Executive summary

`svc-storage` has been brought through a strong QuickChain Phase-0 / preflight hardening pass.

The purpose of this work was not to turn `svc-storage` into QuickChain, a ledger, a wallet, a validator, a checkpoint writer, a root producer, a bridge, an anchor, or a settlement authority.

The purpose was to prove that `svc-storage` remains byte/object infrastructure inside the RustyOnions value loop:

```text
svc-storage stores and serves bytes by canonical b3.
svc-storage does not own economic truth.
svc-storage does not mutate ron-ledger directly.
svc-storage does not mint, issue, transfer, burn, hold, capture, or release ROC by itself.
svc-storage does not invent balances.
svc-storage does not invent receipts.
svc-storage does not invent finality.
svc-storage does not create state roots, receipt roots, checkpoints, validators, bridges, anchors, or external settlement.
svc-storage does not unlock paid content from cache or caller-supplied claims alone.
```

Current working estimate:

```text
svc-storage QuickChain Phase-0/preflight completion: ~92–94%
svc-storage full future QuickChain role completion: ~42–48%
```

Interpretation:

```text
For current Phase-0/preflight, svc-storage is effectively parked.
For full future QuickChain, there is still major future work around durable storage, production media delivery, archive/challenge/DA interactions, proof-aware read models, and integration with gateway/omnigate/client paid-access flows.
```

The crate should now be considered safe to pause while we move to the next QuickChain boundary crate:

```text
Next crate: svc-gateway
```

Reason:

```text
svc-gateway is the public route boundary.
It must enforce paid access using backend-derived wallet/storage/omnigate truth without becoming wallet truth, ledger truth, receipt truth, cache authority, QuickChain authority, bridge authority, or root authority.
```

## 1. Current proof status

Latest confirmed `svc-storage` proof:

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh passed.
cargo fmt -p svc-storage -- --check passed.
Focused QuickChain preflight tests passed.
cargo test -p svc-storage --all-targets passed.
cargo clippy -p svc-storage --all-targets -- -D warnings passed.
The script printed:
== svc-storage QuickChain preflight gate passed ==
```

Focused QuickChain preflight suites now covered:

```text
quickchain_preflight_boundary
quickchain_preflight_b3_integrity
quickchain_preflight_no_direct_mutation
quickchain_preflight_paid_cache
quickchain_preflight_economics_quote
quickchain_preflight_range_media
quickchain_preflight_observability
quickchain_preflight_docs
```

Full all-targets proof included:

```text
src/lib.rs unit tests passed.
src/bin/roc_b3_tool.rs test target passed.
src/main.rs test target passed.
tests/http_blackbox.rs passed.
tests/paid_write_accounting_export.rs passed.
tests/paid_write_economics.rs passed.
tests/paid_write_estimate.rs passed.
tests/paid_write_http_clients.rs passed.
tests/paid_write_policy.rs passed.
tests/paid_write_settlement.rs passed.
tests/paid_write_verifier.rs passed.
tests/paid_write_wallet_mode.rs passed.
all QuickChain preflight tests passed again under all-targets.
tests/web3_paid_storage_loop.rs passed.
bench test targets passed.
clippy passed with -D warnings.
```

## 2. What svc-storage now proves for QuickChain Phase 0

`svc-storage` now has Phase-0 proof coverage for the following boundaries.

### 2.1 Storage remains byte/object infrastructure

`svc-storage` can say:

```text
I have bytes.
I stored bytes.
These bytes hash to b3:<64 lowercase hex>.
This object has length N.
This range exists.
This ETag corresponds to object content.
This request was accepted by the storage admission path.
This usage event was emitted for accounting/metering.
```

`svc-storage` cannot say:

```text
This user has a balance.
This wallet operation is final.
This ledger operation is final.
This receipt is QuickChain-final.
This object is unlocked forever.
This checkpoint is signed.
This state root is valid.
This receipt root is valid.
This validator approved the payment.
This bridge settled.
This external chain settled.
This cache entry alone unlocks paid content.
```

### 2.2 Canonical b3 integrity

We added tests proving object ingest derives canonical b3 from exact body bytes.

The storage path now treats b3 as truth:

```text
b3:<64 lowercase hex>
```

The tests prove:

```text
Object ingest computes b3 from body bytes.
Returned CID is canonical b3.
Caller cannot retrieve bytes under a fake/noncanonical CID.
Malformed CID rejects before object lookup.
Valid but unknown b3 remains a normal 404 miss.
```

Important invariant:

```text
Names, URLs, routes, and caller claims are not storage truth.
The content hash is storage truth.
```

### 2.3 Public boundary: no QuickChain authority routes

We added boundary tests proving the `svc-storage` router exposes storage routes, not QuickChain authority routes.

Allowed route shape:

```text
/o
/o/:cid
/paid/o/estimate
/paid/o
/version
/healthz
/readyz
/metrics when metrics feature is enabled
```

Forbidden route concepts:

```text
wallet mutation routes
ledger mutation routes
QuickChain root routes
checkpoint routes
validator routes
bridge routes
anchor routes
external settlement routes
public chain routes
staking/liquidity routes
```

Important invariant:

```text
svc-storage is not a public QuickChain RPC surface.
```

### 2.4 No direct wallet/ledger mutation

We added no-direct-mutation tests proving production dependencies and route source do not turn storage into wallet or ledger authority.

The important rule:

```text
svc-wallet mutates.
ron-ledger is durable economic truth.
svc-storage does not mutate balances directly.
```

`svc-storage` may have dev/integration tests involving wallet/accounting crates, but production storage logic must not become the mutation front-door.

Tested doctrine:

```text
Production deps do not include wallet/ledger mutation authority.
Router does not expose wallet/ledger mutation endpoints.
Accounting export remains usage/metering, not balance truth.
Wallet capture/release references are constrained to explicit settlement adapter code.
```

### 2.5 Paid cache boundary

We added paid/cache preflight tests proving fake caller claims do not unlock missing bytes or paid content.

The tests prove:

```text
Fake cache headers do not unlock absent objects.
Fake paid headers do not unlock absent objects.
Paid write without backend-derived proof rejects.
Rejected paid write does not store/cache bytes.
Dev-header success response is labeled as storage admission, not QuickChain finality.
```

Important invariant:

```text
Storage cache is convenience, not authorization.
Offline cache cannot unlock paid content alone.
Paid access must come from backend wallet/receipt/admission path.
```

### 2.6 Quote-only economics boundary

We added `quickchain_preflight_economics_quote.rs`.

This test suite proves `/paid/o/estimate` is quote-only.

The estimate endpoint may return:

```text
schema
route
action
asset
bytes
amount_minor
minimum_hold_minor
pricing_mode
economics_policy_path
```

The estimate endpoint must not return:

```text
cid
paid
payer
escrow
wallet_txid
wallet_receipt_hash
wallet_idem
paid_context_idem
settlement
accounting_export
usage_events
balance_minor
state_root
receipt_root
checkpoint
validator
bridge
finality
```

The test suite also proves:

```text
Estimate route uses integer byte inputs only.
Float-like byte inputs reject.
Legacy beta pricing works when no economics policy env is set.
Checked-in ROC economics policy can quote paid_storage_put.
Quote/economics source does not smuggle wallet mutation, ledger mutation, storage writes, accounting export, roots, checkpoints, validators, bridges, or float math.
```

Important issue discovered and fixed:

```text
The first version of quickchain_preflight_economics_quote had an env-var race.
One async test expected legacy pricing, while another test set RON_STORAGE_ROC_ECONOMICS_PATH.
The fixed version uses an async ENV_LOCK so process-wide env mutations are serialized.
After the fix, all economics quote tests passed.
```

Important invariant:

```text
Quoting price is not payment.
Quoting price is not a hold.
Quoting price is not capture.
Quoting price is not receipt truth.
Quoting price is not ledger truth.
Quoting price is not QuickChain finality.
```

### 2.7 Bounded range/media boundary

We added `quickchain_preflight_range_media.rs`.

This test suite proves storage read paths serve only canonical b3 objects and bounded byte ranges.

The tests prove:

```text
GET/HEAD only accept canonical b3-shaped CIDs.
Malformed CID returns BAD_REQUEST.
Valid but missing CID returns NOT_FOUND.
Full object GET returns exact bytes.
Range GET returns PARTIAL_CONTENT with honest Content-Range.
Invalid ranges return RANGE_NOT_SATISFIABLE.
Suffix ranges are bounded and honest.
Free object read routes do not claim paid access or QuickChain authority.
```

Important invariant:

```text
Range support is transport convenience.
Range support is not DRM.
Range support is not ownership proof.
Range support is not paid-access proof.
Range support is not payment finality.
```

### 2.8 Observability/metrics boundary

We added `quickchain_preflight_observability.rs`.

This test suite proves metrics are low-cardinality and non-authoritative.

The tests prove:

```text
Paid-write metrics are exposed.
Metrics do not leak CIDs.
Metrics do not leak wallet receipt hashes.
Metrics do not leak wallet txids.
Metrics do not leak payer accounts.
Metrics do not leak escrow accounts.
Metrics do not expose balance truth.
Metrics do not expose root truth.
Metrics do not expose checkpoint/finality/validator/bridge authority.
Metric labels remain bounded, status-oriented labels.
```

Important invariant:

```text
Metrics are operational observations.
Metrics are not receipts.
Metrics are not balances.
Metrics are not roots.
Metrics are not proofs.
Metrics are not finality.
```

### 2.9 Docs/script preflight

We added and expanded:

```text
crates/svc-storage/docs/quickchain-preflight.md
crates/svc-storage/scripts/dev-quickchain-preflight.sh
crates/svc-storage/tests/quickchain_preflight_docs.rs
```

The script currently runs:

```text
cargo fmt -p svc-storage -- --check
cargo test -p svc-storage --test quickchain_preflight_boundary
cargo test -p svc-storage --test quickchain_preflight_b3_integrity
cargo test -p svc-storage --test quickchain_preflight_no_direct_mutation
cargo test -p svc-storage --test quickchain_preflight_paid_cache
cargo test -p svc-storage --test quickchain_preflight_economics_quote
cargo test -p svc-storage --test quickchain_preflight_range_media
cargo test -p svc-storage --test quickchain_preflight_observability
cargo test -p svc-storage --test quickchain_preflight_docs
cargo test -p svc-storage --all-targets
cargo clippy -p svc-storage --all-targets -- -D warnings
```

The docs test verifies the docs and script retain required QuickChain preflight markers.

Important invariant:

```text
This crate now has a repeatable local QuickChain preflight gate.
```

## 3. Files added or materially changed

### 3.1 Added docs

```text
crates/svc-storage/docs/quickchain-preflight.md
```

Purpose:

```text
Crate-local QuickChain Phase-0 doctrine.
Documents svc-storage boundaries.
Documents b3 integrity.
Documents no direct wallet/ledger mutation.
Documents paid/cache non-authority.
Documents bounded range/media behavior.
Documents observability/metrics non-authority.
Documents quote-only economics.
Documents no roots/checkpoints/validators/bridges/external settlement.
```

### 3.2 Added preflight script

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Purpose:

```text
One command to run the svc-storage QuickChain Phase-0 gate.
Runs fmt check.
Runs focused preflight suites.
Runs all-targets.
Runs clippy -D warnings.
Prints gate-passed marker.
```

### 3.3 Added QuickChain preflight tests

```text
crates/svc-storage/tests/quickchain_preflight_boundary.rs
crates/svc-storage/tests/quickchain_preflight_b3_integrity.rs
crates/svc-storage/tests/quickchain_preflight_no_direct_mutation.rs
crates/svc-storage/tests/quickchain_preflight_paid_cache.rs
crates/svc-storage/tests/quickchain_preflight_economics_quote.rs
crates/svc-storage/tests/quickchain_preflight_range_media.rs
crates/svc-storage/tests/quickchain_preflight_observability.rs
crates/svc-storage/tests/quickchain_preflight_docs.rs
```

Purpose:

```text
Prove storage does not become economic or chain authority.
Prove b3 is canonical truth.
Prove fake paid/cache claims do not unlock.
Prove quote path is read-only/integer-only.
Prove range/media path is bounded/non-authoritative.
Prove metrics are low-cardinality/non-authoritative.
Prove docs/script keep the preflight contract alive.
```

### 3.4 Changed storage read behavior

Likely changed/confirmed:

```text
crates/svc-storage/src/http/routes/get_object.rs
crates/svc-storage/src/storage/mod.rs
crates/svc-storage/tests/http_blackbox.rs
```

Result:

```text
GET validates b3 before lookup.
Malformed CID returns BAD_REQUEST.
Unknown valid b3 returns NOT_FOUND.
Range requests return honest 206/416 behavior.
MemoryStorage::get_range rejects invalid ranges instead of silently clamping.
```

Important invariant:

```text
Invalid ranges must not silently clamp.
Malformed CIDs must not be treated as ordinary authority-bearing object IDs.
```

### 3.5 Existing paid-write/economics/accounting paths validated

We did not rewrite the entire paid-storage implementation. Instead, we used tests to lock down the existing design.

Important existing/validated modules:

```text
src/http/routes/paid_object.rs
src/http/routes/paid_estimate.rs
src/policy/paid_write.rs
src/policy/economics.rs
src/policy/settlement.rs
src/accounting/mod.rs
src/accounting/exporter.rs
src/metrics.rs
```

Validated behavior:

```text
Paid write requires proof.
Wallet receipt mode can validate receipts over HTTP.
Wallet settlement mode calls svc-wallet capture/release through adapter.
Accounting export is usage/metering and opt-in.
Economics policy loads from checked-in ROC economics config when explicit.
Metrics remain low-cardinality.
```

## 4. Current svc-storage command set

To rerun the crate-local QuickChain preflight:

```bash
cd /Users/mymac/Desktop/RustyOnions
crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

To regenerate the crate code bundle:

```bash
cd /Users/mymac/Desktop/RustyOnions
bash scripts/make_crate_codex.sh -c svc-storage
```

To run only focused QuickChain tests manually:

```bash
cargo test -p svc-storage --test quickchain_preflight_boundary
cargo test -p svc-storage --test quickchain_preflight_b3_integrity
cargo test -p svc-storage --test quickchain_preflight_no_direct_mutation
cargo test -p svc-storage --test quickchain_preflight_paid_cache
cargo test -p svc-storage --test quickchain_preflight_economics_quote
cargo test -p svc-storage --test quickchain_preflight_range_media
cargo test -p svc-storage --test quickchain_preflight_observability
cargo test -p svc-storage --test quickchain_preflight_docs
```

To run full crate verification:

```bash
cargo fmt -p svc-storage -- --check
cargo test -p svc-storage --all-targets
cargo clippy -p svc-storage --all-targets -- -D warnings
```

## 5. What remains for svc-storage when we return

`svc-storage` is parked for current QuickChain Phase 0, but it is not “done forever.”

The remaining work falls into later categories.

## 5.1 Optional final Phase-0 polish: settlement-boundary guard

An optional final preflight test can still be added if not already applied:

```text
quickchain_preflight_settlement_boundary.rs
```

Purpose:

```text
Prove storage settlement planning is integer-only.
Prove capture cannot exceed hold.
Prove capture/release idempotency keys are deterministic safeguards only.
Prove wallet capture/release appears only through the explicit svc-wallet adapter.
Prove svc-storage does not become ledger/root/checkpoint/validator/bridge/external-settlement authority.
```

This is useful final polish but not a major blocker because existing `paid_write_settlement.rs`, no-direct-mutation tests, and source guards already cover much of the doctrine.

If this optional guard is applied later, also update:

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
crates/svc-storage/tests/quickchain_preflight_docs.rs
crates/svc-storage/docs/quickchain-preflight.md
```

## 5.2 Production storage backend work

Current storage is heavily in-memory/dev/smoke oriented.

Later production work:

```text
durable filesystem/object backend
atomic writes
crash-safe writes
fsync/durability policy
content-addressed layout
garbage collection policy
tenant quotas
residency policy
repair path
replication path
range-heavy media optimization
persistent metadata if needed
consistent error taxonomy for durable backend
```

Important warning:

```text
Durability work must not add ledger or wallet authority.
Persistent storage is still byte truth, not economic truth.
```

## 5.3 Media/range serving maturity

Later media work:

```text
large media range performance
segment/rendition-aware serving
verified b3 per rendition
strong ETag consistency
content-type handling
honest range failure behavior
bounded body sizes
backpressure
streaming responses instead of full buffering where appropriate
```

Important warning:

```text
Media serving is not DRM.
Range serving is not paid authorization.
Rendition metadata is not ownership proof.
```

## 5.4 Paid-storage production hardening

Later paid-storage work:

```text
move dev-header mode further behind explicit dev-only configuration
tighten production wallet-receipt mode
stronger wallet egress auth/capability
shared service-to-service auth/TLS
clear settlement retries
durable idempotency if storage owns any retry queue
better timeout/failure taxonomy for wallet/accounting dependencies
operator diagnostics for settlement/export modes
```

Important warning:

```text
svc-storage can request/check/settle through svc-wallet only where explicitly designed.
svc-storage must not directly mutate ron-ledger.
svc-storage must not expose wallet mutation endpoints.
svc-storage must not invent backend receipts.
```

## 5.5 Accounting export maturity

Current accounting export is usage-event shaped and opt-in.

Later work:

```text
align DTOs fully with ron-accounting ingest when that service API hardens
add stronger retry/backoff if desired
add export queue only if needed
avoid duplicate usage events
include better operator status without leaking private IDs in metrics
```

Important warning:

```text
Accounting export is metering.
Accounting is not balance truth.
Accounting failure must not turn into hidden rollback unless the wallet/storage operation explicitly requires it.
```

## 5.6 Observability maturity

Current metrics boundary is good for Phase 0.

Later work:

```text
more status labels if bounded
latency histograms
body-size histograms
range request counters
storage backend error counters
accounting export latency
wallet receipt lookup latency
settlement latency
readiness/dependency status
```

Important warning:

```text
Metrics must stay low-cardinality.
No CIDs in labels.
No account IDs in labels.
No txids in labels.
No receipt hashes in labels.
No root/checkpoint/validator/bridge claims.
```

## 5.7 Docs/runbook polish

Later docs work:

```text
operator mode table
dev-header vs wallet-receipt mode explanation
settlement disabled vs wallet-capture mode explanation
accounting export disabled vs HTTP mode explanation
paid write flow diagram
range/media serving notes
threat model section for fake cache/fake receipt/fake CID attempts
production hardening checklist
```

Important warning:

```text
Docs must keep saying storage is not ledger/wallet/chain authority.
```

## 5.8 Future QuickChain proof/archive/DA integration

This is not authorized now, but later QuickChain may need storage to participate in proof/archive/DA workflows.

Future-only concepts:

```text
store canonical checkpoint artifacts
serve immutable receipt bundles
serve canonical byte vectors
serve archive records
serve challenge data
serve proof blobs
serve DA fallback material
```

Still forbidden until later gates:

```text
producing roots inside svc-storage
deciding finality inside svc-storage
validating consensus inside svc-storage
pruning without archive/challenge fallback
creating bridge settlement
creating external anchor authority
```

The correct future shape is likely:

```text
QuickChain/root-producing crate creates canonical artifacts.
svc-storage stores immutable bytes by b3.
svc-gateway/omnigate serve/hydrate artifacts through policy.
validators/verifiers verify artifacts elsewhere.
```

Storage should stay a byte service.

## 6. What not to add to svc-storage

Do not add:

```text
root production
state roots
receipt roots
checkpoint roots
validator signatures
validator sets
validator RPC
chain state
consensus state
staking
liquidity
ROX
Solana
public bridge
external L2 settlement
external DA settlement authority
public anchor mutation
wallet issue routes
wallet transfer routes
wallet burn routes
ledger hold routes
ledger capture routes
ledger release routes
ledger balance mutation
fake receipt fields
fake balance fields
fake finality fields
fake paid=true authority
cache-only paid unlock
raw engagement payout authority
DB-order roots
wall-clock roots
```

Do not make storage decide:

```text
who paid
who owns balance
whether a receipt is final
whether a wallet operation is durable
whether a ledger operation is durable
whether ROC was captured
whether QuickChain finalized something
whether a validator approved something
whether a bridge settled something
whether an external anchor settled something
```

## 7. Known caveats and code smells to remember

### 7.1 Legacy/scaffold storage modules

The bundle contains scaffold/older-looking modules under storage such as:

```text
src/storage/cache.rs
src/storage/fs.rs
src/storage/cas.rs
src/storage/compression.rs
src/storage/erasure.rs
src/storage/hedged.rs
src/storage/placement.rs
src/storage/repair.rs
src/storage/replication.rs
```

Some of these appear to be scaffolds or not currently wired into the active `storage/mod.rs` trait.

Because all-targets and clippy pass, they are not currently blocking.

When we return:

```text
Do not blindly wire these modules into production.
First reconcile trait shapes, error variants, b3 validation, range behavior, and QuickChain boundaries.
```

### 7.2 Dev-header paid mode

`dev-header` exists for local beta/dev proof.

When returning:

```text
Keep dev-header explicit.
Do not present dev-header as production trust.
Do not let dev-header become silent paid unlock.
Do not let dev-header bypass wallet-receipt mode in production.
```

### 7.3 Settlement adapter presence

`svc-storage` has a wallet settlement adapter for explicit capture/release.

Important distinction:

```text
Calling svc-wallet through explicit adapter is not the same as mutating ron-ledger directly.
The adapter must stay narrow and explicit.
Wallet remains mutation front-door.
Ledger remains truth.
```

### 7.4 Accounting export includes CID/wallet_txid in payload

Accounting export payloads may include fields such as CID and wallet txid for accounting ingestion.

Important distinction:

```text
Payload content to accounting is not the same as Prometheus metric labels.
Metrics must remain low-cardinality and non-sensitive.
Accounting remains metering, not balance truth.
```

## 8. Current crate completion estimate

Current status:

```text
svc-storage QuickChain Phase-0/preflight: ~92–94%
```

Why this is high:

```text
b3 integrity tested
router boundary tested
no direct mutation tested
paid/cache boundary tested
quote-only economics tested
range/media path tested
observability/metrics boundary tested
docs/script tested
full crate all-targets passed
clippy -D warnings passed
```

Why it is not 100%:

```text
optional settlement-boundary guard may still be added if not already applied
production durable storage backend is not complete
future proof/archive/DA integration is not started
future QuickChain artifacts are intentionally not implemented
gateway/omnigate paid-access enforcement still needs separate crate work
```

Full future `svc-storage` QuickChain role:

```text
~42–48%
```

Reason:

```text
The Phase-0 safety boundaries are strong, but future full QuickChain will eventually require durable artifact storage, archive/challenge/DA integration, production storage hardening, and cross-service gateway/omnigate enforcement.
```

## 9. Recommended next crate

The next crate should be:

```text
svc-gateway
```

Why:

```text
svc-gateway is the public boundary.
It sits in front of storage/omnigate/wallet-facing flows.
It must reject fake receipts, fake balances, fake roots, fake finality, and client-supplied unlock claims.
It must not mutate ledger directly.
It must not mint/transfer/burn/hold/capture/release ROC directly.
It must not become cache authority.
It must route paid access through backend truth.
```

Expected `svc-gateway` Phase-0 tests:

```text
quickchain_preflight_boundary
quickchain_preflight_no_direct_mutation
quickchain_preflight_paid_access
quickchain_preflight_no_fake_receipts
quickchain_preflight_cache_boundary
quickchain_preflight_docs
```

Expected `svc-gateway` docs/script:

```text
crates/svc-gateway/docs/quickchain-preflight.md
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
```

## 10. Cross-crate mental model after parking svc-storage

Keep this model active:

```text
ron-proto defines DTOs.
ron-ledger is economic truth.
svc-wallet is mutation front-door.
ron-accounting measures/snapshots.
svc-rewarder plans payouts.
svc-storage stores/serves bytes by b3.
svc-gateway is public boundary.
omnigate hydrates product views.
CrabLink displays backend-derived truth.
QuickChain later verifies deterministic roots/proofs after gates are green.
```

Payment/access loop remains:

```text
prepare/quote
explicit confirmation
backend wallet path
backend receipt
backend admission/unlock
storage serves bytes by b3/range
gateway/omnigate hydrate/render
client caches display-only proof
balance refresh comes from backend wallet/ledger truth
```

## 11. One-paragraph handoff

We are parking `svc-storage` after a successful QuickChain Phase-0/preflight hardening pass. The crate now has a repeatable `dev-quickchain-preflight.sh` gate, crate-local QuickChain docs, and focused tests proving storage remains byte/object infrastructure only: canonical b3 integrity, no fake receipts/balances/finality, no direct wallet/ledger mutation, no cache-only paid unlock, quote-only integer economics, bounded range/media reads, and low-cardinality non-authoritative metrics. Full `cargo test -p svc-storage --all-targets` and `cargo clippy -p svc-storage --all-targets -- -D warnings` passed. `svc-storage` is approximately 92–94% complete for current QuickChain Phase-0/preflight and should be considered parked. Remaining later work is production storage durability, optional settlement-boundary polish, stronger production wallet/accounting transport, media/range performance, docs/runbook maturity, and future archive/challenge/DA artifact storage once QuickChain gates allow it. The next crate is `svc-gateway`, where we need to prove the public boundary enforces paid access using backend truth without becoming wallet, ledger, receipt, root, bridge, validator, cache, or finality authority.

## END NOTE - JUNE 15 2026 - svc-storage QuickChain Phase-0 / Preflight Parking Notes


### END NOTE - JUNE 14 2026 - 21:55 CST




### BEGIN NOTE - JUNE 17 2026 - 13:10 CST

Confirmed: `svc-storage` is fully parked. The terminal shows the storage parking gate passed, then the forced codebundle regeneration wrote `75` files for `svc-rewarder` and `104` files for `svc-storage`. 

Here are the comprehensive notes to drop into the crate notes.

# QuickChain Phase-0 Crate Notes — `svc-rewarder` + `svc-storage`

## Status Summary

This notes entry records the completed QuickChain Phase-0 / preflight sweep for the paired crates:

* `crates/svc-rewarder`
* `crates/svc-storage`

Both crates have now been brought into the QuickChain safety/preflight doctrine for the current internal-ROC buildout.

This work does **not** make either crate a QuickChain runtime, validator, bridge, root producer, settlement layer, public-chain component, public finality source, or external-settlement participant.

The purpose of this sweep was to harden each crate’s current RustyOnions role before later QuickChain root/proof/checkpoint work begins elsewhere.

Current doctrine remains:

* `svc-wallet` is the economic mutation front-door.
* `ron-ledger` is durable replayable balance truth.
* `ron-accounting` produces accounting snapshots and reports, but is not balance truth.
* `svc-rewarder` plans deterministic payouts but must not mutate ledger truth directly.
* `svc-storage` stores and serves bytes by canonical b3, but must not become wallet, ledger, finality, root, bridge, validator, or paid-access authority.
* QuickChain is future settlement infrastructure, not current runtime authority.
* ROC remains internal.
* No ROX, Solana, bridge, staking, liquidity, external settlement, validators, anchors, or public-chain authority were added.

---

# Part I — `svc-rewarder`

## Crate Role

`svc-rewarder` is the deterministic reward planning service.

Its proper responsibility is to consume accounting/policy-style inputs, compute payout plans deterministically, expose manifests/intents, and route any actual economic mutation through `svc-wallet`.

It is **not** the ledger.

It is **not** the wallet.

It is **not** QuickChain.

It is **not** a validator.

It is **not** a settlement authority.

It must not directly issue, mint, transfer, burn, hold, capture, release, or mutate ROC balances outside the wallet/ledger path.

The essential boundary is:

`ron-accounting snapshots/reports -> svc-rewarder deterministic payout planning -> svc-wallet mutation front-door -> ron-ledger durable truth`

## QuickChain Phase-0 Purpose for `svc-rewarder`

The Phase-0 work for `svc-rewarder` was designed to prevent a dangerous architectural drift:

A reward service that sees “engagement,” “usage,” or “contribution” data could accidentally become a hidden minting path, a direct payout authority, or a proto-consensus authority. The preflight sweep prevents that.

The goal was to prove that `svc-rewarder` can participate in the internal ROC value loop without becoming:

* direct ledger mutation authority;
* raw engagement payout authority;
* fake protocol reward minting authority;
* settlement authority;
* root/checkpoint producer;
* validator;
* bridge;
* external-token integration point.

The crate is now framed as a deterministic payout planner with explicit boundaries.

## Key Doctrine Added / Enforced

The rewarder doctrine now makes the following clear:

* Raw engagement must never directly mint or allocate protocol ROC.
* Rewarder output is planning/intent material, not final ledger truth.
* Rewarder may prepare deterministic manifests and payout plans.
* Rewarder may emit wallet issue/capture/intents only through explicit wallet-facing seams.
* Wallet remains the mutation front-door.
* Ledger remains the durable replayable truth.
* Accounting remains upstream input/snapshot/report material, not balance truth.
* Funding provenance must be explicit.
* Protocol-pool and governance-style funding sources must not be accepted casually from unsigned/unverified policy material.
* Integer minor-unit accounting is required.
* Float money math is forbidden.
* Payout conservation must hold.
* Rewarder may not invent receipts, balances, roots, finality, checkpoints, validators, or bridges.
* Rewarder must not contain Solana, ROX, external settlement, staking, liquidity, validator-market, or public-chain logic.

## QuickChain Test Coverage Added / Verified

The focused QuickChain preflight suite for `svc-rewarder` consists of seven test targets:

1. `quickchain_preflight_boundary`
2. `quickchain_preflight_docs`
3. `quickchain_preflight_funding_source`
4. `quickchain_preflight_no_direct_mutation`
5. `quickchain_preflight_raw_engagement`
6. `quickchain_preflight_replay_no_double_issue`
7. `quickchain_tooling_boundary`

These tests collectively protect the crate’s intended role.

### `quickchain_preflight_boundary`

Purpose:

* Confirms rewarder routes and public shapes do not claim QuickChain authority.
* Guards against accidental exposure of chain/runtime/validator/finality behavior.
* Ensures rewarder remains a service that computes reward plans, not a settlement chain.

Boundary concepts covered:

* no state roots;
* no receipt roots;
* no checkpoints;
* no validators;
* no finality claims;
* no bridge claims;
* no external settlement claims;
* no balance truth claims;
* no direct ledger truth exposure.

### `quickchain_preflight_docs`

Purpose:

* Ensures the crate has explicit written QuickChain preflight doctrine.
* Prevents future work from silently deleting the boundary notes.
* Keeps the crate’s role readable to the next developer/session.

Docs are expected to explain:

* rewarder plans payouts;
* wallet mutates;
* ledger is truth;
* accounting is input/snapshot/report material;
* raw engagement is not direct protocol ROC authority;
* QuickChain is future infrastructure, not a current runtime path;
* no roots/checkpoints/validators/bridges/external settlement are allowed in this crate.

### `quickchain_preflight_funding_source`

Purpose:

* Guards the reward funding model.
* Ensures reward plans declare explicit funding provenance.
* Prevents silent protocol-pool allocation from arbitrary raw engagement.
* Prevents unsigned/unverified policy material from becoming protocol ROC authority.

Key protected idea:

`funding_source` is provenance, not mutation authority.

Funding-source examples:

* protocol pool;
* advertiser budget;
* creator pool;
* sponsor budget;
* governance budget.

Important doctrine:

* Protocol-pool and governance-budget style funding should require stronger policy/signed verification.
* Raw engagement events should not become direct issuance.
* Rewarder should plan payouts under declared policy, not invent an economic source.

### `quickchain_preflight_no_direct_mutation`

Purpose:

* Confirms rewarder does not mutate ledger balances directly.
* Confirms rewarder does not become a wallet replacement.
* Confirms wallet-facing mutation paths are explicit.
* Confirms direct ledger mutation symbols or routes do not creep into rewarder.

Protected boundary:

`svc-rewarder -> svc-wallet -> ron-ledger`

not:

`svc-rewarder -> ron-ledger direct mutation`

not:

`svc-rewarder -> hidden balance mutation`

not:

`svc-rewarder -> fake receipt`

### `quickchain_preflight_raw_engagement`

Purpose:

* Prevents raw usage/engagement data from directly producing protocol ROC payouts.
* Forces engagement to pass through accounting/policy/reward planning.
* Blocks bot-farm-friendly designs where views/clicks alone become protocol mint authority.
* Supports the longer-term doctrine that useful node/provider work can be rewardable, but only through deterministic policy and accounting.

Protected idea:

Raw engagement can be analytics, metering, or accounting input. It is not direct payout authority.

Event-class alignment:

* `analytics_only` events do not pay directly.
* `metering` events may feed accounting.
* `proof_eligible` events may become eligible only after policy/accounting verification.
* `ad_budgeted` events are budget-constrained, not protocol-mint authority.
* `economic_receipt` events reflect backend wallet/ledger truth, not rewarder invention.

### `quickchain_preflight_replay_no_double_issue`

Purpose:

* Ensures reward planning and settlement-intent behavior remains idempotent.
* Prevents replay from producing duplicate issue effects.
* Keeps epoch/policy/input combinations deterministic.
* Reinforces that retry keys and operation identity must not be confused.

Important identity doctrine:

* `operation_id` is durable backend-assigned ledger-operation identity.
* `idempotency_key` is retry protection, not authority.
* Rewarder run keys and manifests must be deterministic for the same sealed inputs.
* Replay must not cause double issuance.

### `quickchain_tooling_boundary`

Purpose:

* Confirms preflight tooling is Bash/cargo-only.
* Confirms no Python helper tooling is checked into the crate for this QuickChain sweep.
* Confirms the exhaustive preflight script discovers focused QuickChain tests dynamically.
* Confirms the parking script delegates to the exhaustive preflight gate.

This protects the workflow from stale hardcoded lists and tool drift.

## Scripts Added / Validated

### `crates/svc-rewarder/scripts/dev-quickchain-preflight.sh`

Purpose:

Runs the exhaustive QuickChain preflight gate for `svc-rewarder`.

The script verifies:

* required docs exist;
* no checked-in Python helper files exist under `crates/svc-rewarder`;
* formatting is clean with `cargo fmt -p svc-rewarder -- --check`;
* focused QuickChain tests are discovered dynamically from `crates/svc-rewarder/tests/quickchain*.rs`;
* every discovered focused QuickChain test target is run;
* `cargo test -p svc-rewarder --all-targets` passes;
* `cargo clippy -p svc-rewarder --all-targets -- -D warnings` passes;
* forbidden-scope marker is printed;
* final dynamic test-count marker is printed.

Important: the script should remain dynamic. Do not replace it with a stale hardcoded test list.

### `crates/svc-rewarder/scripts/dev-quickchain-park.sh`

Purpose:

Parking gate for the crate.

This script validates required preflight files exist, then delegates to:

`crates/svc-rewarder/scripts/dev-quickchain-preflight.sh`

Expected final parking doctrine:

If the parking gate passes, `svc-rewarder` is parked for the current QuickChain Phase-0 sweep.

## Code/Architecture Areas Now Covered

### Deterministic Reward Computation

Reward computation is treated as pure planning.

Important characteristics:

* no floats;
* checked arithmetic;
* integer minor units;
* deterministic score/payout ordering;
* canonical account sorting;
* conservation checks;
* residual calculation;
* dry-run support;
* explicit egress result modeling.

### Amount Handling

The rewarder uses integer minor units through an `AmountMinor` style.

Important properties:

* serialized as decimal strings at JSON boundaries;
* no floating-point ROC;
* checked add/sub/mul paths;
* overflow/underflow lead to quarantine-style errors;
* zero/negative payouts do not escape as valid payout entries.

### Payout Conservation

The rewarder validates:

* sum of payouts must not exceed pool;
* residual equals pool minus payouts;
* zero payout entries are filtered/rejected;
* empty account destinations are rejected;
* arithmetic overflow quarantines the run.

### Run Keys / Idempotency

Rewarder run identity uses deterministic input material such as:

* epoch id;
* policy hash;
* inputs CID;
* idempotency salt/domain separator.

The run key protects replay semantics and makes repeated computation stable for the same sealed inputs.

Important caution:

Run keys and idempotency keys are not ledger authority.

They are determinism/retry controls only.

### Policy and Funding Source

Reward policy now has stronger structure:

* policy id;
* policy hash;
* signed/verified flag;
* explicit funding source;
* maximum payout cap;
* minimum payout filter;
* basis-point weight;
* rounding mode.

Important doctrine:

* funding source is provenance, not mutation authority;
* protocol/governance-style funding requires stronger trust;
* rewarder must not infer protocol mint authority from engagement alone.

### Wallet Mutation Boundary

Rewarder may prepare wallet issue/intents, but must not mutate ledger directly.

Any real economic mutation must go through `svc-wallet`.

This preserves the value-plane boundary:

`svc-rewarder` plans.

`svc-wallet` mutates.

`ron-ledger` records truth.

### Accounting Input Boundary

Rewarder consumes accounting snapshots or accounting-derived input material.

Accounting is not balance truth.

Rewarder must not treat accounting counters as final wallet balance truth.

Accounting can say what happened in a metering/reporting sense. Wallet/ledger decides economic mutation.

### Public HTTP Boundary

Rewarder routes should not expose:

* balance truth;
* root truth;
* receipt root;
* state root;
* checkpoint;
* finality;
* validator signatures;
* bridge settlement;
* external anchor;
* staking/liquidity behavior.

Routes are for health/readiness/metrics/version, compute, inspect, settlement preview/emission through wallet-facing seams, and related operational behavior.

## `svc-rewarder` Validation Status

The crate has been validated through its focused QuickChain preflight gate.

The relevant gate includes:

* focused QuickChain test targets;
* all-targets test;
* clippy with warnings denied;
* Bash/cargo-only tooling boundary;
* no Python helper files;
* forbidden-scope marker.

Current state:

`svc-rewarder` is parked for the current QuickChain Phase-0 sweep.

## What Remains for `svc-rewarder`

The crate is parked for Phase-0/preflight, but not “done forever.”

Future work should stay ordered and gated.

### Near-Term Remaining Work

1. Keep rewarder aligned with future `ron-accounting` sealed snapshot format.

When accounting snapshots become more formal, rewarder should consume them without weakening the boundary.

2. Keep payout planning deterministic as schemas evolve.

If new contribution counters or scoring policies are added, they must preserve deterministic ordering, integer math, and bounded overflow behavior.

3. Add stronger policy registry integration later.

Current policy validation is enough for Phase-0. Future work may integrate signed policy registry material from `ron-policy`, but must not make rewarder itself policy authority.

4. Improve production wallet egress hardening later.

Rewarder can preview/emit through wallet-facing seams, but production egress should continue tightening:

* auth scopes;
* retry behavior;
* idempotency;
* failure classification;
* replay prevention;
* receipt handling;
* audit logging.

5. Add formal reward artifact vectors later.

Rewarder output vectors can be useful later, but must follow the vector doctrine:

`sketch -> locked_bytes -> locked_hash`

No fake hashes.

No placeholder commitments.

No roots until canonical bytes and root engines are approved.

### Long-Term Remaining Work

1. Reward roots are future work.

`reward_root` or reward manifest commitments should not be treated as QuickChain roots until:

* canonical bytes are frozen;
* domain separators are frozen;
* BLAKE3 preimage framing is frozen;
* golden vectors are locked;
* independent verifier can reproduce bytes and hashes.

2. No validator/committee behavior belongs here.

Even later, rewarder should not become the validator set. It can provide reward artifacts consumed by other systems.

3. No bridge/external settlement logic belongs here.

No ROX, Solana, L2, bridge, staking, liquidity, exchange-facing, or external anchor behavior should be added to rewarder during internal ROC proving.

4. Raw engagement must stay non-authoritative.

If CrabLink adds views, prompts, AI usage, creator engagement, ads, storage work, node uptime, or other signals, rewarder should only consume policy/accounting-validated inputs. It should never mint from raw activity alone.

---

# Part II — `svc-storage`

## Crate Role

`svc-storage` is the content-addressed byte/object service.

Its proper responsibility is:

* store bytes;
* derive canonical b3 content IDs from bytes;
* serve bytes by canonical b3;
* serve bounded ranges for media;
* support paid-write admission after backend-derived proof;
* produce usage/metering signals for accounting;
* expose observability without leaking authority;
* remain storage infrastructure, not economic truth.

`svc-storage` is not:

* wallet;
* ledger;
* accounting truth;
* rewarder;
* gateway authority;
* omnigate authority;
* QuickChain authority;
* root/checkpoint producer;
* validator;
* bridge;
* finality source;
* external-settlement participant.

## QuickChain Phase-0 Purpose for `svc-storage`

The Phase-0 work for `svc-storage` was designed to prevent storage from becoming a hidden economic authority.

Storage is near paid content, byte access, cache, and media. That makes it dangerous if boundaries are unclear.

The preflight sweep proves:

* storage only stores/serves bytes;
* b3 hashes identify bytes only;
* paid writes require backend-derived proof;
* cache cannot unlock paid content alone;
* storage does not invent wallet receipts;
* storage does not claim finality;
* storage does not mutate balances directly;
* settlement/capture/release, where present, is explicitly through wallet-facing adapter seams;
* accounting export is metering, not balance truth;
* observability does not leak high-cardinality or authority-bearing data;
* media reads are bounded and canonical-b3 based.

## Key Doctrine Added / Enforced

The storage doctrine now makes the following clear:

* `svc-storage` remains content-addressed byte/object infrastructure.
* b3 hashes identify bytes only.
* b3 hashes are not payment proofs.
* b3 hashes are not wallet receipts.
* b3 hashes are not account balances.
* b3 hashes are not ledger commitments.
* b3 hashes are not QuickChain roots.
* b3 hashes are not finality proofs.
* b3 hashes are not bridge proofs.
* `crab://` navigation is not storage authority.
* cache entries are not paid-access authority.
* client-side references are not economic truth.
* storage must not unlock paid content from cache alone.
* paid estimate is quote-only.
* paid write requires proof.
* accounting export is usage/metering only.
* settlement behavior is opt-in and must go through wallet front-door seams.
* no fake balances.
* no fake receipts.
* no fake finality.
* no roots.
* no validators.
* no bridges.
* no external settlement.

## QuickChain Test Coverage Added / Verified

The focused QuickChain preflight suite for `svc-storage` consists of ten test targets:

1. `quickchain_preflight_b3_integrity`
2. `quickchain_preflight_boundary`
3. `quickchain_preflight_docs`
4. `quickchain_preflight_economics_quote`
5. `quickchain_preflight_no_direct_mutation`
6. `quickchain_preflight_observability`
7. `quickchain_preflight_paid_cache`
8. `quickchain_preflight_range_media`
9. `quickchain_preflight_settlement_boundary`
10. `quickchain_tooling_boundary`

The exhaustive preflight gate dynamically discovers and runs every `quickchain*.rs` test target.

### `quickchain_preflight_b3_integrity`

Purpose:

* Confirms object ingest derives canonical b3 from actual bytes.
* Confirms callers cannot retrieve bytes under fake or noncanonical CIDs.
* Protects the fundamental storage invariant: b3 hash truth is byte truth.

Boundary protected:

A b3 hash means “these bytes,” not “this payment,” “this balance,” “this proof,” or “this root.”

### `quickchain_preflight_boundary`

Purpose:

* Confirms the router exposes storage routes, not QuickChain authority routes.
* Confirms public response shapes do not claim balance, receipt root, state root, checkpoint, validator, bridge, settlement, or finality truth.

Protected principle:

Storage responses can describe storage admission and byte retrieval. They must not pretend to be wallet/ledger/QuickChain outputs.

### `quickchain_preflight_docs`

Purpose:

* Ensures the crate has explicit written QuickChain Phase-0 doctrine.
* Confirms the docs contain RO headers and a complete test contract.
* Confirms docs state storage’s plain boundary phrases.

Important docs content includes:

* `svc-storage remains content-addressed byte/object infrastructure`
* `b3 hashes identify bytes only`
* `svc-wallet = economic mutation front-door`
* `ron-ledger = durable replayable truth`
* `cache must not decide paid access by itself`
* `no fake balances`
* `no fake receipts`
* `no roots`
* `no validators`
* `no bridges`
* `no external settlement`

The “plain scanner boundary phrases” section was added because tests and safety scanners need exact plain text. This is intentional and should not be removed casually.

### `quickchain_preflight_economics_quote`

Purpose:

* Confirms paid estimate is quote-only.
* Confirms pricing uses integer minor units.
* Confirms checked-in ROC economics policy can quote paid storage without wallet or ledger mutation.
* Confirms quote/economics sources do not smuggle mutation or chain authority.

Protected principle:

A quote is not a hold.

A quote is not a capture.

A quote is not a receipt.

A quote is not finality.

A quote is not ledger mutation.

### `quickchain_preflight_no_direct_mutation`

Purpose:

* Confirms production dependencies do not include wallet or ledger mutation crates in the wrong way.
* Confirms storage router does not expose wallet or ledger mutation endpoints.
* Confirms accounting export is metering, not balance truth.
* Confirms wallet capture/release appears only inside explicit settlement adapter code.

Protected boundary:

`svc-storage` may verify/admit bytes under a paid proof and may call explicit wallet-facing settlement adapters where configured, but it must not become wallet or ledger itself.

### `quickchain_preflight_observability`

Purpose:

* Confirms metrics source keeps labels low-cardinality and non-authoritative.
* Confirms metrics do not expose CIDs, wallet receipts, accounts, balances, roots, validators, bridge data, anchors, or finality claims.

Protected principle:

Observability is for operational health. It is not a public ledger, explorer, receipt database, chain authority, or account-balance API.

### `quickchain_preflight_paid_cache`

Purpose:

* Confirms fake cache or fake paid headers do not unlock absent objects.
* Confirms paid write rejects without backend-derived proof and does not cache bytes.
* Confirms dev-header paid write response is labeled as storage admission, not finality.

Protected principle:

Cache can speed up storage. Cache cannot authorize paid access.

Storage admission is not finality.

Dev headers are dev/test admission material, not production economic truth.

### `quickchain_preflight_range_media`

Purpose:

* Confirms read path serves only canonical b3 and bounded ranges.
* Confirms free object read routes do not claim paid access or QuickChain authority.
* Protects the media boundary.

Important for CrabLink/Tauri:

Large media must be bounded and honest. The storage path should support range/segment access and should not pipe full large media through inappropriate command results. Each rendition should own its own b3.

### `quickchain_preflight_settlement_boundary`

Purpose:

* Confirms settlement plan rejects overcapture, zero capture, and escrow self-payee.
* Confirms settlement plan is integer-bounded and deterministic without roots or finality.
* Confirms settlement source uses wallet front-door only and no chain authority.

Protected principle:

Storage settlement is wallet-facing and bounded. It is not QuickChain finality, not bridge settlement, not external settlement, and not a root/checkpoint system.

### `quickchain_tooling_boundary`

Purpose:

* Confirms the storage preflight script is Bash/cargo-only.
* Confirms no Python helper files are checked into `svc-storage`.
* Confirms the full gate is preserved.
* Confirms the script dynamically discovers all `quickchain*.rs` targets.
* Confirms the park script delegates to the exhaustive preflight gate.

This protects the workflow from stale hardcoded lists and accidental tool drift.

## Scripts Added / Validated

### `crates/svc-storage/scripts/dev-quickchain-preflight.sh`

Purpose:

Runs the exhaustive QuickChain preflight gate for `svc-storage`.

The script verifies:

* docs exist;
* no checked-in Python helper files under `crates/svc-storage`;
* formatting is clean with `cargo fmt -p svc-storage -- --check`;
* focused QuickChain tests are discovered dynamically from `crates/svc-storage/tests/quickchain*.rs`;
* every discovered focused test is run;
* `cargo test -p svc-storage --all-targets` passes;
* `cargo clippy -p svc-storage --all-targets -- -D warnings` passes;
* forbidden-scope marker is printed;
* final dynamic test-count marker is printed.

Expected final marker:

`== svc-storage quickchain exhaustive preflight gate passed: tests=10 ==`

This marker has now been observed.

### `crates/svc-storage/scripts/dev-quickchain-park.sh`

Purpose:

Parking gate for the crate.

It validates required files exist and delegates to:

`crates/svc-storage/scripts/dev-quickchain-preflight.sh`

Expected final marker:

`== svc-storage QuickChain parking gate passed ==`

This marker has now been observed.

## Code/Architecture Areas Now Covered

### B3 Integrity

Storage derives b3 from bytes.

The canonical form is:

`b3:<64 lowercase hex>`

Storage must reject fake, malformed, or noncanonical CIDs where appropriate.

This protects content addressing and keeps b3 as truth for bytes only.

### Free CAS Object Routes

Free/dev object routes are still available for basic storage operations.

These routes must not claim paid unlock, wallet receipt, ledger truth, state root, receipt root, checkpoint, validator approval, bridge settlement, or finality.

### Paid Estimate

The paid estimate route is read-only.

It can compute side-effect-free pricing.

It must not:

* create a wallet hold;
* capture funds;
* release funds;
* mutate ledger;
* store bytes;
* export accounting events;
* claim a receipt;
* claim finality.

### Paid Write Admission

Paid write requires backend-derived proof.

The current architecture supports dev-header and wallet-receipt style verification seams, with explicit modes.

The doctrine is:

* dev-header mode is dev/test admission only;
* wallet-receipt mode is production-shaped;
* disabled mode fails closed;
* paid writes without proof reject and do not store bytes;
* fake headers cannot unlock absent objects;
* paid responses must be labeled as storage admission, not finality.

### Wallet Receipt Verifier

The storage tests cover wallet receipt contract behavior, including rejection of:

* wrong operation;
* wrong asset;
* wrong payer;
* wrong escrow;
* wrong amount;
* zero/non-integer amount;
* bad receipt hash;
* missing receipt;
* missing payer/escrow;
* malformed proof material.

This supports paid storage without allowing storage to invent receipts.

### Settlement Adapter Boundary

Storage contains explicit paid-storage settlement planning/adapter seams.

This is not direct ledger mutation.

Important boundaries:

* settlement mode is explicit;
* default is safe/disabled where applicable;
* wallet capture/release goes through wallet-facing HTTP client seams;
* overcapture rejects;
* zero capture rejects;
* escrow self-payee rejects;
* settlement plan is deterministic and integer-bounded;
* settlement does not claim roots/finality.

This is allowed as an explicit backend wallet path, not QuickChain authority.

### Accounting Export

Storage may export usage events to accounting.

Accounting export is metering/reporting, not balance truth.

Export failure does not become ledger mutation.

Usage events should remain bounded and not include secret or authority-bearing data.

Important exported concepts:

* bytes stored;
* request success;
* optional pin seconds;
* tenant/subject/region/route metadata;
* deterministic idempotency for accounting export batch.

The export must not carry:

* private keys;
* wallet secrets;
* full object body bytes;
* balances;
* chain roots;
* finality claims.

### Observability

Storage metrics must remain low-cardinality and non-authoritative.

Metrics must not expose:

* CIDs;
* accounts;
* wallet receipts;
* balances;
* roots;
* validators;
* bridges;
* anchors;
* finality claims.

Metrics can report operational status, accepted/rejected counts, byte totals, accounting export status, and similar bounded labels.

### Range Media

Storage supports bounded range media reads.

This aligns with CrabLink/Tauri media doctrine:

* no full-file large media through command-result style paths;
* prefer range/segment access;
* each rendition owns its own b3;
* no DRM or anti-rip claims;
* b3 verifies bytes;
* cache cannot unlock paid content alone.

## `svc-storage` Validation Status

The following have now passed:

* `quickchain_preflight_docs`
* `quickchain_tooling_boundary`
* all ten focused QuickChain preflight targets
* `cargo test -p svc-storage --all-targets`
* `cargo clippy -p svc-storage --all-targets -- -D warnings`
* `crates/svc-storage/scripts/dev-quickchain-preflight.sh`
* `crates/svc-storage/scripts/dev-quickchain-park.sh`

The final exhaustive marker was observed:

`== svc-storage quickchain exhaustive preflight gate passed: tests=10 ==`

The final parking marker was observed:

`== svc-storage QuickChain parking gate passed ==`

Current state:

`svc-storage` is parked for the current QuickChain Phase-0 sweep.

## What Remains for `svc-storage`

The crate is parked for Phase-0/preflight, but not “done forever.”

Future work should remain carefully bounded.

### Near-Term Remaining Work

1. Keep paid storage in the wallet-front-door path.

Any storage payment path should continue to use backend wallet/ledger truth.

2. Tighten wallet receipt production mode.

The wallet-receipt mode should remain the production-shaped path. Future hardening can improve auth, timeout behavior, replay protection, receipt lookup, and error classification.

3. Keep dev-header mode explicitly dev/test only.

Do not allow dev-header proof to become production economic truth.

4. Keep accounting export as metering.

If accounting ingestion evolves, storage should export usage/metering events only. It should not emit balances or mutate accounting truth.

5. Keep range/media behavior bounded.

As CrabLink Tauri media grows, storage should support safe segment/range access and content-b3 verification without pretending to provide DRM or uncopyable media.

6. Keep docs scanner phrases.

The plain scanner boundary phrases should remain unless the corresponding tests are intentionally updated.

### Long-Term Remaining Work

1. No root production in storage.

If future QuickChain roots are created, storage should not be the root-producing engine. It can store artifacts by b3, but root production belongs in approved deterministic QuickChain/ledger/proof components after canonical vectors are locked.

2. No validator behavior.

Storage nodes may eventually participate in availability, retrieval, or proof systems, but `svc-storage` itself should not quietly become a validator runtime.

3. No bridge/external settlement behavior.

Storage must not grow Solana/ROX/bridge/external-settlement code during the internal ROC proving phase.

4. No cache-only paid unlock.

Even if offline cache improves, paid content unlock must remain backend-derived. Cache can verify bytes and support UX, but cannot replace wallet/ledger truth.

5. No public finality claims.

Storage admission is not finality. Object availability is not consensus. b3 byte truth is not QuickChain finality.

---

# Pair-Level Summary — `svc-rewarder + svc-storage`

## What This Pair Achieved

Together, these two crates now enforce a critical middle section of the RustyOnions internal ROC value loop.

The intended value loop is:

`ron-proto econ DTOs -> ron-ledger truth -> svc-wallet issue/transfer/burn/hold/capture/release/receipt -> svc-storage/svc-gateway/omnigate paid enforcement -> ron-accounting snapshots -> svc-rewarder payout planning -> wallet/ledger receipts`

This pair specifically covers:

* storage paid admission and metering;
* storage quote-only economics;
* storage wallet-front-door settlement boundary;
* storage b3 integrity;
* storage cache boundary;
* storage range media boundary;
* storage observability boundary;
* rewarder deterministic payout planning;
* rewarder funding-source discipline;
* rewarder raw-engagement boundary;
* rewarder replay/no-double-issue discipline;
* rewarder no-direct-mutation boundary;
* rewarder tooling and docs boundary.

## Why This Pair Matters

This pair is important because it is where accidental inflation or fake economic authority could easily creep in.

Dangerous failure modes prevented:

* storage accepts fake paid headers and stores/unlocks content;
* storage cache unlocks paid content by itself;
* storage response pretends to be finality;
* storage treats b3 as payment proof;
* storage metrics leak wallet receipts/accounts/CIDs/roots;
* rewarder mints directly from raw views/clicks/engagement;
* rewarder double-issues on replay;
* rewarder directly mutates ledger;
* rewarder treats accounting counters as balance truth;
* rewarder invents protocol pool payouts without policy/funding provenance;
* either crate starts exposing roots, validators, bridges, settlement anchors, staking, liquidity, or external-chain logic.

## Gates Now Green

### `svc-rewarder`

Focused QuickChain preflight gate passed with seven focused QuickChain test targets.

The crate is parked for current Phase-0/preflight purposes.

### `svc-storage`

Focused QuickChain preflight gate passed with ten focused QuickChain test targets.

All-targets tests passed.

Clippy with `-D warnings` passed.

Parking gate passed.

The crate is parked for current Phase-0/preflight purposes.

## Regenerated Codebundles

The current working codebundle regeneration command format is:

`bash scripts/make_crate_codex.sh --force -c svc-rewarder`

`bash scripts/make_crate_codex.sh --force -c svc-storage`

The regenerated crate codebundles are now current after the parking gate.

## Current Pair Completion Estimate

For this specific pair’s current QuickChain Phase-0/preflight scope:

* `svc-rewarder`: approximately 95–100% parked for this sweep.
* `svc-storage`: approximately 95–100% parked for this sweep.
* Pair-level Phase-0/preflight status: effectively parked.

This does not mean the full QuickChain blueprint is complete.

It means this crate pair has completed the current preflight boundary sweep.

## What This Does Not Complete

This work does not complete:

* canonical locked hash vectors;
* state root engine;
* receipt root engine;
* accounting root engine;
* reward root engine;
* checkpoint production;
* validator sets;
* committee consensus;
* data availability/challenge/pruning;
* external anchors;
* Solana/ROX/bridge integration;
* public settlement;
* CrabLink chain authority;
* gateway/omnigate full paid enforcement sweep;
* final QuickChain beta runtime.

Those remain later phases and should remain gated.

## Next Crate Pair

The next planned crate pair is:

1. `svc-gateway`
2. `omnigate`

Reason:

After wallet/accounting/rewarder/storage boundaries, the next risk surface is paid enforcement and hydration at the public/client-facing boundary.

The next sweep should verify that:

* `svc-gateway` remains public boundary, not ledger/wallet/root authority;
* `omnigate` hydrates/enforces access without mutating ledger truth directly;
* neither service invents balances/receipts;
* neither service unlocks paid content from cache alone;
* both keep wallet/ledger receipts backend-derived/display-only;
* both remain QuickChain-aware only as future/parked doctrine, not runtime chain authority;
* both preserve Tauri/CrabLink paid-flow doctrine:
  prepare/quote -> explicit confirmation -> backend wallet path -> backend receipt -> unlock/render -> display-only receipt cache -> balance refresh.

## Carry-Forward Warnings

Future sessions should not undo these boundaries.

Do not add to `svc-rewarder` or `svc-storage`:

* public chain state;
* roots;
* checkpoints;
* validators;
* consensus;
* staking;
* liquidity;
* bridge code;
* Solana code;
* ROX code;
* external settlement;
* public anchor mutation;
* exchange-facing logic;
* hidden ledger mutation;
* fake balances;
* fake receipts;
* fake finality;
* cache-only paid unlock;
* raw engagement protocol payouts;
* DB-order roots;
* wall-clock roots;
* placeholder hashes;
* fake golden vectors.

## Safe Future Additions

Safe future additions may include:

* more docs hardening;
* more preflight tests;
* stronger policy validation;
* stronger wallet receipt verification;
* better idempotency/replay tests;
* better bounded media/range tests;
* better accounting export tests;
* better metrics label tests;
* stricter DTO deny-unknown-field checks;
* stricter integer minor-unit checks;
* canonical byte vector sketches;
* locked canonical bytes only after schemas are stable;
* locked hashes only after canonical bytes/domain separators/preimage framing are approved.

## Final Current State

`svc-rewarder + svc-storage` are now parked for the current QuickChain Phase-0/preflight sweep.

The internal ROC proof path is stronger because this pair now protects two major risk surfaces:

1. reward planning must not become unauthorized minting or direct ledger mutation;
2. storage/paid bytes must not become fake paid access, fake receipts, fake finality, or hidden settlement authority.

Next session can safely move forward into `svc-gateway + omnigate` with these boundaries established behind it.


### END NOTE - JUNE 17 2026 - 13:10 CST



### BEGIN NOTE - JUNE 19 2026 - 14:40 CST

The terminal output confirms both crates are now parkable: `svc-rewarder` passed its exhaustive gate with 8 QuickChain tests, and `svc-storage` passed its exhaustive gate with 11 QuickChain tests, including all-targets and clippy for both crates. 

Here are the paste-ready notes.

# QuickChain Phase-0 Notes — `svc-rewarder` + `svc-storage`

Date: 2026-06-19
Scope: QuickChain Phase-0 / QC-0A safety, preflight, and value-loop boundary hardening
Crates covered:

* `crates/svc-rewarder`
* `crates/svc-storage`

## Executive Summary

This session completed the QuickChain Phase-0 boundary hardening pass for the `svc-rewarder + svc-storage` crate pair.

The work did **not** add QuickChain roots, checkpoints, validators, bridge logic, staking, liquidity, public anchors, ROX, Solana, external settlement, public-chain authority, or chain-runtime behavior.

Instead, the changes locked both crates into their correct internal RustyOnions value-plane roles:

```text
svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

The key architectural result is that `svc-storage` remains byte/object infrastructure and metering input, while `svc-rewarder` remains deterministic payout planning only. Neither crate is allowed to become balance truth, receipt truth, ledger mutation authority, root authority, checkpoint authority, validator authority, bridge authority, external settlement authority, or finality authority.

Both crates now have explicit docs and tests enforcing these boundaries.

---

# 1. `svc-rewarder` Changes

## 1.1 Updated QuickChain preflight documentation

Updated:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

The document now explicitly records the crate’s QuickChain Phase-0 posture.

`svc-rewarder` is documented as:

```text
a deterministic ROC payout planner
```

It is explicitly documented as **not** being:

```text
a chain runtime
a validator
a bridge
a checkpoint writer
a root producer
a ledger mutation authority
wallet authority
balance truth
receipt truth
settlement finality
```

The document also names the two most important economic authority boundaries:

```text
svc-wallet is the mutation front-door
ron-ledger is durable economic truth
```

This makes clear that `svc-rewarder` may plan payouts, but it must not mutate balances or create authoritative receipts itself.

## 1.2 Locked `svc-rewarder` into the internal value loop

The docs now state the required internal value loop:

```text
svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

This matters because QuickChain must not drift into a design where reward planning becomes a silent money authority.

`svc-rewarder` is allowed to produce deterministic planning artifacts and wallet issue request planning payloads, but it must not bypass `svc-wallet`, must not treat planning artifacts as settlement finality, and must not treat funding provenance as finality.

The final phrase was made explicit for scanner compatibility:

```text
funding provenance is not settlement finality
```

That phrase is intentionally present because the docs test scans for it literally.

## 1.3 Documented allowed Phase-0 scope

The `svc-rewarder` docs now list allowed Phase-0 work:

```text
strict serde DTOs
integer minor-unit money strings only
canonical lowercase b3 identifiers
explicit funding provenance
wallet issue request planning
deterministic plan ordering
idempotency/replay boundary checks
raw engagement rejection checks
docs and preflight tooling
```

This keeps the crate aligned with the QuickChain doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

For this crate, the only allowed “economic output” is planning and wallet-handoff DTO shape. The authoritative mutation path remains downstream in `svc-wallet` and `ron-ledger`.

## 1.4 Documented forbidden Phase-0 scope

The `svc-rewarder` docs now explicitly forbid:

```text
root-producing code
checkpoint-producing code
validator code
bridge or external settlement code
direct ledger mutation
direct wallet mutation outside explicit svc-wallet handoff
fake balances
fake receipts
fake finality
Solana
ROX
public bridge
external anchors
staking or liquidity
exchange-facing logic
```

The future parked work is also documented as out of scope until the proper prerequisites exist:

```text
canonical bytes and locked vectors
state/account Merkle roots
receipt roots
validator-set logic
checkpoint signing
external DA
public anchors
bridges
staking or liquidity
CrabLink chain authority
gateway/omnigate/rewarder ledger mutation
```

This is important because `svc-rewarder` is near the value loop and could otherwise become a tempting place to add payout authority too early. The docs now make that drift visibly forbidden.

## 1.5 Raw engagement boundary strengthened

The docs now clearly state that raw engagement fields must not become direct protocol ROC payout authority.

Examples documented as forbidden direct payout inputs include:

```text
raw views
raw watch seconds
raw clicks
raw impressions
likes
shares
follows
views-to-ROC formulas
watch-seconds-to-ROC formulas
```

Raw usage may still feed accounting, fraud analysis, or policy inputs after classification and validation, but raw engagement cannot directly mint, issue, allocate, or authorize protocol ROC.

This protects the internal economy from bot-farm style incentives and ensures payout authority remains policy/accounting/wallet/ledger mediated.

## 1.6 Replay and identity boundary documented

The docs now distinguish between retry keys and ledger identity:

```text
idempotency keys are replay/dedupe tools
idempotency keys are not ledger operation identity
idempotency keys are not validator consensus
idempotency keys are not settlement authority
```

This matters because QuickChain’s future event model distinguishes:

```text
operation_id      = backend-assigned durable ledger-operation identity
idempotency_key   = retry/dedupe key
account_sequence  = ledger-assigned sequence
hold_id           = one hold lifecycle identifier
```

For `svc-rewarder`, this means deterministic replay protection can exist without pretending to own durable ledger operation identity.

## 1.7 Added pair-level value-loop boundary test

Added:

```text
crates/svc-rewarder/tests/quickchain_preflight_value_loop_boundary.rs
```

This test is a source/docs boundary test. It does not create chain state. It does not create roots. It does not create checkpoints. It does not create validator logic.

It enforces that the rewarder docs name the correct value-loop sequence and that the source code remains in the proper role.

The test checks the docs for these required phrases:

```text
svc-storage/svc-gateway/omnigate paid enforcement
ron-accounting snapshots
svc-rewarder payout planning
explicit approved payout intent
svc-wallet
ron-ledger
deterministic roc payout planner
svc-wallet is the mutation front-door
ron-ledger is durable economic truth
```

The goal is to make the correct architecture machine-checked, not merely implied.

## 1.8 Settlement intent source boundary locked

The new rewarder test checks:

```text
src/outputs/intents.rs
```

It requires wallet-handoff DTO markers such as:

```text
SettlementIntent
SettlementBatch
WalletIssueRequest
WALLET_ISSUE_PATH
to_wallet_issue_request
#[serde(deny_unknown_fields)]
```

It also rejects signs that settlement intents are turning into receipt/root/finality authority.

Forbidden markers include:

```text
WalletReceipt
receipt_hash
ledger_root
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
finality
external_anchor
bridge_txid
```

This preserves the important distinction:

```text
rewarder settlement intent = wallet handoff planning DTO
wallet receipt             = backend wallet/ledger-derived truth
```

`svc-rewarder` must not generate wallet receipts itself.

## 1.9 Wallet client boundary locked

The new rewarder test checks:

```text
src/outputs/wallet.rs
```

It requires explicit wallet boundary markers:

```text
svc-wallet
preview_issue_batch
emit_issue_batch
dry_run
WalletHttpIssueOutcome
```

It also rejects direct ledger/chain authority markers such as:

```text
ron_ledger::
LedgerClient
ledger_commit
checkpoint_hash
validator_signature
state_root
receipt_root
external_anchor
bridge_txid
```

This protects the intended shape:

```text
rewarder -> svc-wallet -> ron-ledger
```

and prevents the accidental shape:

```text
rewarder -> ron-ledger
```

The rewarder may call wallet-facing seams. It must not directly commit to the ledger.

## 1.10 Core reward planning source scan added

The new rewarder test scans important reward planning files:

```text
src/core/compute.rs
src/inputs/accounting.rs
src/outputs/manifest.rs
src/outputs/intents.rs
```

It verifies they do not gain QuickChain runtime authority fields:

```text
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
settlement_finality
external_anchor
bridge_txid
staking
liquidity
```

This is a drift detector. If a future patch tries to put Phase-1+ chain authority into the rewarder prematurely, the test should fail.

## 1.11 Final `svc-rewarder` gate result

The focused docs failure from the missing literal phrase was fixed.

Final verified `svc-rewarder` status:

```text
cargo test -p svc-rewarder --test quickchain_preflight_docs
cargo test -p svc-rewarder --test quickchain_preflight_value_loop_boundary
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Final result:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=8
```

The exhaustive gate included:

```text
format check
forbidden Python helper scan
dynamic focused QuickChain test discovery
8 focused QuickChain tests
svc-rewarder all-targets test
svc-rewarder clippy --all-targets -- -D warnings
forbidden-scope marker
```

The crate is now parkable for this QuickChain Phase-0 pass.

---

# 2. `svc-storage` Changes

## 2.1 Updated QuickChain preflight documentation

Updated:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

The document now explicitly records that `svc-storage` remains:

```text
content-addressed byte/object infrastructure
```

It also explicitly states:

```text
b3 hashes identify bytes only
```

The docs clarify that b3 hashes are:

```text
content truth only
not payment proof
not receipt roots
not account state roots
not checkpoint roots
not settlement finality
```

This is one of the most important storage-side boundaries for QuickChain. A hash proves bytes, not payment, authorization, finality, or ledger truth.

## 2.2 Locked `svc-storage` into the internal value loop

The docs now state the storage-side value loop:

```text
svc-storage paid admission and b3 byte integrity
-> storage/access metering
-> ron-accounting derivative snapshots
-> svc-rewarder deterministic payout planning
-> explicit approved payout intent
-> svc-wallet
-> ron-ledger
```

This establishes `svc-storage` as the front part of the paid-content and metering path, not as an economic truth source.

`svc-storage` may:

```text
store bytes
serve bytes
serve bounded ranges
verify b3
price/quote paid writes
verify backend wallet hold evidence
capture/release through configured wallet settlement path
emit usage events
export usage events to accounting
```

But `svc-storage` must not:

```text
mutate ledger directly
invent balances
invent receipts
claim finality
produce QuickChain roots
act as a validator
act as a bridge
act as settlement truth
turn raw metering into protocol payout authority
```

The distinction is subtle but critical: storage may have paid admission and wallet-facing settlement adapters, but it is not settlement authority and does not own ledger truth.

## 2.3 Paid access and cache boundary documented

The docs now explicitly state:

```text
cache must not decide paid access by itself
```

Additional documented boundaries:

```text
cache can verify b3 before trusted render
cache cannot unlock paid content alone
offline cache verifies b3 before trusted render
paid content requires backend-derived authorization
paid content requires backend-derived receipt/authorization
receipt cache is display-only
a storage CID, manifest CID, or b3 hash is not payment proof
```

This aligns `svc-storage` with the broader CrabLink/Tauri doctrine:

```text
cache is convenience
backend-derived authorization unlocks paid access
b3 verifies bytes
wallet/ledger own economic truth
```

## 2.4 Metering boundary documented

The docs now make storage metering explicitly derivative:

```text
storage metering is derivative accounting input only
```

Usage events are documented as:

```text
not balance updates
not wallet receipts
not payout authority
not ledger mutation
```

Storage/access metering can feed `ron-accounting`, and accounting snapshots can later feed `svc-rewarder`, but storage metering itself does not create money movement.

This prevents a future mistake where raw storage usage could directly allocate ROC.

## 2.5 Bounded media boundary documented

The docs now include large-media constraints:

```text
large media must stay bounded and honest
range/segment serving is preferred for large media
full-file unbounded command/result paths are not allowed
each rendition owns its own b3
no DRM or anti-rip guarantee is made
```

This is important for CrabLink because media is central to the product layer, and `svc-storage` must remain honest about what it can guarantee.

Storage can prove and serve bytes. It cannot claim DRM, anti-rip protection, or magical paid-content security once bytes are legitimately delivered.

## 2.6 Forbidden Phase-0 scope documented

The `svc-storage` docs now explicitly forbid:

```text
fake balances
fake receipts
silent spend
roots
validators
bridges
external settlement
checkpoints
anchors
external anchors
bridge or external settlement authority
staking
liquidity
Solana
ROX
exchange-facing logic
root-producing code
checkpoint-producing code
validator code
```

This prevents the storage crate from becoming a stealth chain boundary.

`svc-storage` is allowed to perform byte storage, paid admission checks, wallet-hold verification, wallet capture/release through configured wallet paths, and accounting export. It is not allowed to become a QuickChain runtime component or finality source.

## 2.7 Added pair-level value-loop boundary test

Added:

```text
crates/svc-storage/tests/quickchain_preflight_value_loop_boundary.rs
```

This test enforces the storage side of the value loop through docs/source scanning.

It checks the docs for required phrases:

```text
svc-storage paid admission and b3 byte integrity
storage/access metering
ron-accounting derivative snapshots
svc-rewarder deterministic payout planning
explicit approved payout intent
svc-wallet
ron-ledger
b3 hashes identify bytes only
cache must not decide paid access by itself
storage metering is derivative accounting input only
```

This ensures the architectural role of storage remains visible and machine-checked.

## 2.8 Pricing and metering source boundary locked

The new storage test checks:

```text
src/policy/economics.rs
src/accounting/mod.rs
src/accounting/exporter.rs
```

It requires markers proving these paths are planning/metering only:

```text
PaidStoragePriceEstimate
side-effect free
does not call wallet, ledger
UsageEventDto
usage only; no balances; no ledger mutation
export failure never mutates ledger or wallet state
no wallet receipt/body bytes exported
```

This protects the quote and accounting-export paths from becoming mutation paths.

Important distinction:

```text
paid estimate = quote only
usage event   = metering only
accounting export = derivative reporting only
wallet/ledger = economic truth
```

## 2.9 Paid-write proof boundary locked

The new storage test checks:

```text
src/policy/paid_write.rs
```

It requires wallet-hold evidence markers:

```text
WalletReceipt
validate_as_paid_write_hold
PaidWriteProof
paid_storage_context_idem
wallet receipt hash must be b3:<64 lowercase hex>
paid proof must reference a wallet hold receipt
```

This keeps paid-write admission tied to backend wallet evidence.

The test also rejects QuickChain runtime/finality markers in paid-write verification:

```text
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
settlement_finality
external_anchor
bridge_txid
staking
liquidity
```

This protects the intended meaning:

```text
paid-write proof = wallet hold evidence for storage admission
paid-write proof != QuickChain finality
paid-write proof != receipt root
paid-write proof != validator proof
```

## 2.10 Wallet capture/release settlement seam locked

The new storage test checks:

```text
src/policy/settlement.rs
```

It requires explicit wallet-settlement seam markers:

```text
SETTLEMENT_MODE_WALLET_CAPTURE
PaidStorageSettlementPlan
WalletSettlementHttpClient
capture_idem
release_idem
failed_write_release_idem
```

This confirms that storage’s post-write settlement behavior is a wallet-facing adapter, not direct ledger mutation and not chain settlement.

The test rejects direct ledger/chain authority markers:

```text
ron_ledger::
LedgerClient
ledger_commit
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
external_anchor
bridge_txid
staking
liquidity
```

This protects the intended path:

```text
svc-storage -> svc-wallet capture/release path -> ron-ledger
```

and rejects the forbidden path:

```text
svc-storage -> ron-ledger direct commit
```

## 2.11 Storage value-loop source scan added

The new storage test scans:

```text
src/policy/economics.rs
src/policy/paid_write.rs
src/policy/settlement.rs
src/accounting/mod.rs
src/accounting/exporter.rs
```

It rejects QuickChain runtime authority fields:

```text
quickchain_root
state_root
receipt_root
checkpoint_hash
validator_signature
validator_set
settlement_finality
external_anchor
bridge_txid
staking
liquidity
```

This is a future drift detector. If later code tries to push roots, validator signatures, anchors, staking, liquidity, or bridge IDs into storage’s economic path, the test should fail.

## 2.12 Final `svc-storage` gate result

Final verified `svc-storage` status:

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final result:

```text
svc-storage quickchain exhaustive preflight gate passed: tests=11
```

The exhaustive gate included:

```text
format check
forbidden Python helper scan
dynamic focused QuickChain test discovery
11 focused QuickChain tests
svc-storage all-targets test
svc-storage clippy --all-targets -- -D warnings
forbidden-scope marker
```

The crate is now parkable for this QuickChain Phase-0 pass.

---

# 3. Cross-Crate Value-Loop Result

This session’s main contribution was not just adding isolated tests. It locked the relationship between `svc-storage` and `svc-rewarder` inside the larger RustyOnions internal ROC economy.

The resulting internal flow is now documented and tested as:

```text
paid storage/content admission
-> bounded byte serving and b3 verification
-> usage/metering events
-> accounting snapshots
-> deterministic reward planning
-> explicit wallet issue/capture/release intents
-> svc-wallet mutation front-door
-> ron-ledger durable economic truth
```

No crate in this pair is allowed to skip forward and become ledger truth.

## Correct role split

### `svc-storage`

Owns:

```text
bytes
CAS
b3 verification
bounded range serving
paid-write admission
wallet-hold proof verification
wallet capture/release adapter
usage event generation
accounting export
```

Does not own:

```text
balances
receipts
ledger mutation
QuickChain roots
checkpoint roots
validator signatures
bridge state
external settlement
staking
liquidity
finality
```

### `svc-rewarder`

Owns:

```text
deterministic payout planning
policy-aware reward calculation
funding provenance display
wallet issue request planning
idempotent preview/egress behavior
manifest planning artifacts
```

Does not own:

```text
wallet receipts
balance truth
ledger operation truth
direct ledger commits
QuickChain roots
checkpoint roots
validator signatures
external anchors
bridges
staking
liquidity
settlement finality
```

### `svc-wallet`

Still owns:

```text
economic mutation front-door
issue
transfer
burn
hold
capture
release
receipt generation path
```

### `ron-ledger`

Still owns:

```text
durable economic truth
ledger replay
accepted operation history
balance truth
operation identity
```

---

# 4. Why These Changes Matter

These changes protect the project against the most dangerous QuickChain Phase-0 failure mode: accidentally adding chain authority before the internal ROC economy is proven.

Without these tests, it would be easy for future code to drift toward one of these unsafe shapes:

```text
svc-storage creates payment truth from b3 or cache state
svc-storage treats paid-write headers as finality
svc-storage mutates ledger directly
svc-rewarder creates receipts
svc-rewarder treats funding provenance as finality
svc-rewarder emits direct ledger commits
svc-rewarder turns raw engagement into protocol ROC
either crate starts carrying roots/checkpoints/validators/anchors
```

The new docs/tests make those mistakes loud.

This keeps the current mission intact:

```text
prove ROC as a fully internal token/accounting/value plane first
```

No ROX.
No Solana.
No external settlement.
No public bridge.
No staking.
No liquidity.
No exchange-facing logic.
No public-chain authority.
No fake balances.
No fake receipts.
No silent spend.

---

# 5. Tests Added or Strengthened

## `svc-rewarder`

Added:

```text
crates/svc-rewarder/tests/quickchain_preflight_value_loop_boundary.rs
```

Strengthened by docs update:

```text
crates/svc-rewarder/tests/quickchain_preflight_docs.rs
```

Focused QuickChain suite now includes:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Final focused count:

```text
8 focused QuickChain tests
```

## `svc-storage`

Added:

```text
crates/svc-storage/tests/quickchain_preflight_value_loop_boundary.rs
```

Focused QuickChain suite now includes:

```text
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Final focused count:

```text
11 focused QuickChain tests
```

---

# 6. Final Verification Commands Run

## `svc-rewarder`

```text
cargo fmt -p svc-rewarder
cargo test -p svc-rewarder --test quickchain_preflight_docs
cargo test -p svc-rewarder --test quickchain_preflight_value_loop_boundary
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Final gate result:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=8
```

## `svc-storage`

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final gate result:

```text
svc-storage quickchain exhaustive preflight gate passed: tests=11
```

Both crates passed:

```text
cargo fmt check
focused QuickChain tests
all-targets tests
clippy --all-targets -- -D warnings
forbidden helper scan
forbidden scope marker
```

---

# 7. What Remains for These Crates Later

These crates are parkable for this Phase-0 pass, but future work remains once QuickChain advances beyond preflight/boundary hardening.

## Future `svc-rewarder` work

Later, after canonical bytes and locked vectors exist, possible future work may include:

```text
stronger canonical payout vector alignment
formal reward manifest vector fixtures
richer accounting snapshot validation
more interop vectors with ron-accounting
production wallet issue egress hardening
signed policy enforcement improvements
audit/event export hardening
reward planning replay corpus
```

Still forbidden until the correct phase gates are green:

```text
root-producing reward manifests
receipt roots
state roots
validator participation
checkpoint signing
external anchors
bridge IDs
staking
liquidity
direct ledger mutation
raw engagement protocol payouts
```

## Future `svc-storage` work

Later, after internal ROC paid access is more fully proven, possible future work may include:

```text
stronger wallet receipt lookup hardening
paid range-access policy hardening
storage accounting export interop vectors
more paid-write fraud/failure-mode tests
better bounded-media transport proofs
cache verification proofs
offline cache UX rules
gateway/omnigate integration regression tests
```

Still forbidden until the correct phase gates are green:

```text
cache-only paid unlock
b3-as-payment-proof
storage-generated receipts
storage-generated roots
storage-generated finality
direct ledger mutation
validator signatures
bridge settlement
external anchors
staking
liquidity
```

---

# 8. Parked Status

`svc-rewarder` is parkable for this QuickChain Phase-0 pass.

`svc-storage` is parkable for this QuickChain Phase-0 pass.

This pair can now be treated as complete for the current boundary-hardening sweep, subject to final regenerated codebundles being saved after the green run.

Recommended regeneration command:

```text
bash scripts/make_crate_codex.sh --force -c svc-rewarder
bash scripts/make_crate_codex.sh --force -c svc-storage
```

Next crate pair in the coherent QuickChain build order:

```text
svc-gateway + omnigate
```

The next pair should continue the same doctrine:

```text
backend owns truth
wallet mutates
ledger is durable truth
accounting is derivative
rewarder plans
storage serves bytes
gateway/omnigate enforce paid hydration without becoming ledger, wallet, root, checkpoint, validator, bridge, or finality authority
```


### END NOTE - JUNE 19 2026 - 14:40 CST



### BEGIN NOTE - JUNE 20 2026 - 11:30 CST

The latest terminal output confirms the pair is green: `svc-rewarder` parked with **9 focused QuickChain tests**, and `svc-storage` parked with **12 focused QuickChain tests**. The new source-authority scan tests also passed for both crates before the park scripts ran. 

Drop these into the crate-local notes.

### BEGIN NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-rewarder

# svc-rewarder QuickChain Phase-0 / QC-1A Notes

## Current Status

```text
svc-rewarder: GREEN / QuickChain Phase-0 boundary parked / QC-1A foundation slice complete for this pass
```

`svc-rewarder` is now parked for the current QuickChain Phase-0 / QC-1A boundary sweep.

The crate’s QuickChain posture is:

```text
svc-rewarder plans deterministic ROC payout batches.
svc-rewarder does not mutate balances.
svc-rewarder does not mint, issue, transfer, burn, hold, capture, or release ROC.
svc-rewarder does not produce wallet receipts.
svc-rewarder does not produce QuickChain roots.
svc-rewarder does not claim finality or settlement.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
QuickChain remains future verification/settlement infrastructure.
```

This is exactly the correct position for `svc-rewarder` inside the internal ROC value loop.

The crate now proves that reward planning is deterministic, funding-source-gated, raw-engagement-safe, replay-safe, and routed toward wallet/ledger authority instead of becoming authority itself.

---

## Intended Role

`svc-rewarder` is a payout planning service.

It may:

```text
- consume normalized accounting snapshots
- consume approved reward policy/config
- consume explicit funding-source information
- compute deterministic reward manifests
- compute deterministic settlement/payout intent batches
- create wallet issue request shapes for handoff
- expose preview/dry-run behavior
- enforce idempotency at the planning layer
- preserve deterministic ordering of recipients/intents
- quarantine invalid, unsafe, or over-budget planning inputs
- emit metrics about planned intents
```

It must not:

```text
- mutate wallet state directly
- mutate ledger state directly
- mint/issue/transfer/burn/hold/capture/release ROC directly
- produce backend wallet receipts
- claim balance truth
- claim receipt truth
- claim operation truth
- claim account_sequence truth
- treat idempotency_key as operation authority
- treat reward planning as finality
- treat accounting snapshots as balance truth
- treat raw engagement as protocol payout authority
- expose QuickChain roots
- expose checkpoint authority
- expose validator behavior
- expose settlement authority
- expose external anchors
- expose public bridge behavior
- expose staking or liquidity logic
- expose Solana/ROX/external settlement paths
```

The short doctrine is:

```text
ron-accounting measures.
svc-rewarder plans.
svc-wallet mutates.
ron-ledger is truth.
QuickChain later verifies roots/proofs.
```

---

## Why This Crate Matters

`svc-rewarder` is one of the highest-risk crates for accidental economic authority creep.

Without strong boundaries, the dangerous shape would be:

```text
raw views/clicks/engagement
-> rewarder computes payout
-> rewarder directly mutates ledger
-> rewarder emits receipt-looking object
-> gateway/omnigate/CrabLink treat it as spend authority
```

That shape is forbidden.

The safe shape is:

```text
ron-accounting snapshot
-> svc-rewarder deterministic planning
-> explicit funding-source policy
-> wallet issue request / settlement intent handoff
-> svc-wallet mutation path
-> ron-ledger durable truth
-> backend-derived receipt
```

This crate’s current test/documentation posture makes the dangerous shape difficult to introduce accidentally.

---

## Files Added or Hardened

### Documentation

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

Purpose:

```text
- Defines svc-rewarder as deterministic payout planning only.
- States svc-wallet is the mutation front-door.
- States ron-ledger is durable economic truth.
- States ron-accounting snapshots are planning inputs, not balance truth.
- Names the allowed Phase-0 scope.
- Names the forbidden QuickChain/runtime scope.
- Keeps future roots/checkpoints/validators/settlement parked outside the crate.
- Documents funding-source, raw-engagement, replay, value-loop, and no-direct-mutation boundaries.
```

### Scripts

```text
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The preflight script now:

```text
- uses Bash strict mode
- runs from repo root
- verifies quickchain-preflight docs exist
- rejects checked-in Python helper drift under the crate
- runs cargo fmt check
- dynamically discovers every tests/quickchain*.rs target
- runs every discovered focused QuickChain test target
- runs cargo test -p svc-rewarder --all-targets
- runs cargo clippy -p svc-rewarder --all-targets -- -D warnings
- prints the forbidden-scope marker
- prints the final exhaustive preflight success marker
```

The park script now:

```text
- verifies the docs file exists
- verifies the preflight script exists
- verifies the tooling boundary test exists
- delegates to the exhaustive preflight script
- prints the final parking-gate success marker
```

Important low-disk workflow note:

```text
Do not run both dev-quickchain-preflight.sh and dev-quickchain-park.sh for the same crate unless deliberately requesting duplicate full passes.

The park script delegates to preflight.
```

### Focused QuickChain Tests

Current focused QuickChain targets:

```text
crates/svc-rewarder/tests/quickchain_preflight_boundary.rs
crates/svc-rewarder/tests/quickchain_preflight_docs.rs
crates/svc-rewarder/tests/quickchain_preflight_funding_source.rs
crates/svc-rewarder/tests/quickchain_preflight_no_direct_mutation.rs
crates/svc-rewarder/tests/quickchain_preflight_raw_engagement.rs
crates/svc-rewarder/tests/quickchain_preflight_replay_no_double_issue.rs
crates/svc-rewarder/tests/quickchain_preflight_source_authority_scan.rs
crates/svc-rewarder/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-rewarder/tests/quickchain_tooling_boundary.rs
```

This gives `svc-rewarder` nine focused QuickChain test targets in the current pass.

---

## What Each QuickChain Test Target Proves

### `quickchain_preflight_boundary.rs`

This target proves the outer rewarder DTO and response boundary.

It verifies:

```text
- JSON number money is rejected at the snapshot wire boundary.
- Reward manifests do not expose roots, receipts, balances, or finality.
- Wallet preview output remains an issue-request shape, not receipt or balance truth.
- Compute requests reject smuggled QuickChain authority fields.
```

This is important because rewarder output can look economic. These tests keep it as planning output only.

### `quickchain_preflight_docs.rs`

This target proves the documentation stays synchronized with the crate’s intended QuickChain posture.

It verifies that docs:

```text
- list every focused preflight suite
- state rewarder is planning-only and not chain runtime
- name allowed Phase-0 scope
- name forbidden Phase-0 scope
- keep future QuickChain work parked outside rewarder
- name raw-engagement, replay, and funding-source boundaries
```

This matters because the crate boundary is not only code-level. Future contributors need the doctrine written down locally.

### `quickchain_preflight_funding_source.rs`

This target proves payout planning cannot silently invent funding authority.

It verifies:

```text
- unsigned protocol-pool policy is rejected by validator
- policy requires explicit funding source on wire
- current policy accepts explicit protocol-pool provenance only when authority fields are not smuggled
- reward manifests carry funding provenance but not funding finality
- wallet previews carry batch provenance but remain wallet issue request shapes
- compute requests reject top-level funding-authority smuggling
```

This blocks a dangerous future where rewarder could act like a protocol mint.

### `quickchain_preflight_no_direct_mutation.rs`

This target proves rewarder does not expose or accept direct wallet/ledger mutation authority.

It verifies:

```text
- planning outputs do not claim receipts, balances, operation truth, roots, or finality
- config rejects external settlement, bridge, anchor, validator, and root knobs
- compute requests reject direct mutation authority smuggling
- router does not expose direct wallet, ledger, QuickChain, or bridge mutation routes
```

This keeps `svc-rewarder` from becoming a hidden wallet, ledger, or settlement service.

### `quickchain_preflight_raw_engagement.rs`

This target proves raw Web2-style engagement cannot directly become protocol payout authority.

It verifies:

```text
- allowed contribution counters are storage egress and uptime only
- reward policy rejects raw engagement formula fields
- accounting snapshot rejects raw engagement contribution fields
- compute request rejects top-level raw engagement payout fields
```

This is critical because fakeable metrics like views, clicks, likes, or visits must never directly mint or allocate protocol ROC.

### `quickchain_preflight_replay_no_double_issue.rs`

This target proves replay behavior remains deterministic and does not imply duplicate payout authority.

It verifies:

```text
- duplicate epoch replay is dedupe, not second payout authority
- idempotency keys are retry dedupe, not operation identity
- same snapshot, policy, and epoch produce same plan commitment
- reordered snapshot rows produce the same plan
```

This protects the reward loop from replay/double-issue bugs.

### `quickchain_preflight_source_authority_scan.rs`

This target was added in the current pass.

It proves the source tree does not quietly grow runtime authority or QuickChain implementation surfaces.

It verifies:

```text
- svc-rewarder source tree does not define QuickChain runtime modules
- svc-rewarder source does not contain direct chain or ledger authority tokens after stripping comments/string literals
```

This is a guardrail against future accidental imports, modules, or authority-shaped code paths.

The source scan is intentionally code-only: it strips comments and string literals before checking forbidden runtime tokens so docs/tests can still describe forbidden concepts.

### `quickchain_preflight_value_loop_boundary.rs`

This target proves rewarder stays in the correct place in the internal ROC value loop.

It verifies:

```text
- docs lock rewarder position in the internal value loop
- settlement intents are wallet handoff DTOs, not receipts or roots
- wallet client boundary targets wallet without direct ledger or chain authority
- core rewarder sources do not gain QuickChain runtime authority fields
```

This is the cross-crate boundary test that keeps rewarder from replacing wallet/ledger truth.

### `quickchain_tooling_boundary.rs`

This target proves the crate-local QuickChain tooling remains safe and repeatable.

It verifies:

```text
- park script delegates to exhaustive preflight
- preflight script is Bash/cargo-only and keeps the full gate
- preflight script discovers all QuickChain tests dynamically
- no Python helpers are checked into the rewarder crate
```

This supports the current project workflow and prevents stale manual test lists.

---

## Current Verified Gate Results

The final parking run verified:

```text
Focused QuickChain tests discovered: 9

quickchain_preflight_boundary.rs:              passed
quickchain_preflight_docs.rs:                  passed
quickchain_preflight_funding_source.rs:        passed
quickchain_preflight_no_direct_mutation.rs:    passed
quickchain_preflight_raw_engagement.rs:        passed
quickchain_preflight_replay_no_double_issue.rs: passed
quickchain_preflight_source_authority_scan.rs: passed
quickchain_preflight_value_loop_boundary.rs:   passed
quickchain_tooling_boundary.rs:                passed

cargo test -p svc-rewarder --all-targets:      passed
cargo clippy -p svc-rewarder --all-targets -- -D warnings: passed

Final marker:
== svc-rewarder quickchain exhaustive preflight gate passed: tests=9 ==
== svc-rewarder QuickChain parking gate passed ==
```

All-targets coverage included:

```text
src/lib.rs unit tests
src/main.rs tests
tests/integration.rs
tests/unit.rs
benches/reward_calc.rs
all focused quickchain*.rs targets
```

Important successful existing tests include:

```text
- rewarder settlement issues through wallet once without double issue
- settlement intent egress is idempotent by run key
- compute happy path and replay are deterministic
- settlement preview endpoint returns wallet issue batch
- metrics include planned settlement intents after compute
- dry run can promote to production without consuming run key
- accounting interop vector is consumable by rewarder snapshot DTO
- rewarder and accounting agree on canonical snapshot CID
- wallet issue request serializes amount as string
- wallet client dry run posts nothing
- wallet client posts issue requests to wallet route
```

---

## What svc-rewarder Now Proves

`svc-rewarder` now proves:

```text
- rewarder output is plan/intent only
- rewarder planning is deterministic for same input
- rewarder replay is dedupe/idempotency, not second payout authority
- idempotency_key is retry dedupe, not operation identity
- rewarder rejects smuggled authority fields
- rewarder rejects raw engagement protocol-payout basis
- rewarder requires explicit funding-source provenance
- rewarder uses integer minor-unit string money at the wire boundary
- rewarder does not expose balance truth
- rewarder does not expose receipt truth
- rewarder does not expose operation truth
- rewarder does not expose roots/finality/checkpoints
- rewarder does not mutate ledger directly
- rewarder does not bypass svc-wallet
- rewarder routes mutation requests toward svc-wallet only
- rewarder docs preserve future QuickChain scope without implementing it
- rewarder tooling discovers and runs QuickChain gates dynamically
```

---

## Current Forbidden Scope Locked Out

`svc-rewarder` must continue to reject:

```text
roots
receipt roots
account state roots
hold roots
checkpoint roots
epoch roots
checkpoint production
validator behavior
settlement authority
finality claims
external anchors
bridge behavior
public-chain settlement
staking
liquidity
Solana path
ROX path
external L2/DA mutation path
direct ledger mutation
direct wallet mutation
protocol payout from raw engagement
fake balances
fake receipts
silent spend
```

---

## Cross-Crate Boundary After This Work

The intended value-loop position is now clearer:

```text
ron-proto:
  DTOs and strict wire shapes

ron-ledger:
  durable economic truth and replay

svc-wallet:
  mutation front-door and receipt production

svc-storage:
  b3 bytes, paid access enforcement hooks, metering input

ron-accounting:
  derivative snapshots and planning inputs

svc-rewarder:
  deterministic payout planning and wallet handoff intents

future QuickChain:
  eventually verifies canonical roots/proofs/checkpoints after locked vectors
```

The wrong shape remains forbidden:

```text
ron-accounting pays directly.
svc-rewarder pays directly.
svc-storage pays directly.
svc-gateway pays directly.
omnigate pays directly.
CrabLink pays directly.
raw engagement pays directly.
```

---

## Completion Estimate For This Slice

For this crate’s current QuickChain Phase-0 / QC-1A boundary scope:

```text
svc-rewarder: 95–100% parked for this sweep
```

This does not mean the full future reward system is finished.

Remaining future work, after explicit later gates, may include:

```text
- deeper production policy integration
- hardened production wallet transport
- stronger accounting snapshot windows
- more complete payout planning economics
- production reward scheduling
- future root material emitted by upstream canonical systems
- future proof verification paths
```

But those are not required for this current boundary pass.

---

## Do Not Reopen Unless

Do not reopen this crate during the current pass unless:

```text
- terminal output shows a regression
- a later crate-pair exposes a missing rewarder boundary
- svc-wallet/ron-ledger DTO changes require rewarder handoff updates
- ron-accounting snapshot format changes require rewarder parser/interop updates
- QuickChain vector/root gates are explicitly opened later
```

---

## Regeneration Command

After this pass, regenerate the crate codebundle with:

```bash
bash scripts/make_crate_codex.sh --force -c svc-rewarder
```

---

### END NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-rewarder

### BEGIN NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-storage

# svc-storage QuickChain Phase-0 / QC-1A Notes

## Current Status

```text
svc-storage: GREEN / QuickChain Phase-0 boundary parked / QC-1A foundation slice complete for this pass
```

`svc-storage` is now parked for the current QuickChain Phase-0 / QC-1A boundary sweep.

The crate’s QuickChain posture is:

```text
svc-storage stores and serves bytes by canonical b3 identity.
svc-storage may enforce paid-access admission through backend-derived authorization.
svc-storage may provide metering/quote inputs.
svc-storage does not mutate wallet state.
svc-storage does not mutate ledger state.
svc-storage does not mint, issue, transfer, burn, hold, capture, or release ROC.
svc-storage does not produce wallet receipts.
svc-storage does not treat b3 as payment proof.
svc-storage does not let cache unlock paid content alone.
svc-storage does not produce QuickChain roots.
svc-storage does not claim finality or settlement.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
QuickChain remains future verification/settlement infrastructure.
```

This is the correct role for `svc-storage` inside the internal ROC value loop.

The crate now proves b3 integrity, paid/cache boundaries, range-media boundaries, quote-only economics, observability redaction, and no-direct-mutation posture.

---

## Intended Role

`svc-storage` is content-addressed storage and byte-serving infrastructure.

It may:

```text
- ingest bytes
- derive canonical b3 IDs from bytes
- verify b3 before trusted reads
- store objects/manifests/renditions
- serve objects by b3
- serve ranges/segments for bounded media
- quote storage or retrieval cost shapes
- require backend-derived paid authorization for paid content
- expose admission/metering signals
- emit derivative accounting inputs
- participate in paid enforcement by checking authorization/receipt-shaped backend proof
```

It must not:

```text
- decide balances
- create receipts
- create wallet holds
- capture wallet holds
- release wallet holds
- mint/issue/transfer/burn ROC
- mutate ron-ledger
- bypass svc-wallet
- treat b3 byte identity as payment proof
- treat cached bytes as paid unlock authority
- treat display receipt cache as spend authority
- expose QuickChain roots
- expose checkpoint authority
- expose validator behavior
- expose settlement authority
- expose external anchors
- expose public bridge behavior
- expose staking or liquidity logic
- expose Solana/ROX/external settlement paths
```

The short doctrine is:

```text
b3 proves bytes.
b3 does not prove payment.
cache proves possession.
cache does not prove entitlement.
storage admission is not finality.
wallet/ledger truth unlocks paid economics.
```

---

## Why This Crate Matters

`svc-storage` sits directly in the paid content path. That makes it a dangerous place for accidental authority creep.

Without guardrails, the wrong shape would be:

```text
client has bytes in cache
-> storage sees b3
-> storage treats b3/cache as payment proof
-> paid content unlocks without backend wallet/ledger truth
```

Another wrong shape would be:

```text
storage quote/admission
-> storage creates receipt-looking object
-> gateway/omnigate/CrabLink treat storage response as paid finality
```

Both are forbidden.

The safe shape is:

```text
user requests paid content
-> prepare/quote
-> explicit confirmation
-> backend wallet path
-> backend-derived receipt/authorization
-> storage/gateway/omnigate enforce paid access
-> bytes unlock/render
-> receipt cache remains display-only
```

This crate’s current test/documentation posture keeps storage in the safe shape.

---

## Files Added or Hardened

### Documentation

```text
crates/svc-storage/docs/quickchain-preflight.md
```

Purpose:

```text
- Defines svc-storage as bytes/b3/content infrastructure.
- States b3 is content truth only.
- States b3 is not payment proof.
- States cache cannot unlock paid content alone.
- States paid content requires backend-derived authorization/receipt.
- States receipt cache is display-only.
- States storage metering is derivative accounting input only.
- States svc-wallet is the mutation front-door.
- States ron-ledger is durable economic truth.
- Documents bounded media/range/segment posture.
- Documents that each rendition owns its own b3.
- Names forbidden QuickChain/runtime scope.
```

### Scripts

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The preflight script now:

```text
- uses Bash strict mode
- runs from repo root
- verifies quickchain-preflight docs exist
- rejects checked-in Python helper drift under the crate
- runs cargo fmt check
- dynamically discovers every tests/quickchain*.rs target
- runs every discovered focused QuickChain test target
- runs cargo test -p svc-storage --all-targets
- runs cargo clippy -p svc-storage --all-targets --no-deps -- -D warnings
- prints the forbidden-scope marker
- prints the final exhaustive preflight success marker
```

The park script now:

```text
- verifies the docs file exists
- verifies the preflight script exists
- verifies the tooling boundary test exists
- delegates to the exhaustive preflight script
- prints the final parking-gate success marker
```

Important low-disk workflow note:

```text
Do not run both dev-quickchain-preflight.sh and dev-quickchain-park.sh for the same crate unless deliberately requesting duplicate full passes.

The park script delegates to preflight.
```

### Focused QuickChain Tests

Current focused QuickChain targets:

```text
crates/svc-storage/tests/quickchain_preflight_b3_integrity.rs
crates/svc-storage/tests/quickchain_preflight_boundary.rs
crates/svc-storage/tests/quickchain_preflight_docs.rs
crates/svc-storage/tests/quickchain_preflight_economics_quote.rs
crates/svc-storage/tests/quickchain_preflight_no_direct_mutation.rs
crates/svc-storage/tests/quickchain_preflight_observability.rs
crates/svc-storage/tests/quickchain_preflight_paid_cache.rs
crates/svc-storage/tests/quickchain_preflight_range_media.rs
crates/svc-storage/tests/quickchain_preflight_settlement_boundary.rs
crates/svc-storage/tests/quickchain_preflight_source_authority_scan.rs
crates/svc-storage/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-storage/tests/quickchain_tooling_boundary.rs
```

This gives `svc-storage` twelve focused QuickChain test targets in the current pass.

---

## What Each QuickChain Test Target Proves

### `quickchain_preflight_b3_integrity.rs`

This target proves byte identity remains canonical and content-derived.

It verifies:

```text
- caller cannot retrieve bytes under fake or noncanonical CID
- object ingest derives canonical b3 from bytes
```

This keeps b3 as content truth, not caller-provided authority.

### `quickchain_preflight_boundary.rs`

This target proves storage route/response shapes remain storage-shaped, not economic or QuickChain authority-shaped.

It verifies:

```text
- router exposes storage routes, not QuickChain authority routes
- public response shapes do not claim balance, receipt, root, or finality truth
```

This blocks storage from exposing runtime/settlement surfaces.

### `quickchain_preflight_docs.rs`

This target proves documentation stays synchronized with the crate’s intended QuickChain posture.

It verifies docs preserve the critical storage doctrine:

```text
- storage is bytes/b3/content infrastructure
- b3 is content truth only
- b3 is not payment proof
- cache cannot unlock paid content alone
- paid content requires backend-derived authorization/receipt
- receipt cache is display-only
- storage does not mutate wallet or ledger
- future QuickChain roots/checkpoints/validators/settlement remain parked
```

This matters because paid storage is a subtle boundary that future contributors could easily misunderstand.

### `quickchain_preflight_economics_quote.rs`

This target proves storage economics remain quote/admission/meters only, not mutation authority.

It verifies:

```text
- quote fields do not become wallet/ledger mutation fields
- storage quote behavior does not produce receipt truth
- storage cost/admission shapes do not imply final settlement
- money remains integer/minor-unit shaped where applicable
```

Storage can help price or enforce paid content, but it cannot become the wallet.

### `quickchain_preflight_no_direct_mutation.rs`

This target proves storage cannot directly mutate wallet/ledger state.

It verifies:

```text
- storage does not expose direct wallet or ledger mutation routes
- storage does not expose mint/issue/transfer/burn/hold/capture/release authority
- storage does not claim receipt/balance/finality/root truth
- storage config does not grow direct external settlement/bridge/anchor knobs
```

This keeps storage out of the economic mutation path.

### `quickchain_preflight_observability.rs`

This target proves metrics/logging/observability stay safe.

It verifies:

```text
- storage observability does not leak authority-shaped values
- metrics do not claim receipts, balances, roots, or finality
- observability remains derivative and operational
```

Storage metrics can inform accounting, debugging, and operations. They cannot become payout authority or economic proof.

### `quickchain_preflight_paid_cache.rs`

This target proves paid content cannot be unlocked by cache alone.

It verifies:

```text
- cache hit is not paid authorization
- display receipt cache is not spend authority
- paid unlock requires backend-derived authorization/receipt path
- offline/cache posture verifies b3 before trusted render but cannot replace wallet/ledger truth
```

This is one of the most important storage boundary tests because CrabLink will eventually rely on offline/cache UX.

### `quickchain_preflight_range_media.rs`

This target proves media serving remains bounded and honest.

It verifies:

```text
- range/segment media posture is bounded
- large media is not shoved through unbounded command/result paths
- each rendition owns its own b3 identity
- media delivery does not imply payment/finality truth
```

This preserves the CrabLink media doctrine: bounded/honest media, no DRM/anti-rip claims, and each rendition has its own content identity.

### `quickchain_preflight_settlement_boundary.rs`

This target proves storage settlement-shaped concepts do not become settlement authority.

It verifies:

```text
- storage may verify or carry backend-derived settlement/authorization references where appropriate
- storage does not produce settlement
- storage does not produce finality
- storage does not produce ledger roots
- storage does not replace wallet/ledger truth
```

This is intentionally careful: storage may need to interact with receipt/authorization metadata for paid content, but it must not author that truth.

### `quickchain_preflight_source_authority_scan.rs`

This target was added in the current pass.

It proves the source tree does not quietly grow runtime authority or external settlement surfaces.

It verifies:

```text
- svc-storage source tree does not define QuickChain runtime modules
- svc-storage source does not contain direct chain or external settlement tokens after stripping comments/string literals
```

This is a guardrail against future accidental imports, modules, or authority-shaped code paths.

The source scan is intentionally code-only: it strips comments and string literals before checking forbidden runtime tokens so docs/tests can still describe forbidden concepts.

### `quickchain_preflight_value_loop_boundary.rs`

This target proves storage stays in the correct place in the internal ROC value loop.

It verifies:

```text
- storage participates in paid enforcement and metering without becoming economic truth
- storage does not bypass svc-wallet
- storage does not replace ron-ledger
- storage outputs remain admission/storage/byte-serving shaped
```

This keeps storage as content infrastructure, not wallet/ledger authority.

### `quickchain_tooling_boundary.rs`

This target proves the crate-local QuickChain tooling remains safe and repeatable.

It verifies:

```text
- park script delegates to exhaustive preflight
- preflight script is Bash/cargo-only and keeps the full gate
- preflight script discovers all QuickChain tests dynamically
- no Python helpers are checked into the storage crate
```

This supports the current project workflow and prevents stale manual test lists.

---

## Current Verified Gate Results

The final parking run verified:

```text
Focused QuickChain tests discovered: 12

quickchain_preflight_b3_integrity.rs:         passed
quickchain_preflight_boundary.rs:             passed
quickchain_preflight_docs.rs:                 passed
quickchain_preflight_economics_quote.rs:      passed
quickchain_preflight_no_direct_mutation.rs:   passed
quickchain_preflight_observability.rs:        passed
quickchain_preflight_paid_cache.rs:           passed
quickchain_preflight_range_media.rs:          passed
quickchain_preflight_settlement_boundary.rs:  passed
quickchain_preflight_source_authority_scan.rs: passed
quickchain_preflight_value_loop_boundary.rs:  passed
quickchain_tooling_boundary.rs:               passed

cargo test -p svc-storage --all-targets:      passed
cargo clippy -p svc-storage --all-targets --no-deps -- -D warnings: passed

Final marker:
== svc-storage quickchain exhaustive preflight gate passed: tests=12 ==
== svc-storage QuickChain parking gate passed ==
```

The focused source-authority scan also passed before the park run:

```text
storage_source_tree_does_not_define_quickchain_runtime_modules: ok
storage_source_has_no_direct_chain_or_external_settlement_tokens: ok
```

---

## What svc-storage Now Proves

`svc-storage` now proves:

```text
- b3 identity is derived from bytes
- fake or noncanonical CIDs cannot retrieve bytes
- storage routes remain storage routes
- storage public responses do not claim balances, receipts, roots, or finality
- storage quotes remain quote/admission shapes
- storage does not expose direct wallet mutation
- storage does not expose direct ledger mutation
- storage observability remains operational/derivative
- paid cache cannot unlock content by itself
- offline/cache posture cannot replace backend authorization
- bounded range/media posture remains explicit
- each rendition owns its own b3
- storage settlement boundary remains verification/enforcement only, not authority
- source tree does not define QuickChain runtime modules
- source tree does not grow external settlement/bridge/validator/staking/liquidity logic
- tooling discovers and runs QuickChain gates dynamically
```

---

## Current Forbidden Scope Locked Out

`svc-storage` must continue to reject:

```text
roots
receipt roots
account state roots
hold roots
checkpoint roots
epoch roots
checkpoint production
validator behavior
settlement authority
finality claims
external anchors
bridge behavior
public-chain settlement
staking
liquidity
Solana path
ROX path
external L2/DA mutation path
direct ledger mutation
direct wallet mutation
fake balances
fake receipts
silent spend
cache-only paid unlock
b3-as-payment-proof
manifest-as-payment-proof
storage-admission-as-finality
```

---

## Cross-Crate Boundary After This Work

The intended value-loop position is now clearer:

```text
ron-proto:
  DTOs and strict wire shapes

ron-ledger:
  durable economic truth and replay

svc-wallet:
  mutation front-door and receipt production

svc-storage:
  b3 byte storage, paid admission/enforcement support, bounded media, metering input

ron-accounting:
  derivative snapshots and planning inputs

svc-rewarder:
  deterministic payout planning and wallet handoff intents

future QuickChain:
  eventually verifies canonical roots/proofs/checkpoints after locked vectors
```

The wrong shape remains forbidden:

```text
storage treats b3 as receipt.
storage treats cache as entitlement.
storage treats quote as payment.
storage treats admission as finality.
storage mutates wallet directly.
storage mutates ledger directly.
gateway/omnigate unlock paid content from cache alone.
CrabLink treats display receipt cache as spend authority.
```

---

## Completion Estimate For This Slice

For this crate’s current QuickChain Phase-0 / QC-1A boundary scope:

```text
svc-storage: 95–100% parked for this sweep
```

This does not mean all future storage economics/media/availability work is complete.

Remaining future work, after explicit later gates, may include:

```text
- deeper production paid-storage flows
- stronger storage/provider availability proofs
- refined metering feeds into accounting
- hardened large-media delivery paths
- future proof systems for availability/retrieval
- future canonical root/proof integration after locked vectors
```

But those are not required for this current boundary pass.

---

## Do Not Reopen Unless

Do not reopen this crate during the current pass unless:

```text
- terminal output shows a regression
- a later crate-pair exposes a missing storage boundary
- svc-wallet/ron-ledger paid authorization DTOs change
- gateway/omnigate paid access contracts change
- CrabLink cache/offline behavior needs a stricter backend signal
- QuickChain vector/root gates are explicitly opened later
```

---

## Regeneration Command

After this pass, regenerate the crate codebundle with:

```bash
bash scripts/make_crate_codex.sh --force -c svc-storage
```

---

### END NOTE - JUNE 20 2026 - QUICKCHAIN PHASE-0 / QC-1A - svc-storage

The pair is now cleanly documented as parked. Next is the cross-session carryover for **`svc-gateway + omnigate`**.


### END NOTE - JUNE 20 2026 - 11:30 CST


### BEGIN NOTE - JUNE 22 2026 - 11:40 CST

Below are paste-ready crate notes for `svc-rewarder + svc-storage`. The latest terminal confirms `svc-rewarder` parked with **10 focused QuickChain tests**, all-targets, clippy, and the parking gate passed; `svc-storage` parked with **13 focused QuickChain tests**, all-targets, clippy, and the parking gate passed.  

# QuickChain Crate Notes — `svc-rewarder + svc-storage`

Date: 2026-06-22
Project: RustyOnions / CrabLink
Track: QuickChain Phase 1 / QC-1A Round 1 foundation
Crate pair: `svc-rewarder + svc-storage`
Status: GREEN / parked for this QuickChain slice

---

## Summary

This session completed the `svc-rewarder + svc-storage` QuickChain QC-1A / Phase 1 Round 1 pair pass.

The goal of this pass was not to implement roots, checkpoints, validators, settlement, anchors, bridges, staking, liquidity, ROX, Solana, public-chain state, or external settlement logic.

The goal was to harden the service boundaries so:

```text
svc-rewarder remains deterministic payout planning only.
svc-storage remains bytes, b3 integrity, paid admission, range/media, and metering only.
svc-wallet remains the ROC mutation front-door.
ron-ledger remains durable economic truth.
ron-accounting remains derivative snapshot/accounting input, not balance truth.
QuickChain remains future settlement infrastructure.
```

Both crates are now parked green under their crate-local QuickChain parking gates.

---

## Files Added / Updated

### `svc-rewarder`

Added:

```text
crates/svc-rewarder/tests/quickchain_preflight_phase1_pair_interlock.rs
```

This new test target locks the QC-1A pair boundary for `svc-rewarder`.

It verifies that:

```text
- rewarder docs preserve rewarder as planner, not root authority
- manifests and settlement batches remain planning artifacts, not roots
- accounting and storage inputs cannot smuggle payout execution or root material
- backend wallet receipts may pass through wallet HTTP outcomes, but rewarder does not become receipt authority
- rewarder source tree does not gain root producer, validator, bridge, Solana, ROX, staking, liquidity, or external settlement runtime
```

No `Cargo.toml` changes were needed.

### `svc-storage`

Added / updated:

```text
crates/svc-storage/tests/quickchain_preflight_phase1_pair_interlock.rs
```

This new test target locks the QC-1A pair boundary for `svc-storage`.

It verifies that:

```text
- storage docs preserve storage as paid admission and metering only
- paid object responses remain storage admission + metering output, not reward/root/finality authority
- settlement adapter targets svc-wallet capture/release only and does not become ledger or chain authority
- accounting export remains metering only, not payout authorization
- storage runtime source tree does not gain root producer, validator, bridge, Solana, ROX, staking, liquidity, or external settlement runtime
```

A follow-up patch fixed one false positive in the `svc-storage` source authority scan.

The failure was caused by the string `solana` appearing inside a defensive negative unit-test fixture, not in production runtime code. The test was corrected so the runtime authority scan strips ordinary `#[cfg(test)] mod tests { ... }` blocks before scanning production source for forbidden runtime markers.

This keeps the test strict against real runtime authority creep while allowing unit tests to contain hostile/forbidden strings used as rejection fixtures.

No `Cargo.toml` changes were needed.

---

## `svc-rewarder` Boundary Locked In

`svc-rewarder` is now explicitly guarded as:

```text
deterministic payout planner
reward manifest producer
wallet-facing intent planner
policy/accounting consumer
idempotent replay/dedupe surface
```

It is explicitly not:

```text
ledger truth
wallet truth
receipt authority
balance authority
operation authority
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
staking/liquidity authority
public-chain authority
```

The crate now has a focused Phase 1 pair-interlock test proving rewarder cannot accidentally replace wallet/ledger truth.

Important protected concepts:

```text
rewarder output is plan/intent only
rewarder planning is deterministic for the same input
rewarder replay is dedupe/idempotency, not second payout authority
idempotency keys are retry dedupe, not durable operation identity
rewarder rejects smuggled authority fields
rewarder rejects raw engagement protocol-payout basis
rewarder requires explicit funding-source provenance
rewarder uses integer minor-unit money at the wire boundary
rewarder does not expose balance truth
rewarder does not expose receipt truth
rewarder does not expose operation truth
rewarder does not expose roots, finality, checkpoints, or validator claims
rewarder does not mutate ledger directly
rewarder does not bypass svc-wallet
rewarder routes mutation requests toward svc-wallet only
```

---

## `svc-storage` Boundary Locked In

`svc-storage` is now explicitly guarded as:

```text
canonical b3 byte storage
paid storage admission surface
wallet receipt evidence verifier
optional wallet capture/release adapter through svc-wallet
range/media byte serving surface
usage/metering source
accounting export source
```

It is explicitly not:

```text
ledger truth
wallet truth
receipt truth
balance truth
reward authority
payout authority
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
cache entitlement authority
staking/liquidity authority
public-chain authority
```

Important protected concepts:

```text
b3 hashes identify bytes only
cache is not paid-access authority
storage cannot unlock paid content from cache alone
storage paid-write responses are admission/metering evidence, not finality
wallet receipts are backend evidence, not storage-created truth
settlement adapter calls wallet capture/release only
storage never calls ron-ledger directly for economic mutation
accounting export is usage metering only
accounting export failure does not mutate wallet/ledger state
storage does not mint, allocate, or pay ROC
storage does not create reward roots or payout plans
storage does not create QuickChain roots/checkpoints/validator state
```

---

## Important False Positive Fixed

Initial `svc-storage` focused test result:

```text
storage_src_has_no_root_producer_validator_bridge_or_external_settlement_runtime ... FAILED
```

Cause:

```text
The broad runtime source scan matched the word "solana" inside a negative unit-test fixture.
```

Resolution:

```text
The scan now strips ordinary #[cfg(test)] mod tests { ... } blocks before checking runtime source text.
```

Why this is correct:

```text
The test is intended to detect production/runtime authority creep.
Defensive test fixtures may contain forbidden words to prove parser rejection.
Runtime source must remain free of Solana/ROX/bridge/external-settlement authority.
```

After the fix:

```text
cargo test -p svc-storage --test quickchain_preflight_phase1_pair_interlock
```

passed:

```text
5 passed; 0 failed
```

---

## Commands Run

Focused pair-interlock tests:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_phase1_pair_interlock
cargo test -p svc-storage --test quickchain_preflight_phase1_pair_interlock
```

Final parking gates:

```bash
crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The low-disk workflow was respected: focused tests first, then each park script once. We did not duplicate preflight + park unnecessarily.

---

## Final Verified Gate Results

### `svc-rewarder`

Final status:

```text
GREEN / parked
```

Focused QuickChain tests discovered:

```text
10
```

Discovered focused tests:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Additional gates:

```text
cargo test -p svc-rewarder --all-targets: passed
cargo clippy -p svc-rewarder --all-targets -- -D warnings: passed
```

Final markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=10 ==
== svc-rewarder QuickChain parking gate passed ==
```

Forbidden-scope marker preserved:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-rewarder remains deterministic payout planning only; svc-wallet remains mutation front-door; ron-ledger remains truth
```

### `svc-storage`

Final status:

```text
GREEN / parked
```

Focused QuickChain tests discovered:

```text
13
```

Discovered focused tests:

```text
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Additional gates:

```text
cargo test -p svc-storage --all-targets: passed
cargo clippy -p svc-storage --all-targets -- -D warnings: passed
```

Final markers:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=13 ==
== svc-storage QuickChain parking gate passed ==
```

Forbidden-scope marker preserved:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-storage remains bytes by canonical b3 only; cache is not paid-access authority; wallet/ledger truth stays backend-owned
```

---

## Why This Pair Matters

This pair is important because it protects the middle of the internal ROC value loop.

`svc-storage` creates paid storage/access/metering signals. Those signals are useful inputs for accounting and future reward planning, but they must never directly unlock paid access from cache alone and must never directly create payout authority.

`svc-rewarder` consumes accounting/policy-shaped information and produces deterministic payout plans. Those plans may become wallet-facing requests, but they are not receipts, not balance truth, not ledger truth, not finality, and not QuickChain roots.

The pair now proves the intended split:

```text
storage emits bytes + paid admission + metering evidence
accounting derives snapshots
rewarder plans payouts
wallet executes approved ROC mutations
ledger remains durable truth
```

---

## Current QuickChain Pair Progress

Current QC-1A / Phase 1 Round 1 pair order:

```text
1. ron-proto + ron-ledger          ✅ completed for this slice
2. svc-wallet + ron-accounting     ✅ completed for this slice
3. svc-rewarder + svc-storage      ✅ completed for this slice
4. svc-gateway + omnigate          ← next
5. svc-index + ron-policy
6. CrabLink Tauri + client adapters
```

The next crate pair is:

```text
svc-gateway + omnigate
```

The next pair should focus on public route/admission and hydration/access composition boundaries.

---

## Next Pair Preview: `svc-gateway + omnigate`

Next mission:

```text
Harden svc-gateway and omnigate so they can participate in paid access and hydration flows without becoming wallet truth, ledger truth, receipt truth, balance truth, storage truth, cache authority, root authority, checkpoint authority, validator authority, bridge authority, external settlement authority, finality authority, staking/liquidity authority, or public-chain authority.
```

Expected next-session focus:

```text
svc-gateway = public boundary / route / admission / proxy / fail-closed surface
omnigate = product hydration / access composition / backend-derived display surface
```

Do not add:

```text
roots
checkpoints
validators
committee/quorum logic
settlement finality
anchors
bridges
ROX
Solana
staking
liquidity
exchange-facing logic
public-chain state
cache-only paid unlock
fake receipts
fake balances
fake finality
direct wallet mutation from the client/product layer
direct ledger mutation from gateway/omnigate
```

---

## Recommended Cleanup Before Next Pair

Because the park gates compiled many tests and dependencies, this is a good cleanup point:

```bash
cargo clean
```

Then regenerate focused codebundles for the next pair:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Recommended files to attach for the next session:

```text
QUICKCHAIN_BUILDPLAN.MD
QUICKCHAIN_REVIEW_BUNDLE.MD
CODEBUNDLE_RS.md
latest carryover notes
latest terminal output
crates/svc-gateway/CODEBUNDLE.md
crates/omnigate/CODEBUNDLE.md
```

---

## One-Line Handoff

`svc-rewarder + svc-storage` are now green/parked for QuickChain QC-1A / Phase 1 Round 1: rewarder is locked as deterministic payout planning only, storage is locked as b3 bytes/paid admission/metering only, wallet/ledger remain economic truth, and the next crate pair is `svc-gateway + omnigate`.


### END NOTE - JUNE 22 2026 - 11:40 CST



### BEGIN NOTE - JUNE 22 2026 - 18:35 CST

The terminal evidence supports these notes: `svc-rewarder` passed its Phase 1 Round 2 confirmation and parking gate, and `svc-storage` passed the fixed confirmation test plus its full parking gate with 14 focused QuickChain tests.  

# QuickChain Phase 1 Crate Notes — `svc-rewarder` + `svc-storage`

Date: June 22, 2026
Scope: QuickChain Phase 1, Round 2
Crate pair:

```text
svc-rewarder + svc-storage
```

## Status Summary

`svc-rewarder` and `svc-storage` are now complete for QuickChain Phase 1.

Final pair status:

```text
svc-rewarder:
  Phase 1 Round 1: COMPLETE
  Phase 1 Round 2: COMPLETE
  Pair status: PHASE 1 COMPLETE / PARKED

svc-storage:
  Phase 1 Round 1: COMPLETE
  Phase 1 Round 2: COMPLETE
  Pair status: PHASE 1 COMPLETE / PARKED
```

This does not mean all of QuickChain Phase 1 is complete. It means this crate pair is done for Phase 1 and should not be touched again unless a later cross-crate audit exposes a real integration mismatch.

Remaining Phase 1 Round 2 pairs after this:

```text
svc-gateway + omnigate
svc-index + ron-policy
CrabLink Tauri + client adapters
```

## What This Session Accomplished

This session completed the downstream Phase 1 Round 2 confirmation pass for `svc-rewarder` and `svc-storage`.

The goal was not to make either crate a QuickChain runtime. The goal was to confirm that both crates can safely coexist with Phase 1 deterministic root/proof material while preserving strict service authority boundaries.

The pair now confirms:

```text
svc-rewarder remains deterministic payout planning only.
svc-storage remains canonical b3 byte/artifact storage only.
reward plans and manifests may be referenced as artifacts.
storage CIDs may reference vector/root/proof artifacts as bytes.
artifact CIDs are not QuickChain roots.
artifact storage is not finality.
rewarder planning is not ledger mutation.
storage admission is not paid-access authority by itself.
svc-wallet remains the economic mutation front-door.
ron-ledger remains durable economic truth.
```

## `svc-rewarder` Notes

### Role

`svc-rewarder` remains deterministic reward planning infrastructure.

Its valid role is:

```text
ron-accounting snapshot / metering material
  -> deterministic reward planning
  -> reward manifest / payout plan artifact
  -> explicit wallet path later if approved
```

It is not:

```text
a wallet
a ledger
a root producer
a checkpoint producer
a validator
a bridge
an external settlement adapter
a finality authority
a direct payout executor
```

### Phase 1 Round 2 Confirmation

The new/confirmed Round 2 test target is:

```text
crates/svc-rewarder/tests/quickchain_phase1_round2_confirmation.rs
```

It confirms:

```text
docs name the Phase 1 Round 2 downstream confirmation boundary
reward manifest commitments are referenceable artifacts, not root authority
accounting and policy inputs remain planning inputs, not balance/root truth
wallet egress remains explicit wallet issue handoff, not rewarder mutation truth
```

The focused confirmation test passed:

```text
cargo test -p svc-rewarder --test quickchain_phase1_round2_confirmation
```

Result:

```text
4 passed; 0 failed
```

The full rewarder park gate also passed:

```text
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The park gate confirmed:

```text
focused QuickChain tests discovered dynamically
all focused QuickChain tests passed
all-targets tests passed
clippy passed with -D warnings
forbidden-scope marker preserved
parking gate passed
```

### Protected Invariants

`svc-rewarder` now has Phase 1 protection for:

```text
no direct ledger mutation
no hidden wallet mutation
no fake balances
no fake receipts
no fake finality
no root-producing rewarder runtime
no checkpoint authority
no validator behavior
no bridge or external settlement
no staking
no liquidity
no ROX/Solana path
no raw engagement protocol ROC minting
```

### Important Design Boundary

Rewarder output can be treated as deterministic planning material or artifact material.

It must not be treated as:

```text
balance truth
receipt truth
ledger truth
QuickChain root truth
settlement finality
validator approval
protocol payout execution
```

The mental model remains:

```text
ron-accounting measures.
svc-rewarder plans.
svc-wallet mutates.
ron-ledger is truth.
QuickChain later verifies roots.
```

## `svc-storage` Notes

### Role

`svc-storage` remains byte/object infrastructure.

Its valid role is:

```text
canonical b3 object storage
bounded read/range media serving
paid write admission evidence handling
storage metering
artifact byte retention
```

It is not:

```text
a wallet
a ledger
a receipt authority
a paid-unlock authority by itself
a root producer
a checkpoint producer
a validator
a bridge
an external settlement adapter
a finality oracle
```

### Phase 1 Round 2 Confirmation

The new/confirmed Round 2 test target is:

```text
crates/svc-storage/tests/quickchain_phase1_round2_confirmation.rs
```

It confirms:

```text
docs name the Phase 1 Round 2 storage artifact boundary
storage can store and retrieve Phase 1 artifacts by canonical b3
object routes and storage remain content-addressed byte paths, not chain authority
paid/accounting storage sources do not turn artifacts into unlock or finality authority
```

The focused confirmation test passed after the marker-string fix:

```text
cargo test -p svc-storage --test quickchain_phase1_round2_confirmation
```

Result:

```text
4 passed; 0 failed
```

The full storage park gate also passed:

```text
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The park gate confirmed:

```text
14 focused QuickChain tests discovered dynamically
all focused QuickChain tests passed
all-targets tests passed
clippy passed with -D warnings
forbidden-scope marker preserved
parking gate passed
```

### Specific Fix Applied

The only follow-up fix needed was in the storage confirmation test.

The original marker expected the literal phrase:

```text
b3:<64 lowercase hex>
```

inside stripped source text. Since the source scanner stripped comments and the real enforcement lived in code-level guards, the test was corrected to check actual source guards instead:

```text
starts_with("b3:")
3 + 64
b'a'..=b'f'
```

This better reflects the runtime source boundary: canonical b3 is enforced by code, not by a comment-only marker.

### Protected Invariants

`svc-storage` now has Phase 1 protection for:

```text
canonical b3 object identity
content-addressed object reads
bounded range reads
storage artifact retrieval by b3
paid write proof requirement
wallet-derived paid evidence
accounting export as metering only
cache cannot unlock paid content alone
no balance mutation
no direct ledger mutation
no fake balances
no fake receipts
no fake finality
no root-producing storage runtime
no checkpoint authority
no validator behavior
no bridge or external settlement
no staking
no liquidity
no ROX/Solana path
```

### Important Design Boundary

Storage may retain bytes that represent future QuickChain vectors, roots, or proofs.

That does not make storage:

```text
the root authority
the verifier
the validator
the finality oracle
the wallet
the ledger
the paid-access authority
```

The correct model is:

```text
storage stores bytes by b3.
b3 identifies bytes.
QuickChain semantics live elsewhere.
wallet/ledger truth lives elsewhere.
paid unlock requires backend-derived receipt/proof flow.
```

## Files Changed / Added

The effective Phase 1 Round 2 work for this pair centered on:

```text
crates/svc-rewarder/tests/quickchain_phase1_round2_confirmation.rs
crates/svc-storage/tests/quickchain_phase1_round2_confirmation.rs
crates/svc-rewarder/docs/quickchain-preflight.md
crates/svc-storage/docs/quickchain-preflight.md
```

The changes were intentionally downstream-light and boundary-focused. They did not add root production, checkpoint production, validator logic, external settlement logic, bridge logic, staking, liquidity, ROX, Solana, fake receipts, fake balances, or direct ledger mutation.

## Validation Commands Run

Focused tests:

```bash
cargo test -p svc-rewarder --test quickchain_phase1_round2_confirmation
cargo test -p svc-storage --test quickchain_phase1_round2_confirmation
```

Parking gates:

```bash
crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

Final observed status:

```text
svc-rewarder QuickChain parking gate passed
svc-storage QuickChain parking gate passed
```

## Completion Decision

`svc-rewarder + svc-storage` should now be marked:

```text
QuickChain Phase 1 complete / parked for this crate pair.
```

Do not continue patching this pair during Phase 1 unless later work in `svc-gateway + omnigate`, `svc-index + ron-policy`, CrabLink Tauri, or the final cross-crate audit exposes a concrete mismatch.

## Next Crate Pair

The next QuickChain Phase 1 Round 2 pair should be:

```text
svc-gateway + omnigate
```

The next pair should prove:

```text
svc-gateway remains public boundary / paid enforcement boundary only.
omnigate remains hydration/enforcement orchestration only.
gateway and omnigate may surface backend-derived proof/root/vector references.
gateway and omnigate must not mutate ledger or wallet truth.
gateway and omnigate must not produce roots/checkpoints.
gateway and omnigate must not claim finality.
gateway and omnigate must not unlock paid content from cache alone.
wallet/ledger receipts remain backend truth.
```

Use `QUICKCHAIN_BUILDPLAN.MD` as the phase/round reference and `QUICKCHAIN_REVIEW_BUNDLE.MD` as the master blueprint before starting the next pair.


### END NOTE - JUNE 22 2026 - 18:35 CST


### BEGIN NOTE - JUNE 23 2026 - 19:30 CST

Below are paste-ready crate notes. The terminal evidence shows the Phase 2 Round 1 patch applied, both focused `quickchain_phase2_replay_boundary` tests passed, `svc-rewarder` discovered/passed **12** focused QuickChain tests, `svc-storage` discovered/passed **15**, and both parking gates passed.  The prior carryover target for this pair was exactly to prove read-only verifier artifact handling without rewarder/storage becoming verifier, finality, committee, payout execution, settlement, or paid-unlock authority. 

Add this to `crates/svc-rewarder/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-rewarder

# QuickChain Crate Notes — svc-rewarder

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-rewarder`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-rewarder` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-rewarder COMPLETE / PARKED
```

This means `svc-rewarder` has been hardened for the Phase 2 Round 1 verifier-artifact/read-only replication slice.

It does not mean all of Phase 2 is complete. It does not mean committee signing, quorum/fork-choice, validator readiness, staking, slashing, bridge behavior, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `svc-rewarder` can safely coexist with Phase 2 read-only verifier artifacts without becoming QuickChain authority.

The crate remains:

```text
deterministic payout planning only
reward manifest / planning artifact producer
accounting and policy input consumer
non-mutating payout planner
wallet handoff planner only
```

The crate does not become:

```text
verifier authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
ledger mutation authority
wallet mutation authority
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
svc-rewarder plans payouts; svc-wallet commits approved payout intents; ron-ledger records durable economic truth.
```

---

## 2. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-rewarder/tests/quickchain_phase2_replay_boundary.rs
```

This test locks the Phase 2 Round 1 boundary:

```text
reward manifests may become read-only verifier artifact inputs
reward manifests remain planning artifacts
rewarder does not sign committee votes
rewarder does not decide quorum
rewarder does not claim fork choice
rewarder does not claim finality
rewarder still cannot mutate ledger truth
svc-wallet commits approved payout intents
ron-ledger remains durable economic truth
```

The focused test passed:

```text
cargo test -p svc-rewarder --test quickchain_phase2_replay_boundary

running 3 tests
docs_name_phase2_round1_read_only_verifier_boundary ... ok
reward_outputs_are_replay_artifact_inputs_not_committee_authority ... ok
rewarder_does_not_turn_replay_artifacts_into_wallet_or_ledger_mutation_truth ... ok

test result: ok. 3 passed; 0 failed
```

---

## 3. Documentation Updated

Updated:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

The doc now explicitly names the Phase 2 Round 1 status:

```text
Phase 2 Round 1 verifier artifact / read-only replication
```

The doc now preserves these boundaries:

```text
reward manifests may become read-only verifier artifact inputs
reward manifests are not committee votes
reward manifests are not quorum decisions
reward manifests are not fork choice
reward manifests are not finality
reward manifests are not validator signatures
reward manifests are not balance truth
reward manifests are not direct payout execution
```

The doc also keeps the forbidden scope clear:

```text
no committee signing
no quorum/fork-choice
no validator signatures
no staking
no slashing
no public bridge
no external settlement
no ROX
no Solana
no direct ledger mutation
no fake receipts
no fake balances
no fake finality
```

---

## 4. Final Gate Evidence

The final local gate discovered 12 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=12 ==
== svc-rewarder QuickChain parking gate passed ==
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

The all-targets test gate completed cleanly:

```text
cargo test -p svc-rewarder --all-targets
```

---

## 5. Important Existing Coverage Still Preserved

This pass did not replace older rewarder safety coverage. It extended it.

Still protected:

```text
reward manifest does not expose roots, receipts, balances, or finality
wallet preview is issue-request shape, not receipt or balance truth
compute request rejects smuggled QuickChain authority fields
funding source is explicit
raw engagement cannot directly allocate protocol ROC
duplicate epoch replay is dedupe, not second payout authority
idempotency keys are retry dedupe, not operation identity
same snapshot/policy/epoch produces same plan commitment
reordered snapshot rows produce same plan
source tree does not define QuickChain runtime modules
source has no direct chain or ledger authority tokens
wallet client boundary targets wallet without direct ledger or chain authority
```

---

## 6. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-rewarder is green/parked for QuickChain Phase 2 Round 1.
It can reference or produce planning artifacts that may be consumed as read-only verifier evidence,
but it cannot become verifier, finality, committee, settlement, payout execution, wallet, or ledger authority.
```

---

## 7. What Not To Claim

Do not claim:

```text
Phase 2 complete
Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
reward execution live from verifier artifacts
rewarder can commit wallet payouts directly
rewarder can mutate ledger
external settlement ready
bridge ready
staking/slashing ready
```

---

## 8. Do Not Reopen Unless

Do not reopen `svc-rewarder` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in rewarder verifier-artifact boundaries
svc-wallet approved payout intent DTOs change
ron-ledger economic truth or receipt format changes
ron-accounting snapshot format changes
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source scanner catches new authority creep
```

Otherwise, `svc-rewarder` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 9. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-gateway + omnigate
```

Expected goal for that pair:

```text
public boundary / hydration / paid-enforcement surfaces may display or route backend-derived verifier/readiness artifacts,
but cannot become verifier, finality, settlement, wallet mutation, paid unlock, or chain authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-rewarder

Add this to `crates/svc-storage/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-storage

# QuickChain Crate Notes — svc-storage

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-storage`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-storage` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-storage COMPLETE / PARKED
```

This means `svc-storage` has been hardened for the Phase 2 Round 1 verifier-artifact/read-only replication slice.

It does not mean all of Phase 2 is complete. It does not mean storage is a verifier, finality system, payment truth system, committee participant, settlement layer, bridge, or public-chain runtime.

---

## 1. What This Pass Proved

This pass proved that `svc-storage` can safely retain and retrieve Phase 2 read-only verifier artifacts as bytes by canonical b3 without becoming QuickChain authority.

The crate remains:

```text
bytes/artifacts/manifests storage
b3 integrity surface
paid admission support surface
metering source
artifact persistence layer
bounded range/media byte path
```

The crate does not become:

```text
verifier authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
payment truth
wallet mutation authority
ledger mutation authority
paid-unlock authority by cache alone
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
b3 hashes prove bytes; they do not prove payment, finality, settlement, or access authority by themselves.
```

---

## 2. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-storage/tests/quickchain_phase2_replay_boundary.rs
```

This test locks the Phase 2 Round 1 boundary:

```text
storage may retain read-only verifier artifact bytes by canonical b3
storage may retrieve read-only verifier artifact bytes by canonical b3
artifact cids are byte references, not verifier authority
b3 proves bytes, not balance truth
b3 proves bytes, not paid access
b3 proves bytes, not finality
storage cannot decide quorum
storage cannot sign committee votes
storage cannot claim fork choice
storage cannot claim finality
storage cannot mutate replay outcomes
storage cannot unlock paid content from cache alone
wallet/ledger receipts remain backend truth
```

The focused test passed:

```text
cargo test -p svc-storage --test quickchain_phase2_replay_boundary

running 3 tests
docs_name_phase2_round1_storage_verifier_artifact_boundary ... ok
storage_can_archive_and_retrieve_read_only_verifier_artifacts_by_b3 ... ok
storage_replay_artifact_paths_do_not_become_verifier_or_paid_unlock_authority ... ok

test result: ok. 3 passed; 0 failed
```

---

## 3. Documentation Updated

Updated:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

The doc now explicitly names the Phase 2 Round 1 status:

```text
Phase 2 Round 1 verifier artifact / read-only replication
```

The doc now preserves these boundaries:

```text
storage may retain read-only verifier artifact bytes by canonical b3
storage may retrieve read-only verifier artifact bytes by canonical b3
artifact cids are byte references, not verifier authority
b3 proves bytes, not balance truth
b3 proves bytes, not paid access
b3 proves bytes, not finality
storage cannot decide quorum
storage cannot sign committee votes
storage cannot claim fork choice
storage cannot claim finality
storage cannot mutate replay outcomes
storage cannot unlock paid content from cache alone
wallet/ledger receipts remain backend truth
```

The doc also keeps the forbidden scope clear:

```text
no committee signing
no quorum/fork-choice
no validator signatures
no staking
no slashing
no public bridge
no external settlement
no ROX
no Solana
no direct wallet mutation
no direct ledger mutation
no fake receipts
no fake balances
no fake finality
no cache-only paid unlock
```

---

## 4. Final Gate Evidence

The final local gate discovered 15 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=15 ==
== svc-storage QuickChain parking gate passed ==
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings
```

The all-targets test gate completed cleanly:

```text
cargo test -p svc-storage --all-targets
```

---

## 5. Important Existing Coverage Still Preserved

This pass did not replace older storage safety coverage. It extended it.

Still protected:

```text
storage stores/retrieves Phase 1 artifacts by canonical b3
object routes and store remain content-addressed byte paths, not chain authority
paid/accounting storage sources do not turn artifacts into unlock or finality authority
object ingest derives canonical b3 from bytes
caller cannot retrieve bytes under fake or noncanonical cid
public response shapes do not claim balance, receipt, root, or finality truth
paid estimate is quote-only and integer minor units
storage does not mutate wallet or ledger directly
accounting export is metering, not balance truth
metrics stay low-cardinality and non-authoritative
fake cache or paid headers do not unlock absent objects
paid write rejects without backend-derived proof and does not cache bytes
range media path serves canonical b3 and bounded ranges only
settlement plan is integer, bounded, and deterministic without roots or finality
settlement source uses wallet front door only and no chain authority
source tree does not define QuickChain runtime modules
source has no direct chain or external settlement tokens
```

---

## 6. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-storage is green/parked for QuickChain Phase 2 Round 1.
It can retain and retrieve read-only verifier artifacts as canonical b3 bytes,
but those bytes do not become verifier, finality, committee, payment, settlement, wallet, ledger, or paid-unlock truth.
```

---

## 7. What Not To Claim

Do not claim:

```text
Phase 2 complete
Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
storage proves payment
storage proves settlement
storage unlocks paid content from cache
storage can mutate wallet
storage can mutate ledger
external settlement ready
bridge ready
staking/slashing ready
```

---

## 8. Do Not Reopen Unless

Do not reopen `svc-storage` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in storage verifier-artifact boundaries
svc-wallet receipt / paid-admission DTOs change
ron-ledger economic truth or receipt format changes
svc-gateway / omnigate paid enforcement contracts change
CrabLink cache/offline behavior needs a stricter backend signal
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source scanner catches new authority creep
```

Otherwise, `svc-storage` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 9. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-gateway + omnigate
```

Expected goal for that pair:

```text
public boundary / hydration / paid-enforcement surfaces may display or route backend-derived verifier/readiness artifacts,
but cannot become verifier, finality, settlement, wallet mutation, paid unlock, or chain authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-storage


### END NOTE - JUNE 23 2026 - 19:30 CST



### BEGIN NOTE - JUNE 24 2026 - 01:00 CST

The terminal confirms both crate-local park gates passed after the Phase 2 Round 2 committee-boundary patch, so these notes can be dropped into the two crate `NOTES.MD` files. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-rewarder — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-rewarder` is **parked for QuickChain Phase 2 Round 2**.

This round added the Phase 2 Round 2 **committee-readiness boundary** for `svc-rewarder` without implementing committee runtime, validator runtime, quorum logic, fork choice, finality, roots, checkpoints, bridge, staking, slashing, liquidity, or external settlement.

The crate remains exactly where it belongs in the RustyOnions/QuickChain value loop:

```text
ron-accounting snapshots / policy inputs
→ svc-rewarder deterministic payout planning
→ explicit svc-wallet issue-request handoff previews
→ svc-wallet commits approved payout intents
→ ron-ledger remains durable economic truth
```

`svc-rewarder` is still a deterministic payout planner only. It does not mutate ledger truth and does not become committee, finality, settlement, or validator-economy authority.

## What changed in this round

Added the new focused Phase 2 Round 2 committee-boundary test:

```text
crates/svc-rewarder/tests/quickchain_phase2_committee_boundary.rs
```

Updated the QuickChain preflight documentation with a new Phase 2 Round 2 marker section:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

No `Cargo.toml` changes were required.

## Boundary locked by this patch

The new test locks the following `svc-rewarder` rules:

```text
- reward manifests remain payout planning artifacts
- wallet issue requests remain explicit svc-wallet handoff previews
- svc-rewarder is not a committee member
- svc-rewarder does not produce signed verification attestations
- svc-rewarder does not decide quorum
- svc-rewarder cannot claim fork choice
- svc-rewarder cannot claim finality
- svc-rewarder cannot create validator rewards from raw engagement
- svc-rewarder cannot mutate ledger truth
- svc-wallet commits approved payout intents
- ron-ledger remains durable economic truth
```

## New test coverage added

The new test suite verifies:

```text
docs_name_phase2_round2_committee_readiness_boundary
rewarder_wire_edges_reject_committee_attestation_poison_fields
rewarder_preserves_planning_and_wallet_handoff_seams_without_committee_authority
rewarder_source_does_not_implement_committee_or_validator_economy_runtime
serialized_rewarder_handoff_dtos_have_no_committee_authority_keys
```

Important coverage details:

```text
- WalletIssueRequest continues to reject unknown committee/quorum/finality/validator fields.
- Rewarder manifest Attestation continues to reject unknown committee/quorum/finality/validator fields.
- Rewarder output and wallet handoff seams remain explicit planning/egress DTO seams.
- Source scans prevent committee authority fields from creeping into svc-rewarder runtime code.
- Serialized wallet issue handoff DTOs expose no committee-authority keys.
```

## Commands proven green

The following focused command passed:

```bash
cargo test -p svc-rewarder --test quickchain_phase2_committee_boundary
```

Result:

```text
5 passed; 0 failed
```

The crate-local parking gate also passed:

```bash
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The park gate discovered and ran 13 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The park gate also ran:

```text
svc-rewarder all-targets test
svc-rewarder clippy
forbidden-scope marker check
```

All passed.

## Final forbidden-scope marker

The park gate confirmed:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-rewarder remains deterministic payout planning only; svc-wallet remains mutation front-door; ron-ledger remains truth
```

## Phase 2 Round 2 verdict

`svc-rewarder` is complete for this Phase 2 Round 2 pass.

The crate now has explicit committee-readiness boundary coverage proving that future committee/verifier artifacts may use rewarder outputs as deterministic replay/planning inputs, but `svc-rewarder` itself remains non-authoritative over:

```text
- committee membership
- signed verification attestations
- quorum
- fork choice
- finality
- validator rewards
- raw engagement protocol payouts
- ledger mutation
- roots/checkpoints
- external settlement
```

## Next time this crate is touched

Do not add committee runtime here.

Future safe work for later phases may include:

```text
- making rewarder artifacts easier for independent verifiers to consume
- stronger deterministic artifact shape checks
- tighter replay package compatibility checks
- additional anti-smuggling tests around payout policy and accounting snapshots
```

Unsafe or out-of-scope work remains forbidden:

```text
- no direct ledger mutation
- no direct wallet mutation outside explicit wallet handoff path
- no root/checkpoint production
- no quorum certificates
- no validator signatures
- no fork-choice/finality authority
- no staking/slashing/bonded economics
- no bridge/external settlement
- no protocol ROC payouts from raw engagement
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-storage — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-storage` is **parked for QuickChain Phase 2 Round 2**.

This round added the Phase 2 Round 2 **committee-readiness boundary** for `svc-storage` without implementing committee runtime, validator runtime, quorum logic, fork choice, finality, roots, checkpoints, bridge, staking, slashing, liquidity, or external settlement.

The crate remains exactly where it belongs in the RustyOnions/QuickChain value loop:

```text
svc-storage
→ stores bytes/artifacts by canonical b3
→ supports paid storage admission and metering
→ may retain replay/committee-readiness artifact bytes
→ does not decide verifier agreement or paid unlock truth
→ wallet/ledger receipts remain backend truth
```

`svc-storage` remains byte/artifact infrastructure only. It is not payment truth, balance truth, committee truth, finality truth, or validator authority.

## What changed in this round

Added the new focused Phase 2 Round 2 committee-boundary test:

```text
crates/svc-storage/tests/quickchain_phase2_committee_boundary.rs
```

Updated the QuickChain preflight documentation with a new Phase 2 Round 2 marker section:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

No `Cargo.toml` changes were required.

## Boundary locked by this patch

The new test locks the following `svc-storage` rules:

```text
- svc-storage stores committee/replay artifacts as bytes only
- storage is not a committee member
- storage does not produce signed verification attestations
- storage does not decide quorum
- storage does not claim fork choice
- storage does not claim finality
- b3 proves byte integrity, not committee agreement
- artifact cids are byte references, not verifier authority
- cache cannot unlock paid content alone
- wallet/ledger receipts remain backend truth
```

## New test coverage added

The new test suite verifies:

```text
docs_name_phase2_round2_committee_readiness_boundary
storage_can_hold_committee_readiness_artifact_bytes_without_interpreting_authority
storage_source_preserves_byte_storage_and_paid_boundary_seams
storage_source_does_not_implement_committee_or_validator_economy_authority
storage_boundary_has_no_committee_authority_keys_in_paid_or_policy_runtime_source
```

Important coverage details:

```text
- MemoryStorage can store and retrieve committee-readiness artifact bytes by canonical b3.
- The stored artifact payload may contain opaque committee-looking text, but storage treats it only as bytes.
- b3 integrity is checked as byte truth only, not committee agreement.
- Storage source seams preserve canonical b3, get_full, get_range, wallet settlement adapter, and accounting export boundaries.
- Source scans prevent committee/quorum/finality/validator-economy authority markers from entering storage runtime code.
- Paid, policy, and accounting source paths do not accept committee-authority keys.
```

## Commands proven green

The following focused command passed:

```bash
cargo test -p svc-storage --test quickchain_phase2_committee_boundary
```

Result:

```text
5 passed; 0 failed
```

The crate-local parking gate also passed:

```bash
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The park gate discovered and ran 16 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The park gate also ran:

```text
svc-storage all-targets test
svc-storage clippy
forbidden-scope marker check
```

All passed.

## Final forbidden-scope marker

The park gate confirmed:

```text
no roots; no checkpoints; no validators; no settlement; no anchors; no bridges; no staking; no liquidity
svc-storage remains bytes by canonical b3 only; cache is not paid-access authority; wallet/ledger truth stays backend-owned
```

## Phase 2 Round 2 verdict

`svc-storage` is complete for this Phase 2 Round 2 pass.

The crate now has explicit committee-readiness boundary coverage proving that it may store replay/committee-readiness artifacts as canonical b3-addressed bytes, but it must never become authoritative over:

```text
- committee membership
- signed verification attestations
- quorum
- fork choice
- finality
- paid unlock
- wallet receipts
- ledger receipts
- validator rewards
- roots/checkpoints
- external settlement
```

## Next time this crate is touched

Do not add committee runtime here.

Future safe work for later phases may include:

```text
- storing verifier/replay artifacts as opaque content-addressed bytes
- improving artifact metadata readability without granting authority
- stronger b3/range/integrity checks around replay artifact storage
- additional paid-cache regression tests
- clearer archive/retention policy once DA/challenge/archive fallback is explicitly in scope
```

Unsafe or out-of-scope work remains forbidden:

```text
- no direct wallet mutation
- no direct ledger mutation
- no cache-only paid unlock
- no root/checkpoint production
- no quorum certificates
- no validator signatures
- no fork-choice/finality authority
- no staking/slashing/bonded economics
- no bridge/external settlement
- no pruning before DA/challenge/archive fallback is proven
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2


### END NOTE - JUNE 24 2026 - 01:00 CST



### BEGIN NOTE - JUNE 24 2026 - 12:55 CST

Here are paste-ready crate notes for `svc-rewarder + svc-storage`. The terminal output confirms both crate-local exhaustive preflight gates passed: `svc-rewarder` with 14 focused QuickChain tests and `svc-storage` with 17 focused QuickChain tests, plus all-target tests and clippy. 

## QuickChain Phase 3 Round 1 Notes — svc-rewarder

### Status

`svc-rewarder` is complete for QuickChain Phase 3 Round 1.

The crate now has an explicit Phase 3 validator/passport boundary test layer proving that `svc-rewarder` remains deterministic payout planning only. It can compute reward manifests, generate planning artifacts, and shape wallet handoff previews/intents, but it does not become validator authority, passport registry authority, wallet authority, ledger authority, staking authority, slashing authority, settlement authority, or QuickChain runtime authority.

### Phase 3 Scope Completed

Added Phase 3 validator/passport boundary coverage for:

* compute requests rejecting validator/passport authority poison fields
* nested accounting snapshots rejecting validator/passport authority poison fields
* nested reward policy DTOs rejecting validator/passport authority poison fields
* reward manifests remaining planning artifacts, not validator membership or passport authority
* reward manifests rejecting validator/passport authority poison fields
* wallet handoff preview staying wallet issue-request shape only
* source tree avoiding validator admission, revocation, rotation, staking, slashing, registry proof handling, direct ledger mutation, and validator-economy runtime
* manifest avoiding `svc-passport`, `svc-registry`, `ron-auth`, and `ron-ledger` authority dependencies

### Files Touched

* `crates/svc-rewarder/tests/quickchain_phase3_validator_boundary.rs`
* `crates/svc-rewarder/docs/quickchain-preflight.md`

### Important Boundaries Preserved

`svc-rewarder` remains:

* deterministic reward planner
* payout manifest producer
* wallet handoff planner
* funding-source-aware policy consumer
* accounting snapshot consumer
* non-authoritative planning layer

`svc-rewarder` does not become:

* wallet mutation authority
* direct ledger mutation authority
* balance truth
* receipt truth
* validator identity authority
* passport registry authority
* validator capability authority
* validator-set authority
* staking/bonding/slashing authority
* QuickChain root/checkpoint/finality producer
* bridge/external settlement authority

### Phase 3 Design Decision

For Phase 3 Round 1, validator/passport material is rejected at rewarder boundaries instead of being accepted as metadata.

That means fields such as validator set hash, validator capability, passport subject, registry proof, bond requirement, bonded economics, staking power, or slash evidence cannot be smuggled into rewarder compute requests, reward policies, accounting snapshots, reward manifests, or wallet handoff preview shapes.

This keeps reward planning separate from validator membership and prevents validator identity from becoming payout execution authority.

### Test Results

Focused Phase 3 tests passed:

* `quickchain_phase3_validator_boundary` — 7 passed

Full preflight passed:

* discovered focused QuickChain tests: 14
* all 14 focused QuickChain tests passed
* all-target tests passed
* unit tests passed
* integration tests passed
* bench smoke passed
* clippy passed

Final gate:

`svc-rewarder quickchain exhaustive preflight gate passed: tests=14`

### Remaining Risks / Deferred Work

No passport, registry, auth, validator lifecycle, bond, staking, slashing, bridge, external settlement, or root-producing runtime was added.

That is intentional.

Future Phase 3 Round 2 work may harden validator lifecycle visibility or evidence handling elsewhere, but `svc-rewarder` should continue to treat validator/passport material as non-authoritative unless a later phase explicitly defines a safe planning-only artifact role.

## QuickChain Phase 3 Round 1 Notes — svc-storage

### Status

`svc-storage` is complete for QuickChain Phase 3 Round 1.

The crate now has an explicit Phase 3 validator/passport boundary test layer proving that `svc-storage` remains bytes-by-canonical-b3 infrastructure. It may store opaque validator/readiness artifact bytes by b3, but it does not interpret those bytes as validator membership, passport authority, registry truth, capability truth, paid-unlock truth, wallet truth, ledger truth, staking authority, slashing authority, settlement authority, or QuickChain runtime authority.

### Phase 3 Scope Completed

Added Phase 3 validator/passport boundary coverage for:

* opaque validator/readiness artifacts stored and retrieved by canonical b3 only
* b3 remaining byte-integrity truth only
* storage not interpreting validator/passport artifacts as authority
* usage event DTOs rejecting validator/passport authority poison fields
* accounting export DTOs rejecting validator/passport authority poison fields
* selected paid/policy/accounting/storage source paths avoiding Phase 3 authority fields
* source tree avoiding validator admission, revocation, rotation, staking, slashing, registry proof handling, cache unlock authority, wallet authority, and ledger authority
* manifest avoiding `svc-passport`, `svc-registry`, `ron-auth`, and `ron-ledger` authority dependencies

### Files Touched

* `crates/svc-storage/tests/quickchain_phase3_validator_boundary.rs`
* `crates/svc-storage/src/accounting/mod.rs`
* `crates/svc-storage/src/accounting/exporter.rs`
* `crates/svc-storage/docs/quickchain-preflight.md`

### Important Boundaries Preserved

`svc-storage` remains:

* content-addressed byte storage
* canonical b3 verifier
* range/media-serving infrastructure
* paid-storage admission participant
* accounting usage-event exporter
* non-authoritative artifact holder

`svc-storage` does not become:

* payment truth
* paid-access truth
* cache-only unlock authority
* wallet mutation authority
* direct ledger mutation authority
* balance truth
* receipt truth
* validator identity authority
* passport registry authority
* validator capability authority
* validator-set authority
* staking/bonding/slashing authority
* QuickChain root/checkpoint/finality producer
* bridge/external settlement authority

### Phase 3 Design Decision

For Phase 3 Round 1, storage may retain validator/passport-related artifact bytes only as opaque b3-addressed objects.

That means:

* b3 proves the bytes are the bytes
* b3 does not prove validator membership
* b3 does not prove passport admission
* b3 does not prove registry truth
* b3 does not prove payment
* b3 does not unlock paid content
* b3 does not create settlement finality

The patch also hardened storage accounting DTOs with unknown-field rejection so validator/passport authority cannot be smuggled into usage events or accounting export requests.

### Test Results

Focused Phase 3 tests passed:

* `quickchain_phase3_validator_boundary` — 5 passed

Full preflight passed:

* discovered focused QuickChain tests: 17
* all 17 focused QuickChain tests passed
* all-target tests passed
* paid-write tests passed
* wallet receipt verifier tests passed
* b3/range/media tests passed
* web3 paid storage loop passed
* clippy passed

Final gate:

`svc-storage quickchain exhaustive preflight gate passed: tests=17`

### Remaining Risks / Deferred Work

No passport, registry, auth, validator lifecycle, bond, staking, slashing, bridge, external settlement, root-producing runtime, or cache-only paid unlock behavior was added.

That is intentional.

Future work may route or display validator/readiness artifacts by b3, but storage must continue to treat them as opaque bytes. Interpretation, authorization, validator lifecycle, and payment truth must remain outside storage.


### END NOTE - JUNE 24 2026 - 12:55 CST



### BEGIN NOTE - JUNE 24 2026 - 20:10 CST

Here are the carry-forward crate notes for **`svc-rewarder + svc-storage`**. The terminal output confirms both lifecycle tests, both Round 1 validator tests, all-target tests, clippy, and exhaustive QuickChain gates are green: `svc-rewarder ... tests=15` and `svc-storage ... tests=18`.  This matches the Phase 3 Round 2 lifecycle-hardening scope: rotation/revocation/evidence/downtime/parameter updates, while still forbidding staking, slashing, liquidity, and public bridge behavior. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-rewarder + svc-storage

# QuickChain Phase 3 Round 2 Crate Notes — `svc-rewarder + svc-storage`

## 0. Status

Crate pair:

```text
svc-rewarder + svc-storage
```

Phase / round:

```text
QuickChain Phase 3 — passport-gated validator set
Round 2 — validator lifecycle / evidence / operation hardening
```

Status:

```text
COMPLETE / PARKED for QuickChain Phase 3
```

Because Phase 3 has only two rounds, and this pair now has both Round 1 validator/passport boundary coverage and Round 2 validator lifecycle coverage, the safe status label is:

```text
QuickChain Phase 3 — svc-rewarder + svc-storage: 100% COMPLETE / PARKED
```

Important precision:

```text
svc-rewarder + svc-storage are complete for QuickChain Phase 3.
They are not Phase 4 staking/slashing/bonding complete.
They should not be patched again for Phase 3 unless a regression appears.
```

Current Phase 3 progress:

```text
1. ron-proto + ron-ledger        ✅ Phase 3 complete / parked
2. svc-wallet + ron-accounting   ✅ Phase 3 complete / parked
3. svc-rewarder + svc-storage    ✅ Phase 3 complete / parked
4. svc-gateway + omnigate        ← next active pair
5. svc-index + ron-policy        pending
6. CrabLink Tauri + adapters     pending
```

---

## 1. Why this pair mattered

`svc-rewarder` is payout planning infrastructure.

`svc-storage` is bytes/artifacts/paid-storage infrastructure.

Phase 3 Round 2 introduced validator lifecycle hardening upstream, including:

```text
validator rotation
validator revocation
equivocation evidence
double-attestation evidence
split-brain evidence
replay challenge evidence
invalid-attestation evidence
downtime/degraded status
governance-gated parameter updates
```

This pair’s job was to prove that lifecycle/evidence metadata can pass through the downstream system only as safe data or opaque bytes, and cannot become:

```text
payout authority
wallet mutation authority
ledger mutation authority
paid unlock authority
validator runtime authority
staking authority
slashing authority
bridge authority
external settlement authority
```

The patch intentionally did not add runtime validator logic.

No staking, slashing, bonding, bridge, public settlement, external settlement, liquidity, ROX, Solana, or public-chain behavior was added.

---

## 2. Files added or changed

New `svc-rewarder` test:

```text
crates/svc-rewarder/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

New `svc-storage` test:

```text
crates/svc-storage/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

Follow-up test-only fix:

```text
crates/svc-rewarder/tests/quickchain_phase3_validator_lifecycle_boundary.rs
crates/svc-storage/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

The follow-up fix did two things:

```text
removed an unused RewardFundingSource import from the svc-rewarder test
fixed svc-storage test-only lifetime handling for DTOs with &'static str fields
```

No production runtime files were changed.

No `Cargo.toml` dependencies were changed.

No Python helper scripts were added.

No validator runtime logic was added.

No wallet, ledger, bridge, staking, slashing, or settlement authority was added.

---

## 3. `svc-rewarder` Phase 3 Round 2 coverage

The new `svc-rewarder` lifecycle-boundary test proves:

```text
ComputeEpochRequest rejects validator lifecycle authority fields
AccountingSnapshot rejects validator lifecycle authority fields
AccountingSnapshot contribution rows reject validator lifecycle authority fields
RewardPolicy rejects validator lifecycle authority fields
RewardManifest rejects validator lifecycle authority fields
WalletIssueRequest handoff rejects validator lifecycle authority fields
Rewarder outputs remain planning artifacts only
Rewarder wallet preview remains wallet issue-request shape only
Rewarder source does not construct validator lifecycle runtime authority
Rewarder source does not construct staking/slashing/bridge/external-settlement authority
```

The lifecycle poison-field matrix included:

```text
validator_rotation
validator_rotation_epoch
validator_rotation_decision
validator_revocation
validator_revocation_reason
validator_revocation_decision
validator_lifecycle_decision
validator_lifecycle_status
validator_lifecycle_rejection_code
equivocation_evidence
double_attestation_evidence
split_brain_evidence
replay_challenge_evidence
invalid_attestation_evidence
validator_downtime_status
validator_degraded_status
downtime_report
governance_parameter_update
validator_set_parameter_update
quorum_parameter_update
checkpoint_parameter_update
slash_evidence
slashing
staking_power
validator_bond
bonded_economics
validator_reward
bridge_settlement
external_settlement
```

The output-boundary checks also confirmed rewarder artifacts do not expose:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payout_executed
paid_unlock
settlement_finality
finalized
anchored
```

`svc-rewarder` remains:

```text
deterministic payout planning only
```

It does not become:

```text
wallet
ledger
validator runtime
settlement authority
staking/slashing authority
bridge authority
```

---

## 4. `svc-rewarder` final test status

Focused Round 2 lifecycle test passed:

```bash
cargo test -p svc-rewarder --test quickchain_phase3_validator_lifecycle_boundary
```

Result:

```text
4 passed; 0 failed
```

Covered tests:

```text
rewarder_compute_request_rejects_validator_lifecycle_authority_fields
rewarder_input_dtos_reject_validator_lifecycle_authority_fields
rewarder_outputs_remain_planning_artifacts_not_lifecycle_or_payout_execution_authority
rewarder_source_does_not_construct_validator_lifecycle_runtime_or_economic_authority
```

Existing Phase 3 Round 1 validator-boundary test passed:

```bash
cargo test -p svc-rewarder --test quickchain_phase3_validator_boundary
```

Result:

```text
7 passed; 0 failed
```

Covered tests:

```text
rewarder_wallet_handoff_preview_has_no_phase3_validator_authority_keys
rewarder_manifest_keeps_passport_registry_and_auth_crates_out_of_planning_path
reward_manifest_remains_planning_artifact_not_validator_membership_or_passport_authority
rewarder_nested_snapshot_and_policy_reject_phase3_validator_authority_fields
rewarder_compute_request_rejects_phase3_validator_passport_authority_fields
reward_manifest_rejects_phase3_validator_passport_authority_fields
rewarder_source_does_not_implement_phase3_validator_or_passport_authority
```

Final exhaustive gate:

```bash
bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=15 ==
```

Additional green gates:

```text
all-targets test passed
clippy -D warnings passed
no checked-in Python helpers under svc-rewarder
format check passed
dynamic QuickChain test discovery found 15 tests
forbidden-scope marker preserved
```

Therefore:

```text
svc-rewarder is complete / parked for QuickChain Phase 3.
```

---

## 5. `svc-storage` Phase 3 Round 2 coverage

The new `svc-storage` lifecycle-boundary test proves:

```text
AccountingExportRequest rejects validator lifecycle authority fields
UsageEventDto rejects nested validator lifecycle authority fields
storage usage events remain metering only
storage accounting export remains derivative metering only
lifecycle evidence bytes can be stored by canonical b3
lifecycle evidence bytes can be read by exact b3
lifecycle evidence bytes can be range-read safely
stored lifecycle evidence does not unlock paid content
stored lifecycle evidence does not become validator authority
storage source does not construct lifecycle/cache/economic authority
```

The lifecycle poison-field matrix included:

```text
validator_rotation
validator_rotation_epoch
validator_rotation_decision
validator_revocation
validator_revocation_reason
validator_revocation_decision
validator_lifecycle_decision
validator_lifecycle_status
validator_lifecycle_rejection_code
equivocation_evidence
double_attestation_evidence
split_brain_evidence
replay_challenge_evidence
invalid_attestation_evidence
validator_downtime_status
validator_degraded_status
downtime_report
governance_parameter_update
validator_set_parameter_update
quorum_parameter_update
checkpoint_parameter_update
slash_evidence
slashing
staking_power
validator_bond
bonded_economics
validator_reward
bridge_settlement
external_settlement
```

The usage/accounting export checks also confirmed storage usage events do not expose:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payout_executed
paid_unlock
settlement_finality
finalized
anchored
cache_only_unlock
```

`svc-storage` can store validator lifecycle evidence only as:

```text
opaque bytes
canonical b3-addressed artifacts
retrievable byte objects
range-readable byte objects
```

It cannot interpret those artifacts as:

```text
paid unlock authority
wallet truth
ledger truth
validator truth
settlement truth
staking/slashing truth
bridge truth
```

---

## 6. `svc-storage` final test status

Focused Round 2 lifecycle test passed:

```bash
cargo test -p svc-storage --test quickchain_phase3_validator_lifecycle_boundary
```

Result:

```text
4 passed; 0 failed
```

Covered tests:

```text
storage_accounting_export_rejects_validator_lifecycle_authority_fields
storage_usage_events_remain_metering_not_lifecycle_or_paid_unlock_authority
lifecycle_evidence_bytes_store_by_b3_without_unlock_or_validator_authority
storage_source_does_not_construct_validator_lifecycle_or_cache_unlock_authority
```

Existing Phase 3 Round 1 validator-boundary test passed:

```bash
cargo test -p svc-storage --test quickchain_phase3_validator_boundary
```

Result:

```text
5 passed; 0 failed
```

Covered tests:

```text
storage_accounting_usage_dtos_reject_phase3_validator_authority_fields
storage_manifest_keeps_passport_registry_auth_wallet_and_ledger_authority_out_of_runtime_deps
storage_can_retain_phase3_validator_readiness_artifacts_as_opaque_b3_bytes_only
storage_paid_policy_and_accounting_sources_have_no_phase3_authority_keys
storage_source_does_not_implement_phase3_validator_or_passport_authority
```

Final exhaustive gate:

```bash
bash crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=18 ==
```

Additional green gates:

```text
all-targets test passed
clippy -D warnings passed
no checked-in Python helpers under svc-storage
format check passed
dynamic QuickChain test discovery found 18 tests
forbidden-scope marker preserved
```

Therefore:

```text
svc-storage is complete / parked for QuickChain Phase 3.
```

---

## 7. Test-only failure and fix

Initial `svc-storage` failure:

```text
error[E0597]: `top_level` does not live long enough
error[E0597]: `nested` does not live long enough
```

Cause:

```text
AccountingExportRequest and UsageEventDto contain &'static str fields.
The test dynamically built JSON Strings and tried to deserialize them into DTOs requiring static string fields.
```

Resolution:

```text
Converted those dynamic test JSON strings into leaked static test fixtures using Box::leak(...into_boxed_str()).
```

This was test-only.

It did not weaken production behavior.

It did not change runtime code.

It did not change DTO shape.

It preserved the intended lifecycle poison-field rejection coverage.

`svc-rewarder` also had a clippy-blocking unused import:

```text
RewardFundingSource
```

Resolution:

```text
Removed unused RewardFundingSource import from the new lifecycle test.
```

---

## 8. What remains forbidden after this pair

This pair did not add and must not be treated as adding:

```text
staking
slashing
bonding
validator rewards as live economics
validator payout execution
public validator economy
public chain runtime
public bridge
ROX
Solana
external settlement
liquidity
exchange-facing logic
rewarder ledger mutation
storage ledger mutation
storage paid-unlock truth
storage cache paid-unlock truth
storage validator authority
raw engagement direct ROC allocation
gateway/omnigate wallet mutation
accounting as balance truth
fake balances
fake receipts
silent spend
placeholder roots
fake finality
```

`svc-rewarder` remains:

```text
deterministic payout planning only
```

`svc-storage` remains:

```text
bytes/artifacts by canonical b3
paid storage admission/enforcement only
usage metering/export only
not payment truth
not validator truth
```

---

## 9. Boundary state after completion

The architecture remains intact:

```text
ron-proto = DTOs / strict shapes
ron-ledger = economic truth
svc-wallet = mutation front-door
ron-accounting = snapshots / metering / read-model artifacts
svc-rewarder = payout planning only
svc-storage = bytes/artifacts/paid-write enforcement only
svc-gateway = public boundary / enforcement
omnigate = hydration / product coordinator
svc-index = lookup / pointer service
ron-policy = declarative policy
CrabLink Tauri = display / user intent only
```

Validator lifecycle evidence now remains safely contained downstream as:

```text
strict rejected poison fields in reward planning DTOs
strict rejected poison fields in storage accounting export DTOs
opaque b3-addressed bytes in storage
non-authoritative metering labels
non-authoritative artifacts
```

It cannot become:

```text
wallet spend authority
wallet receipt authority
balance truth
accounting truth
rewarder payout execution
storage paid unlock authority
validator membership authority
validator lifecycle authority
staking authority
slashing authority
bridge authority
external settlement authority
```

---

## 10. Retest commands for this completed pair

Only rerun if these crates are modified again.

Focused retests:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-rewarder --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p svc-rewarder --test quickchain_phase3_validator_boundary

cargo test -p svc-storage --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p svc-storage --test quickchain_phase3_validator_boundary
```

Crate-local exhaustive gates:

```bash
bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
bash crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Expected final markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=15 ==
== svc-storage quickchain exhaustive preflight gate passed: tests=18 ==
```

---

## 11. Completion judgment

Final judgment:

```text
QuickChain Phase 3 — svc-rewarder + svc-storage: 100% COMPLETE / PARKED
```

This pair now has:

```text
Phase 3 Round 1 validator/passport boundary coverage
Phase 3 Round 2 validator lifecycle hardening coverage
full crate-local QuickChain preflight green
all-target tests green
clippy -D warnings green
no Python helper drift
no runtime authority creep
no wallet/ledger mutation creep
no staking/slashing/bridge/external settlement creep
```

Next active pair:

```text
svc-gateway + omnigate
```

Recommended next codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-rewarder + svc-storage


### END NOTE - JUNE 24 2026 - 20:10 CST




### BEGIN NOTE - JUNE 25 2026 - 19:05 CST

Yes — this round is green for **`svc-rewarder + svc-storage`**.

The terminal confirms the storage lifetime fix passed, `svc-storage` Phase 4 artifact boundary passed 4/4, `svc-storage` tooling passed 4/4, `svc-rewarder` discovered and passed 16 focused QuickChain tests, and `svc-storage` discovered and passed 19 focused QuickChain tests. Final gate markers show:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=16 ==
== svc-storage quickchain exhaustive preflight gate passed: tests=19 ==
```

The terminal also preserved the forbidden-scope markers: no roots, checkpoints, validators, settlement, anchors, bridges, staking, or liquidity; `svc-rewarder` remains deterministic payout planning only, and `svc-storage` remains bytes by canonical b3 only with cache not acting as paid-access authority. 

# QuickChain Phase 4 Round 1 Crate Notes — `svc-rewarder + svc-storage`

## 0. Status

Crate pair:

```text
svc-rewarder + svc-storage
```

Phase / round:

```text
QuickChain Phase 4 — bonded safety model
Round 1 — bond boundary hardening / no live slashing / no public staking / no liquidity
```

Status:

```text
COMPLETE / PARKED for Phase 4 Round 1
```

This does **not** mean all of Phase 4 is complete. It means the third crate pair in the normal Phase 4 Round 1 sweep is complete and should not be reopened unless a later pair exposes a real boundary regression.

Current Phase 4 Round 1 progress:

```text
ron-proto + ron-ledger              ✅ complete / parked
svc-wallet + ron-accounting         ✅ complete / parked
svc-rewarder + svc-storage          ✅ complete / parked
svc-gateway + omnigate              ⏭ next
svc-index + ron-policy              pending
CrabLink Tauri + client adapters    pending
```

---

## 1. What This Pair Proves

This pass proves that **bond-shaped Phase 4 data can reach planning/storage-adjacent seams without becoming authority**.

`svc-rewarder` remains:

```text
deterministic payout planning only
wallet handoff DTO producer only
not bond truth
not slash truth
not validator reward authority
not public staking authority
not liquidity authority
not wallet mutation truth
not ledger mutation truth
not bridge or external settlement authority
```

`svc-storage` remains:

```text
bytes/artifacts by canonical b3 only
paid admission and metering support only
not bond truth
not slash truth
not wallet truth
not ledger truth
not validator truth
not paid-unlock authority
not cache-only unlock authority
not bridge or external settlement authority
```

The important doctrine is intact:

```text
svc-wallet remains the mutation front-door.
ron-ledger remains economic truth.
ron-accounting remains derivative reporting/snapshot material.
svc-rewarder plans but never mutates ledger truth.
svc-storage stores bytes but never authorizes economic lifecycle decisions.
```

---

## 2. Files Added / Updated

### `svc-rewarder`

Added:

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_planning_boundary.rs
```

Updated:

```text
crates/svc-rewarder/docs/quickchain-preflight.md
```

The new test suite verifies:

```text
rewarder manifests and wallet handoffs remain planning artifacts
ComputeEpochRequest rejects Phase 4 bond/slash/staking/liquidity authority fields
AccountingSnapshot rejects those fields
RewardPolicy rejects those fields
RewardManifest rejects those fields
SettlementBatch rejects those fields
WalletIssueRequest rejects those fields
rewarder replay/dedupe does not become second payout or slash authority
rewarder source does not implement bond, slash, staking, liquidity, bridge, external settlement, or ron-ledger authority
```

Important fix applied during this round:

```text
The rewarder test fixture originally used broad substring checks against "bond".
That falsely failed because the test data itself included "bond" in epoch/account/salt names.
The fixture was renamed to neutral "reward-planning" vocabulary, and exact authority-key checks were kept.
```

Final focused result:

```text
quickchain_phase4_bond_planning_boundary: 5 passed
quickchain_tooling_boundary: 4 passed
full svc-rewarder preflight: tests=16 passed
all-target tests: passed
clippy: passed
```

---

## 3. `svc-rewarder` Boundary Now Locked

The crate now explicitly rejects or scans against Phase 4 authority creep such as:

```text
QuickChainValidatorBondAccount
QuickChainSlashEvidence
bond_lifecycle_decision
bond_account_status
apply_bond
commit_bond
execute_bond
apply_slash
commit_slash
execute_slash
slash_bond
auto_slash_now
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
solana
rox
ron_ledger::
```

Allowed posture:

```text
rewarder may consume accounting/policy inputs for deterministic payout planning
rewarder may produce wallet issue handoff DTOs
rewarder may dedupe replay by run key
rewarder may preview or emit through svc-wallet routes where already designed
```

Forbidden posture remains:

```text
rewarder must not mutate wallet directly
rewarder must not mutate ledger directly
rewarder must not produce roots/checkpoints/finality
rewarder must not slash bonds
rewarder must not create validator rewards
rewarder must not create public staking markets
rewarder must not touch liquidity, bridge, ROX, Solana, or external settlement
```

---

## 4. `svc-storage`

Added:

```text
crates/svc-storage/tests/quickchain_phase4_bond_artifact_boundary.rs
```

Updated:

```text
crates/svc-storage/docs/quickchain-preflight.md
```

The new test suite verifies:

```text
AccountingExportRequest rejects Phase 4 bond/slash/staking/liquidity authority fields
UsageEventDto rejects nested authority fields
usage events remain metering only
storage can retain opaque bond/evidence/report-like artifact bytes by canonical b3
b3 proves bytes only, not lifecycle authority
storage source does not implement Phase 4 bond/slash/staking/liquidity/bridge/external settlement runtime authority
```

Important fix applied during this round:

```text
AccountingExportRequest and UsageEventDto contain &'static str fields.
The generated rejection JSON strings had to be converted into leaked &'static str test fixtures with Box::leak(...into_boxed_str()).
That fixed the E0597 lifetime failure without changing production code.
```

Final focused result:

```text
quickchain_phase4_bond_artifact_boundary: 4 passed
quickchain_tooling_boundary: 4 passed
full svc-storage preflight: tests=19 passed
all-target tests: passed
clippy: passed
```

---

## 5. `svc-storage` Boundary Now Locked

The crate now explicitly rejects or scans against Phase 4 authority creep such as:

```text
QuickChainValidatorBondAccount
QuickChainSlashEvidence
bond_lifecycle_decision
bond_account_status
apply_bond
commit_bond
execute_bond
apply_slash
commit_slash
execute_slash
slash_bond
auto_slash_now
validator_reward_receipt
public_staking_market
liquidity_pool
paid_unlock_from_bond
unlock_from_bond_artifact
cache_only_unlock
bridge_settlement
external_settlement
solana
rox
ron_ledger::
```

Allowed posture:

```text
storage may store opaque bytes
storage may serve canonical b3 content
storage may support paid admission paths
storage may emit metering/accounting export DTOs
storage may hold future proof/evidence/report artifacts as bytes
```

Forbidden posture remains:

```text
storage must not treat b3 as payment proof
storage must not treat cache as entitlement
storage must not unlock paid content from artifact bytes alone
storage must not generate wallet receipts
storage must not become bond truth or slash truth
storage must not mutate wallet or ledger truth
storage must not produce roots/finality/checkpoints
storage must not introduce public staking, liquidity, bridge, Solana, ROX, or external settlement
```

---

## 6. Commands That Passed

Focused repair / validation:

```text
cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary
cargo test -p svc-rewarder --test quickchain_tooling_boundary
cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary
cargo test -p svc-storage --test quickchain_tooling_boundary
```

Full crate-local preflights:

```text
bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
bash crates/svc-storage/scripts/dev-quickchain-preflight.sh
```

Final gate markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=16 ==
== svc-storage quickchain exhaustive preflight gate passed: tests=19 ==
```

---

## 7. Parked Status

Final status for this pair:

```text
QuickChain Phase 4 Round 1 — svc-rewarder + svc-storage
Status: COMPLETE / PARKED
```

Do not reopen these crates for Phase 4 Round 1 unless:

```text
a later crate-pair exposes a real integration mismatch
a scanner catches new authority vocabulary drift
svc-wallet / ron-ledger bond DTOs change
gateway / omnigate paid enforcement contracts require stricter storage/rewarder boundary checks
the buildplan explicitly opens a later Phase 4 round for this pair
```

---

## 8. Next Crate Pair

Next normal crate pair:

```text
svc-gateway + omnigate
```

Expected Phase 4 Round 1 mission for the next pair:

```text
svc-gateway remains public/API boundary only.
omnigate remains hydration/enforcement coordinator only.
Neither becomes bond truth, slash truth, wallet truth, ledger truth, public staking authority, liquidity authority, bridge authority, or external settlement authority.
Paid access must still follow backend wallet/ledger receipt paths.
No cache-only unlock.
No silent spend.
No fake receipts.
No ROX/Solana/public bridge creep.
```


### END NOTE - JUNE 25 2026 - 19:05 CST




### BEGIN NOTE - JUNE 26 2026 - 01:35 CST

Here are the crate notes for the completed `svc-rewarder + svc-storage` slice. Phase 4 Round 2 is specifically the slashing/challenge **simulation** round, with challenge windows, appeal/freeze states, replayable dispute state, and **no live irreversible slash yet**. 

# QuickChain Phase 4 Round 2 Crate Notes — `svc-rewarder` + `svc-storage`

Date: June 26, 2026
Project: RustyOnions / CrabLink
Phase / round: QuickChain Phase 4 Round 2
Crate pair: `svc-rewarder + svc-storage`
Status: COMPLETE / PARKED / GREEN

## 0. Final status

`svc-rewarder + svc-storage` are complete and parked for QuickChain Phase 4 Round 2.

Final status:

```text
QuickChain Phase 4 Round 2 — svc-rewarder + svc-storage
Status: COMPLETE / PARKED / GREEN
```

Final proof markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=17 ==
== svc-rewarder QuickChain parking gate passed ==

== svc-storage quickchain exhaustive preflight gate passed: tests=20 ==
== svc-storage QuickChain parking gate passed ==
```

The final `svc-rewarder` run shows the clippy fix passed, all 17 focused QuickChain tests ran, all-target tests passed, clippy passed, and the final parking gate passed.  The prior run shows `svc-storage` discovered 20 focused QuickChain tests, passed all-target tests, passed clippy, and reached its final parking gate. 

Current Phase 4 Round 2 crate-pair position:

```text
1. ron-proto + ron-ledger              COMPLETE / PARKED
2. svc-wallet + ron-accounting         COMPLETE / PARKED
3. svc-rewarder + svc-storage          COMPLETE / PARKED
4. svc-gateway + omnigate              NEXT
5. svc-index + ron-policy              pending
6. CrabLink Tauri + client adapters    pending
```

## 1. Phase 4 Round 2 context

Phase 4 Round 2 is the **slashing/challenge simulation** round.

Allowed scope:

```text
challenge windows
slash evidence validation
appeal/freeze states
hold-like lifecycle for disputed bonds
replayable dispute state
safe dispute simulation
no one-step irreversible slash
```

Exit gate:

```text
slashing simulation is deterministic
slashing simulation is replayable
challenge path is safe
dispute windows are explicit
no live irreversible slash yet
```

For this crate pair, the correct downstream shape was:

```text
svc-rewarder:
  reject dispute/slash/challenge simulation as reward or payout authority

svc-storage:
  store dispute/evidence bytes only as opaque b3-addressed artifacts
```

This pair did not implement live slashing, bond enforcement, validator rewards, staking, liquidity, bridges, ROX/Solana, external settlement, payout execution, or storage unlock authority.

## 2. Doctrine preserved

The following boundaries remain intact:

```text
svc-rewarder plans payouts but never mutates ledger
svc-storage stores bytes/artifacts but is not payment truth
svc-wallet remains mutation front-door
ron-ledger remains economic truth
ron-accounting remains derivative reporting/snapshots, not balance truth
storage cache is convenience, not paid-access authority
b3 proves bytes, not economic truth
dispute evidence artifacts do not become slash truth
reward plans do not become receipts
wallet handoff DTOs do not become rewarder-created receipts
```

Forbidden scope still absent:

```text
no live irreversible slash
no automatic slash
no slash reward
no penalty reward
no validator reward receipt
no bond dispute payout
no wallet mutation from rewarder
no ledger mutation from rewarder
no paid unlock from storage evidence
no cache-only unlock
no storage-created entitlement truth
no staking market
no liquidity pool
no bridge settlement
no external settlement
no ROX/Solana active runtime
```

## 3. `svc-rewarder` changes

### Files added / updated

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_dispute_reward_boundary.rs
crates/svc-rewarder/docs/quickchain-preflight.md
```

Script executable bits were also ensured for:

```text
crates/svc-rewarder/scripts/dev-quickchain-preflight.sh
crates/svc-rewarder/scripts/dev-quickchain-park.sh
```

The uploaded `svc-rewarder` bundle confirms the crate already had QuickChain-focused tests and dynamic park/preflight scripts before this patch. 

### New test file

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_dispute_reward_boundary.rs
```

Purpose:

```text
Prove disputed-bond simulation cannot become rewarder payout, slash, penalty, wallet, ledger, staking, liquidity, bridge, or external settlement authority.
```

The new test suite contains 4 tests:

```text
dispute_simulation_fields_reject_at_rewarder_input_boundaries
dispute_simulation_fields_reject_at_rewarder_output_and_wallet_handoff_boundaries
rewarder_does_not_emit_dispute_reward_when_scores_are_zero_or_dry_run
rewarder_source_does_not_construct_phase4_round2_dispute_runtime_authority
```

Focused result:

```text
cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary

Result:
  4 passed
  0 failed
```

The terminal output confirms all four tests passed. 

### Boundary keys rejected by `svc-rewarder`

The new tests reject Phase 4 Round 2 dispute/slash authority fields at these rewarder boundaries:

```text
ComputeEpochRequest
AccountingSnapshot
RewardPolicy
RewardManifest
SettlementBatch
WalletIssueRequest
```

Rejected authority field families include:

```text
dispute_id
dispute_status
challenge_window
challenge_window_open
appeal_window
appeal_window_open
freeze_pending_appeal
frozen_minor
disputed_minor
slash_evidence
slash_decision
slash_recommendation
slash_reward
slash_reward_recipient
slash_penalty_minor
automatic_slash
auto_slash_now
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
bond_penalty
bond_dispute_payout
dispute_reward
reward_from_dispute
penalty_reward
validator_reward
validator_reward_receipt
wallet_receipt
ledger_receipt
wallet_mutation
ledger_mutation
payout_execution
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
solana
rox
```

Important meaning:

```text
Dispute simulation cannot enter rewarder as a payout basis.
Dispute simulation cannot enter rewarder outputs as a reward plan.
Dispute simulation cannot enter wallet handoff DTOs as wallet authority.
```

### Source scanner added for `svc-rewarder`

The new source scanner verifies `svc-rewarder/src` does not construct runtime authority through strings like:

```text
reward_from_dispute
dispute_reward
slash_reward
penalty_reward
bond_dispute_payout
payout_from_dispute
payout_from_slash
slash_penalty_minor
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
wallet_slash
ledger_slash
auto_slash_now
freeze_bond_payout
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
```

This preserves:

```text
svc-rewarder = deterministic payout planning only
not dispute authority
not slash authority
not wallet authority
not ledger authority
not validator reward authority
not public staking authority
not bridge/external settlement authority
```

## 4. `svc-rewarder` docs updated

The `svc-rewarder` QuickChain runbook was appended with a Phase 4 Round 2 section.

New documented boundary:

```text
challenge/freeze/appeal/slash simulation must not create reward payouts
disputed-bond state must not become validator reward authority
slash evidence must not become payout authority
rejected slash simulation must not become protocol reward input
rewarder must not mutate wallet or ledger state
rewarder must not produce bond/slash/penalty receipts
rewarder must not create staking, liquidity, bridge, ROX, Solana, or external settlement behavior
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

This keeps docs aligned with the new test boundary.

## 5. `svc-rewarder` issue encountered and fixed

### Clippy failure

After focused tests passed, `svc-rewarder` park stopped during clippy with three lints:

```text
unnecessary use of to_owned
unnecessary use of to_owned
assert_eq! with literal bool
```

The failing lines were in:

```text
crates/svc-rewarder/tests/quickchain_phase4_bond_dispute_reward_boundary.rs
```

Fix applied:

```text
ContentCid::parse(INPUTS_CID.to_owned())
->
ContentCid::parse(INPUTS_CID)

assert_eq!(manifest.ledger.emitted, false)
->
assert!(!manifest.ledger.emitted)
```

The fix passed:

```text
cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

The terminal confirms the focused test passed and clippy finished cleanly. 

## 6. `svc-rewarder` final gate

Final `svc-rewarder` preflight discovered 17 QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_phase3_validator_lifecycle_boundary
quickchain_phase4_bond_dispute_reward_boundary
quickchain_phase4_bond_planning_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_funding_source
quickchain_preflight_no_direct_mutation
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_raw_engagement
quickchain_preflight_replay_no_double_issue
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Final marker:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=17 ==
== svc-rewarder QuickChain parking gate passed ==
```

The final output confirms this exact result. 

Important count change:

```text
Phase 4 Round 1 svc-rewarder count: 16
Phase 4 Round 2 svc-rewarder count: 17
```

The new test target is:

```text
quickchain_phase4_bond_dispute_reward_boundary
```

## 7. What `svc-rewarder` now proves

`svc-rewarder` now proves:

```text
dispute simulation cannot become rewarder payout basis
challenge/freeze/appeal/slash fields reject at request input boundary
challenge/freeze/appeal/slash fields reject at accounting snapshot boundary
challenge/freeze/appeal/slash fields reject at reward policy boundary
challenge/freeze/appeal/slash fields reject at reward manifest boundary
challenge/freeze/appeal/slash fields reject at settlement batch boundary
challenge/freeze/appeal/slash fields reject at wallet issue handoff boundary
zero-score and dry-run dispute-adjacent planning emits no dispute reward
rewarder source does not construct slash/reward/penalty/bond-dispute payout authority
rewarder remains deterministic payout planning only
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

## 8. What `svc-rewarder` still does not implement

`svc-rewarder` does not implement:

```text
slash rewards
penalty rewards
validator rewards from dispute state
bond dispute payouts
live slashing
automatic slashing
wallet freezing
wallet mutation
ledger mutation
bond capture
bond forfeiture
payout execution from dispute state
public staking market
liquidity pool
bridge settlement
external settlement
ROX/Solana runtime
```

## 9. `svc-storage` changes

### Files added / updated

```text
crates/svc-storage/tests/quickchain_phase4_dispute_evidence_artifact_boundary.rs
crates/svc-storage/docs/quickchain-preflight.md
```

Script executable bits were also ensured for:

```text
crates/svc-storage/scripts/dev-quickchain-preflight.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

The uploaded `svc-storage` bundle confirms the crate already had dynamic QuickChain park/preflight scripts and prior Phase 4 bond artifact tests. 

### New test file

```text
crates/svc-storage/tests/quickchain_phase4_dispute_evidence_artifact_boundary.rs
```

Purpose:

```text
Prove dispute/challenge/evidence bytes are opaque b3-addressed artifacts only, never unlock, slash, wallet, ledger, bond, staking, liquidity, bridge, or external settlement authority.
```

The new test suite contains 4 tests:

```text
storage_accounting_export_rejects_dispute_authority_fields
storage_usage_event_remains_metering_not_dispute_or_unlock_authority
dispute_evidence_bytes_store_by_b3_without_unlock_or_slash_authority
storage_source_does_not_construct_phase4_round2_dispute_runtime_or_unlock_authority
```

Focused result:

```text
cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary

Result:
  4 passed
  0 failed
```

The terminal output confirms all four tests passed. 

### Boundary keys rejected by `svc-storage`

The new tests reject Phase 4 Round 2 dispute/unlock/slash authority fields at storage metering/export boundaries:

```text
AccountingExportRequest
UsageEventDto
nested UsageEventDto inside AccountingExportRequest
```

Rejected authority field families include:

```text
dispute_id
dispute_status
challenge_window
challenge_window_open
appeal_window
appeal_window_open
freeze_pending_appeal
frozen_minor
disputed_minor
slash_evidence
slash_decision
slash_recommendation
slash_capture
automatic_slash
auto_slash_now
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
bond_penalty
wallet_receipt
ledger_receipt
wallet_mutation
ledger_mutation
paid_unlock
paid_unlock_from_dispute
unlock_from_evidence
cache_only_unlock
cache_unlock_authority
validator_reward
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
solana
rox
```

Important meaning:

```text
Storage usage events stay metering-only.
Accounting exports stay metering-only.
Evidence metadata cannot smuggle paid unlock or slash authority.
```

### Opaque b3 artifact proof

The new storage test stores this kind of body as bytes:

```json
{"schema":"quickchain.dispute-evidence.bytes-only.test","status":"opaque"}
```

It verifies:

```text
bytes are stored under canonical b3
full read returns same bytes
range read returns bounded bytes
head returns length and content-derived etag
etag remains content-hash-derived
no unlock/slash authority is inferred
```

Meaning:

```text
b3 proves the bytes stored.
b3 does not prove slash truth.
b3 does not prove bond truth.
b3 does not unlock paid content.
```

### Source scanner added for `svc-storage`

The new source scanner verifies `svc-storage/src` does not construct runtime authority through strings like:

```text
dispute_unlock
paid_unlock_from_dispute
unlock_from_dispute
unlock_from_evidence
evidence_paid_unlock
cache_dispute_authority
cache_unlock_authority_from_dispute
slash_evidence_truth
bond_dispute_truth
execute_slash
commit_slash_decision
capture_disputed_bond
bond_forfeiture
wallet_slash
ledger_slash
auto_slash_now
validator_reward_receipt
public_staking_market
liquidity_pool
bridge_settlement
external_settlement
```

This preserves:

```text
svc-storage = bytes/artifacts by b3 only
not dispute authority
not slash authority
not bond truth
not paid unlock authority
not cache entitlement authority
not wallet authority
not ledger authority
not public staking authority
not bridge/external settlement authority
```

## 10. `svc-storage` docs updated

The `svc-storage` QuickChain runbook was appended with a Phase 4 Round 2 section.

New documented boundary:

```text
b3 proves bytes only
stored evidence bytes do not become slash truth
stored evidence bytes do not become bond truth
stored evidence bytes do not unlock paid content
cache does not become entitlement authority
accounting export remains metering only
storage must not mutate wallet or ledger state
storage must not create staking, liquidity, bridge, ROX, Solana, or external settlement behavior
svc-wallet remains mutation front-door
ron-ledger remains economic truth
```

This keeps docs aligned with the new test boundary.

## 11. `svc-storage` final gate

Final `svc-storage` preflight discovered 20 QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_phase3_validator_lifecycle_boundary
quickchain_phase4_bond_artifact_boundary
quickchain_phase4_dispute_evidence_artifact_boundary
quickchain_preflight_b3_integrity
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_economics_quote
quickchain_preflight_no_direct_mutation
quickchain_preflight_observability
quickchain_preflight_paid_cache
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_range_media
quickchain_preflight_settlement_boundary
quickchain_preflight_source_authority_scan
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Final marker:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=20 ==
== svc-storage QuickChain parking gate passed ==
```

The terminal output confirms this exact result. 

Important count change:

```text
Phase 4 Round 1 svc-storage count: 19
Phase 4 Round 2 svc-storage count: 20
```

The new test target is:

```text
quickchain_phase4_dispute_evidence_artifact_boundary
```

## 12. What `svc-storage` now proves

`svc-storage` now proves:

```text
dispute evidence artifacts are opaque bytes only
dispute evidence artifacts store and read by canonical b3
b3 does not become slash truth
b3 does not become bond truth
b3 does not become unlock authority
cache does not become entitlement authority
accounting export rejects dispute/slash/unlock authority fields
usage event DTO rejects dispute/slash/unlock authority fields
storage source does not construct dispute runtime authority
storage source does not construct slash authority
storage source does not construct paid unlock authority from evidence
storage source does not construct staking, liquidity, bridge, or external settlement authority
```

## 13. What `svc-storage` still does not implement

`svc-storage` does not implement:

```text
slash evidence truth
bond dispute truth
paid unlock from evidence
cache-only unlock
wallet mutation
ledger mutation
automatic slashing
bond capture
bond forfeiture
validator rewards
staking market
liquidity pool
bridge settlement
external settlement
ROX/Solana runtime
```

## 14. Focused tests run

Focused tests that passed for this pair:

```bash
cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary
cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary
cargo test -p svc-rewarder --test quickchain_tooling_boundary

cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary
cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary
cargo test -p svc-storage --test quickchain_tooling_boundary
```

Focused results:

```text
svc-rewarder quickchain_phase4_bond_dispute_reward_boundary:
  4 passed
  0 failed

svc-rewarder quickchain_phase4_bond_planning_boundary:
  5 passed
  0 failed

svc-rewarder quickchain_tooling_boundary:
  4 passed
  0 failed

svc-storage quickchain_phase4_dispute_evidence_artifact_boundary:
  4 passed
  0 failed

svc-storage quickchain_phase4_bond_artifact_boundary:
  4 passed
  0 failed

svc-storage quickchain_tooling_boundary:
  4 passed
  0 failed
```

The terminal output confirms the focused tests passed before park gates were run. 

## 15. Final park commands used

```bash
cd /Users/mymac/Desktop/RustyOnions

crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

Final result:

```text
svc-rewarder: COMPLETE / PARKED / GREEN
svc-storage: COMPLETE / PARKED / GREEN
```

## 16. Safe status wording

Use this wording:

```text
svc-rewarder + svc-storage are COMPLETE / PARKED for QuickChain Phase 4 Round 2.

svc-rewarder now rejects disputed-bond challenge/freeze/appeal/slash simulation as reward, payout, wallet, ledger, staking, liquidity, bridge, or external settlement authority.

svc-storage now treats dispute/evidence artifacts as opaque b3-addressed bytes only and rejects any attempt to turn those bytes, cache state, accounting export, or usage events into slash truth, bond truth, paid unlock authority, wallet authority, ledger authority, staking, liquidity, bridge, or external settlement authority.
```

Do not say:

```text
slashing is live
bond enforcement is live
validator rewards are live
rewarder pays slashing rewards
storage verifies slash evidence truth
storage unlocks paid content from evidence
cache can unlock paid content
staking is live
liquidity is live
bridge is ready
external settlement is active
ROX/Solana is active
```

## 17. Retest commands if reopened

Only rerun if these crates are modified again.

Focused retests:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-rewarder --test quickchain_phase4_bond_dispute_reward_boundary
cargo test -p svc-rewarder --test quickchain_phase4_bond_planning_boundary
cargo test -p svc-rewarder --test quickchain_tooling_boundary
cargo clippy -p svc-rewarder --all-targets -- -D warnings

cargo test -p svc-storage --test quickchain_phase4_dispute_evidence_artifact_boundary
cargo test -p svc-storage --test quickchain_phase4_bond_artifact_boundary
cargo test -p svc-storage --test quickchain_tooling_boundary
cargo clippy -p svc-storage --all-targets -- -D warnings
```

Full park gates:

```bash
cd /Users/mymac/Desktop/RustyOnions

crates/svc-rewarder/scripts/dev-quickchain-park.sh
crates/svc-storage/scripts/dev-quickchain-park.sh
```

Expected final markers:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=17 ==
== svc-rewarder QuickChain parking gate passed ==

== svc-storage quickchain exhaustive preflight gate passed: tests=20 ==
== svc-storage QuickChain parking gate passed ==
```

## 18. Low-disk note

This pair is parked.

Do not rerun these gates unless:

```text
svc-rewarder is modified
svc-storage is modified
a downstream pair exposes a compile failure
a downstream pair exposes a real boundary regression
a final Phase 4 Round 2 audit needs targeted confirmation
```

Otherwise, move forward.

## 19. Next crate pair

Next active pair:

```text
svc-gateway + omnigate
```

Recommended codebundle commands:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Expected focus for `svc-gateway + omnigate` in Phase 4 Round 2:

```text
gateway must not expose slash/dispute execution routes
gateway must not turn dispute reports into paid unlock authority
gateway must not mutate wallet or ledger state
gateway must not claim settlement/finality/slash truth
gateway must keep paid enforcement backend-derived

omnigate must hydrate/display dispute state only if needed
omnigate must not become dispute truth
omnigate must not execute slash/capture/freeze/reward behavior
omnigate must not turn cache or hydration into paid unlock authority
omnigate must not create bridge, staking, liquidity, ROX/Solana, or external settlement behavior
```

## 20. Completion judgment

Final judgment:

```text
svc-rewarder + svc-storage are complete / parked for QuickChain Phase 4 Round 2.
```

This pair now has:

```text
rewarder disputed-bond reward boundary tests
storage dispute evidence artifact boundary tests
docs updated for Phase 4 Round 2 dispute boundaries
strict unknown-field rejection at rewarder input/output/wallet handoff surfaces
strict unknown-field rejection at storage accounting export/usage event surfaces
source scanners for dispute/slash/reward/unlock/staking/liquidity/bridge/external settlement authority
b3-only evidence artifact proof
clippy fix for rewarder test
full svc-rewarder park green
full svc-storage park green
no live slashing
no dispute payout
no validator reward
no storage unlock authority
no cache entitlement authority
no wallet/ledger mutation from these crates
no staking
no liquidity
no bridge
no external settlement
no ROX/Solana
```

Next active pair:

```text
svc-gateway + omnigate
```

### END NOTE — JUNE 26 2026 — QUICKCHAIN PHASE 4 ROUND 2 — `svc-rewarder + svc-storage`


### END NOTE - JUNE 26 2026 - 01:35 CST


### BEGIN NOTE - JUNE 26 2026 - 16:25 CST

QuickChain Phase 4 crate notes — svc-rewarder + svc-storage

Status

svc-rewarder + svc-storage are complete for QuickChain Phase 4.

This pair has now cleared all three Phase 4 rounds:

Phase 4 Round 1:
svc-rewarder added bond-planning boundary coverage.
svc-storage added bond-artifact storage boundary coverage.

Phase 4 Round 2:
svc-rewarder added disputed-bond reward boundary coverage.
svc-storage added dispute-evidence artifact boundary coverage.

Phase 4 Round 3:
svc-rewarder added controlled bond-enforcement reward boundary coverage.
svc-storage added controlled bond-enforcement artifact boundary coverage.

Final status:
Both crates are green and parked for Phase 4.

The terminal confirms the final Round 3 marker:

QuickChain Phase 4 Round 3 svc-rewarder + svc-storage clippy fix passed. 

Files touched / important areas

svc-rewarder

crates/svc-rewarder/tests/quickchain_phase4_bond_enforcement_reward_boundary.rs

Added the Phase 4 Round 3 controlled bond-enforcement reward boundary suite.

This test suite proves that controlled bond enforcement cannot become rewarder payout, penalty, wallet, ledger, staking, liquidity, bridge, validator-reward, external-chain, or public-market authority.

The tests cover:

ComputeEpochRequest rejects top-level controlled bond-enforcement authority fields.

Nested RewardPolicy rejects controlled bond-enforcement authority fields.

Nested AccountingSnapshot rejects controlled bond-enforcement authority fields.

RewardManifest remains a deterministic reward planning artifact, not enforcement truth.

SettlementBatch remains a wallet-handoff planning artifact.

WalletIssueBatch remains `/v1/issue` request preview/handoff shape, not bond enforcement, slash, capture, release, or receipt authority.

Rewarder replay/dedupe still prevents duplicate payout and does not become slash/punishment logic.

Rewarder source does not implement controlled bond-enforcement runtime authority.

The final clippy cleanup changed:

ContentCid::parse(INPUTS_CID.to_string())

to:

ContentCid::parse(INPUTS_CID)

This fixed the `unnecessary use of to_string` clippy warning without changing behavior.

crates/svc-rewarder/scripts/dev-quickchain-preflight.sh

No structural script rewrite was needed.

The script dynamically discovered the new Round 3 test automatically.

Final discovered QuickChain tests for svc-rewarder:

18 focused QuickChain tests.

svc-rewarder gates passed

Focused Round 3 test:

cargo test -p svc-rewarder --test quickchain_phase4_bond_enforcement_reward_boundary

Result:

4 passed / 0 failed

Tests inside the focused Round 3 suite:

bond_enforcement_like_replay_is_still_rewarder_dedupe_not_second_payout_or_slash — passed

rewarder_manifest_and_wallet_issue_batch_remain_planning_not_enforcement_truth — passed

rewarder_compute_request_rejects_round3_enforcement_authority_fields — passed

rewarder_source_does_not_implement_phase4_round3_bond_enforcement_authority — passed

Full preflight:

bash crates/svc-rewarder/scripts/dev-quickchain-preflight.sh

Result:

svc-rewarder quickchain exhaustive preflight gate passed: tests=18

Additional svc-rewarder gates inside preflight:

forbidden helper scan passed

format check passed

all discovered QuickChain focused tests passed

all-target tests passed

unit tests passed

integration tests passed

bench smoke passed

clippy passed

Important svc-rewarder doctrine preserved

svc-rewarder remains deterministic payout planning only.

svc-rewarder does not mutate ron-ledger.

svc-rewarder does not directly create wallet receipts.

svc-rewarder does not execute controlled bond enforcement.

svc-rewarder does not reserve slash amounts.

svc-rewarder does not release slash reserves.

svc-rewarder does not capture slash reserves.

svc-rewarder does not create validator rewards from enforcement events.

svc-rewarder does not convert bond enforcement into reward eligibility.

svc-rewarder does not become staking, liquidity, bridge, public-market, external-settlement, Solana, or ROX authority.

All economic mutation still goes through svc-wallet, with ron-ledger as truth.

svc-storage

crates/svc-storage/tests/quickchain_phase4_bond_enforcement_artifact_boundary.rs

Added the Phase 4 Round 3 controlled bond-enforcement artifact boundary suite.

This test suite proves that controlled bond-enforcement artifacts may be stored as opaque bytes by canonical b3, but svc-storage does not interpret those bytes as wallet, ledger, paid unlock, slash, capture, release, staking, liquidity, bridge, external settlement, or public-market authority.

The tests cover:

AccountingExportRequest rejects top-level controlled bond-enforcement authority fields.

UsageEventDto rejects nested controlled bond-enforcement authority fields.

Storage usage events remain metering only.

Accounting export remains usage/metering only.

Controlled bond-enforcement artifact bytes can be stored and retrieved by canonical b3.

Stored artifact bytes do not unlock paid content.

Stored artifact bytes do not authorize wallet mutation.

Stored artifact bytes do not authorize ledger mutation.

Stored artifact bytes do not authorize slash, reserve, release, or capture.

Storage source does not implement controlled bond-enforcement runtime authority.

crates/svc-storage/scripts/dev-quickchain-preflight.sh

No structural script rewrite was needed.

The script dynamically discovered the new Round 3 test automatically.

Final discovered QuickChain tests for svc-storage:

21 focused QuickChain tests.

svc-storage gates passed

Focused Round 3 test:

cargo test -p svc-storage --test quickchain_phase4_bond_enforcement_artifact_boundary

Result:

4 passed / 0 failed

Tests inside the focused Round 3 suite:

storage_usage_events_remain_metering_not_bond_enforcement_or_paid_unlock_authority — passed

enforcement_artifact_bytes_store_by_b3_without_unlock_slash_or_wallet_authority — passed

storage_accounting_export_rejects_round3_enforcement_authority_fields — passed

storage_source_does_not_implement_phase4_round3_bond_enforcement_authority — passed

Full preflight:

bash crates/svc-storage/scripts/dev-quickchain-preflight.sh

Result:

svc-storage quickchain exhaustive preflight gate passed: tests=21

Additional svc-storage gates inside preflight:

forbidden helper scan passed

format check passed

all discovered QuickChain focused tests passed

all-target tests passed

paid write tests passed

wallet receipt mode tests passed

settlement boundary tests passed

b3 integrity tests passed

range/media boundary tests passed

bench compile/smoke passed

clippy passed

Important svc-storage doctrine preserved

svc-storage remains bytes/artifacts by canonical b3 only.

b3 proves bytes, not authority.

svc-storage cache is not paid-access authority.

svc-storage cannot unlock paid content from cache alone.

svc-storage does not become wallet truth.

svc-storage does not become ledger truth.

svc-storage does not create receipts.

svc-storage does not execute bond enforcement.

svc-storage does not reserve slash amounts.

svc-storage does not release slash reserves.

svc-storage does not capture slash reserves.

svc-storage does not mutate balances.

svc-storage does not become validator, staking, liquidity, bridge, public-market, external-settlement, Solana, or ROX authority.

Paid storage still requires backend-derived wallet/ledger evidence.

Issues fixed during this pair

1. svc-rewarder clippy warning

Initial Round 3 focused tests passed, but svc-rewarder preflight stopped during clippy on an unnecessary `to_string()` call.

Fix:

Changed:

ContentCid::parse(INPUTS_CID.to_string())

to:

ContentCid::parse(INPUTS_CID)

Result:

svc-rewarder clippy passed.

2. No design or boundary failure

The failure was only a clippy cleanup in the new test file.

No production source authority issue was found.

No rewarder/storage doctrine issue was found.

No forbidden runtime behavior was introduced.

Final confirmed terminal summary

svc-rewarder:

Focused Round 3 test: 4/4 passed

Exhaustive QuickChain preflight: passed

Discovered QuickChain tests: 18

All-target tests: passed

Clippy: passed

svc-storage:

Focused Round 3 test: 4/4 passed

Exhaustive QuickChain preflight: passed

Discovered QuickChain tests: 21

All-target tests: passed

Clippy: passed

Final proof marker:

QuickChain Phase 4 Round 3 svc-rewarder + svc-storage clippy fix passed. 

Conclusion

svc-rewarder + svc-storage are complete for QuickChain Phase 4.

This crate pair is parked.

Next Phase 4 Round 3 crate pair:

svc-gateway + omnigate


### END NOTE - JUNE 26 2026 - 16:25 CST




### BEGIN NOTE - JUNE 27 2026 - 19:05 CST

## Crate Notes — QuickChain Phase 5 Round 1

## Crates: `svc-rewarder` + `svc-storage`

### Session status

```text
QuickChain Phase 5 Round 1 crate pair:
svc-rewarder + svc-storage = COMPLETE / PARKED
```

Both crates are green after the Phase 5 anchor-only evidence/artifact patch and the storage scanner fix. The final terminal output confirms:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=19
svc-rewarder QuickChain parking gate passed

svc-storage quickchain exhaustive preflight gate passed: tests=22
svc-storage QuickChain parking gate passed

QuickChain Phase 5 Round 1 svc-rewarder + svc-storage scanner fix passed.
```



---

# svc-rewarder notes

## Phase 5 Round 1 purpose

`svc-rewarder` now has a Phase 5 Round 1 boundary proving that reward manifest commitments may be referenced as **anchor evidence only**.

This does **not** make rewarder an anchor authority.

This does **not** make rewarder payout authority.

This does **not** make rewarder wallet, ledger, balance, finality, settlement, or outside-chain truth.

The safe interpretation is:

```text
A deterministic reward manifest commitment may be referenced by anchor dry-run evidence.
The report is evidence-only metadata.
svc-rewarder remains deterministic payout planning only.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
```

## Files changed

```text
crates/svc-rewarder/docs/quickchain-preflight.md
crates/svc-rewarder/tests/quickchain_phase5_anchor_evidence_boundary.rs
```

## Added test coverage

```text
quickchain_phase5_anchor_evidence_boundary
```

Focused tests added/proven:

```text
reward_manifest_commitment_can_be_referenced_as_anchor_evidence_only
rewarder_anchor_evidence_rejects_authority_flags_unknown_fields_and_bad_b3
rewarder_phase5_docs_record_anchor_only_non_authority_boundary
rewarder_runtime_source_does_not_gain_phase5_anchor_authority
```

The focused Phase 5 rewarder test passed:

```text
running 4 tests
4 passed
```



## Boundary enforced

The test-local evidence report proves these flags stay safe:

```text
report_only = true
evidence_only = true

rewarder_side_effect = false
wallet_side_effect = false
ledger_side_effect = false
payout_side_effect = false
reward_truth = false
paid_unlock_authority = false
settlement_truth = false
outside_chain_truth = false
```

## Authority fields rejected

The test rejects unknown/poison authority fields such as:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payout_executed
reward_executed
paid_unlock
settlement_status
finalized
bridge_settlement
solana
rox
```

## Runtime source scanner

The new scanner verifies `svc-rewarder/src` does not gain Phase 5 runtime authority patterns such as:

```text
anchor_payout
payout_from_anchor
settle_from_anchor
commit_from_anchor
apply_anchor
anchor_wallet_receipt
anchor_ledger_receipt
paid_unlock_from_anchor
bridge_settlement
external_settlement
solana
rox
liquidity_pool
```

## What svc-rewarder still cannot do

```text
No anchor payout execution.
No anchor-created reward truth.
No wallet mutation.
No ledger mutation.
No direct payout execution.
No fake receipts.
No fake balances.
No paid unlock from anchor evidence.
No settlement truth.
No outside-chain ROC truth.
No bridge.
No ROX runtime.
No Solana runtime.
No staking.
No liquidity.
```

## Green gate summary

```text
Focused Phase 5 test: 4/4 passed
Focused QuickChain tests discovered: 19
All focused QuickChain tests passed
All-targets test passed
Clippy passed
Parking gate passed
```

---

# svc-storage notes

## Phase 5 Round 1 purpose

`svc-storage` now has a Phase 5 Round 1 boundary proving that opaque anchor dry-run artifact bytes may be stored and retrieved by canonical `b3` only.

This does **not** make storage an anchor authority.

This does **not** make storage payment truth.

This does **not** make storage paid-unlock authority.

This does **not** make cache, CIDs, artifacts, or b3 byte identity ROC balance/finality truth.

The safe interpretation is:

```text
svc-storage may store opaque anchor dry-run artifact bytes by canonical b3.
The b3 proves byte identity only.
svc-storage remains bytes/artifact infrastructure.
Paid access remains backend wallet/gateway/omnigate derived.
svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
```

## Files changed

```text
crates/svc-storage/docs/quickchain-preflight.md
crates/svc-storage/tests/quickchain_phase5_anchor_artifact_boundary.rs
```

## Added test coverage

```text
quickchain_phase5_anchor_artifact_boundary
```

Focused tests added/proven:

```text
storage_can_retain_anchor_dry_run_artifact_bytes_by_b3_only
storage_anchor_artifact_report_rejects_authority_flags_unknown_fields_and_bad_b3
storage_anchor_artifact_report_exposes_no_payment_or_unlock_authority_keys
storage_phase5_docs_record_anchor_artifact_non_authority_boundary
storage_runtime_source_does_not_gain_phase5_anchor_authority
```

The focused Phase 5 storage test passed after scanner narrowing:

```text
running 5 tests
5 passed
```



## Boundary enforced

The test-local artifact report proves these flags stay safe:

```text
byte_identity_only = true
report_only = true
evidence_only = true

storage_side_effect = false
wallet_side_effect = false
ledger_side_effect = false
balance_truth = false
payment_truth = false
paid_unlock_authority = false
settlement_truth = false
reward_truth = false
outside_chain_truth = false
```

## Authority fields rejected

The test rejects unknown/poison authority fields such as:

```text
wallet_receipt
ledger_receipt
balance_minor
wallet_mutation
ledger_mutation
payment_receipt
paid_unlock
cache_unlock
settlement_status
finalized
reward_payout
bridge_settlement
solana
rox
```

## Scanner fix applied

The first storage run failed because the scanner searched for bare:

```text
solana
```

That was too broad because `src/config.rs` already contained a negative/deny-test spelling like:

```text
settle-on-solana
```

The fix narrowed the scanner to authority-shaped tokens:

```text
solana_runtime
solana_settlement
solana_anchor_authority
rox_runtime
rox_settlement
```

After that fix, the full storage Phase 5 test and parking gate passed. 

## Runtime source scanner

The new scanner verifies `svc-storage/src` does not gain Phase 5 runtime authority patterns such as:

```text
anchor_paid_unlock
paid_unlock_from_anchor
cache_unlock_from_anchor
settle_from_anchor
commit_from_anchor
apply_anchor
anchor_wallet_receipt
anchor_ledger_receipt
wallet_side_effect: true
ledger_side_effect: true
balance_truth: true
payment_truth: true
paid_unlock_authority: true
settlement_truth: true
reward_truth: true
outside_chain_truth: true
bridge_settlement
external_settlement
solana_runtime
solana_settlement
solana_anchor_authority
rox_runtime
rox_settlement
liquidity_pool
```

## What svc-storage still cannot do

```text
No anchor-based paid unlock.
No cache-only paid unlock.
No anchor-created payment truth.
No anchor-created balance truth.
No anchor-created reward truth.
No wallet mutation.
No ledger mutation.
No fake receipts.
No fake balances.
No settlement truth.
No outside-chain ROC truth.
No bridge.
No ROX runtime.
No Solana runtime.
No staking.
No liquidity.
```

## Green gate summary

```text
Focused Phase 5 test: 5/5 passed
Focused QuickChain tests discovered: 22
All focused QuickChain tests passed
All-targets test passed
Clippy passed
Parking gate passed
```

---

# Pair-level summary

This crate pair now proves the Phase 5 Round 1 anchor-only boundary for reward planning and storage artifacts:

```text
svc-rewarder:
  reward manifest commitments may be referenced as anchor evidence only.

svc-storage:
  opaque anchor dry-run artifact bytes may be stored and retrieved by b3 only.
```

Neither crate becomes economic authority:

```text
No wallet mutation.
No ledger mutation.
No payout execution.
No paid unlock authority.
No balance truth.
No reward truth.
No settlement truth.
No outside-chain ROC truth.
```

No forbidden scope was introduced:

```text
No ROX active runtime.
No Solana active runtime.
No public bridge.
No external settlement.
No staking.
No liquidity.
No exchange-facing logic.
No public-chain authority.
No fake receipts.
No fake balances.
No silent spend.
```

---

# Final status

```text
svc-rewarder + svc-storage
QuickChain Phase 5 Round 1
COMPLETE / PARKED
```

Current Phase 5 Round 1 progress:

```text
ron-proto + ron-ledger: COMPLETE / PARKED
svc-wallet + ron-accounting: COMPLETE / PARKED
svc-rewarder + svc-storage: COMPLETE / PARKED
```

Next crate pair:

```text
svc-gateway + omnigate
```

Next target:

```text
Gateway and omnigate may expose/hydrate anchor dry-run evidence status only.
They must not mutate ledger truth, unlock paid content from anchors, claim external settlement, or turn anchors into ROC balance/finality truth.
```


### END NOTE - JUNE 27 2026 - 19:05 CST



### BEGIN NOTE - JUNE 28 2026 - 00:40 CST

Here are the crate notes for the completed **QuickChain Phase 5 Round 2 `svc-rewarder + svc-storage` pass**. The terminal output confirms both focused DA fallback tests, both parking gates, all-targets tests, clippy, and exhaustive QuickChain preflights passed. 

QuickChain Phase 5 Round 2 Crate Notes — svc-rewarder + svc-storage

Status

COMPLETE / GREEN for QuickChain Phase 5 Round 2 on the svc-rewarder + svc-storage crate pair.

Round theme

Phase 5 Round 2: DA/archive/challenge fallback.

The purpose of this pair was to carry the DA/archive/challenge fallback boundary into reward planning and storage/artifact handling without allowing either crate to become pruning authority, paid-unlock authority, payment truth, reward truth, settlement truth, wallet truth, ledger truth, or outside-chain truth.

Crates covered

svc-rewarder

svc-storage

Terminal result summary

Focused gates passed:

svc-rewarder DA fallback reward boundary test: 5/5 passed

svc-storage DA fallback artifact boundary test: 4/4 passed

Exhaustive gates passed:

svc-rewarder QuickChain preflight passed with 20 focused QuickChain tests discovered.

svc-storage QuickChain preflight passed with 23 focused QuickChain tests discovered.

All-targets tests passed:

svc-rewarder all-targets test passed.

svc-storage all-targets test passed.

Clippy passed:

svc-rewarder clippy passed.

svc-storage clippy passed.

Parking gates passed:

svc-rewarder QuickChain parking gate passed.

svc-storage QuickChain parking gate passed.

Final terminal completion line:

QuickChain Phase 5 Round 2 svc-rewarder shadow fix and svc-rewarder + svc-storage gates completed.

Files added

svc-rewarder:

crates/svc-rewarder/tests/quickchain_phase5_da_fallback_reward_boundary.rs

svc-storage:

crates/svc-storage/tests/quickchain_phase5_da_fallback_artifact_boundary.rs

No production runtime files were changed for this pair.

The patch was intentionally test-only boundary hardening.

svc-rewarder notes

What changed

Added a Phase 5 Round 2 DA/archive/challenge fallback reward boundary test.

The new test defines a strict local evidence-only report shape for rewarder DA fallback context and proves that DA/archive/challenge evidence cannot become reward entitlement, direct payout execution, wallet mutation, ledger mutation, pruning authority, external settlement truth, or outside-chain ROC truth.

New test

quickchain_phase5_da_fallback_reward_boundary

Test count:

5 tests passed

Test coverage

The test proves:

Rewarder DA fallback evidence is report-only.

Rewarder DA fallback evidence is evidence-only.

Archive fallback must be checked.

Missing-data challenge handling must be checked.

Restore path must be checked.

Pruning remains blocked.

The report must reference a nonempty reward manifest.

Canonical b3 hashes are required.

Bad checkpoint hash rejects.

Bad data availability root rejects.

Bad reward manifest commitment rejects.

Bad challenged chunk ID rejects.

Empty payout context rejects.

Unknown authority fields reject.

Authority flags reject when enabled.

Required blocker/check flags reject when disabled.

DA fallback evidence cannot drive a second payout.

DA fallback evidence cannot drive a carrier reward.

DA fallback evidence cannot drive an archive reward.

DA fallback evidence cannot create wallet receipts.

DA fallback evidence cannot create ledger receipts.

DA fallback evidence cannot become reward truth.

Source scanner coverage

The source scanner confirms svc-rewarder does not implement DA fallback reward/runtime authority through forbidden patterns such as:

pay_from_da_fallback

reward_from_da_fallback

issue_from_da_fallback

mint_from_archive_restore

pay_from_missing_data_challenge

direct_carrier_reward

direct_archive_reward

archive_reward_receipt

carrier_reward_receipt

da_fallback_wallet_issue

da_fallback_ledger_commit

pruning authority

outside DA truth

outside chain truth

bridge settlement

external settlement

Solana runtime

ROX runtime

Important svc-rewarder invariant preserved

svc-rewarder remains deterministic payout planning only.

It may produce or reference reward manifests as planning artifacts, but it does not mutate wallet or ledger truth.

DA/archive/challenge fallback evidence can inform review context, but it cannot become payout authority, reward truth, pruning authority, settlement truth, or external-chain truth.

svc-storage notes

What changed

Added a Phase 5 Round 2 DA/archive/challenge fallback artifact boundary test.

The new test proves storage can retain and restore opaque DA fallback artifacts by canonical b3 while remaining byte storage only.

New test

quickchain_phase5_da_fallback_artifact_boundary

Test count:

4 tests passed

Test coverage

The test proves:

DA fallback artifacts can be stored by canonical b3.

DA fallback artifacts can be retrieved by full object read.

DA fallback artifacts can be retrieved by bounded range read.

DA fallback artifacts expose content-derived ETag behavior.

Artifact reports are byte-identity-only.

Artifact reports are report-only.

Artifact reports are evidence-only.

Archive fallback must be checked.

Missing-data challenge handling must be checked.

Restore path must be checked.

Pruning remains blocked.

Canonical b3 hashes are required.

Bad checkpoint hash rejects.

Bad data availability root rejects.

Bad artifact CID rejects.

Bad challenged chunk ID rejects.

Empty artifact evidence rejects.

Unknown authority fields reject.

Authority flags reject when enabled.

Required blocker/check flags reject when disabled.

Stored DA fallback artifacts do not become paid unlock authority.

Stored DA fallback artifacts do not become payment truth.

Stored DA fallback artifacts do not become reward truth.

Stored DA fallback artifacts do not become balance truth.

Stored DA fallback artifacts do not become pruning authority.

Stored DA fallback artifacts do not become settlement truth.

Stored DA fallback artifacts do not become wallet or ledger truth.

Source scanner coverage

The source scanner confirms svc-storage does not construct DA fallback artifact/unlock/pruning authority through forbidden patterns such as:

unlock_from_da_fallback

paid_unlock_from_da_fallback

unlock_from_archive_restore

unlock_from_missing_data_challenge

da_fallback_unlock_authority

archive_restore_payment_truth

missing_data_payment_truth

artifact_payment_truth

artifact_balance_truth

artifact_reward_truth

prune_from_da_fallback

allow_pruning_from_da

pruning authority

outside DA truth

outside chain truth

wallet mutation from archive

ledger mutation from archive

bridge settlement

external settlement

Solana runtime

ROX runtime

Important svc-storage invariant preserved

svc-storage remains bytes/artifacts by canonical b3.

It can store and retrieve DA/archive/challenge artifacts as opaque bytes, but b3 proves byte identity only.

Storage does not become payment truth, paid-unlock authority, reward truth, balance truth, pruning authority, settlement truth, wallet truth, ledger truth, or outside-chain truth.

Bug encountered and fixed

Initial failure

svc-rewarder failed to compile because a local variable named report shadowed the helper function report().

Compiler error:

expected function, found RewarderDaFallbackEvidenceReport

Cause

The test used:

let mut report = report();

After that local binding, later calls to report() tried to call the local variable instead of the helper function.

Fix

Renamed the local variables in the bad-hash/empty-context test to specific names:

bad_checkpoint_report

bad_da_root_report

bad_commitment_report

bad_chunk_report

empty_manifest_report

After the shadowing fix, the focused rewarder DA fallback test passed 5/5 and the full pair parking gates passed.

Architecture result

This crate pair is now aligned with Phase 5 Round 2 doctrine:

DA/archive/challenge fallback evidence remains evidence-only.

Archive fallback checks are present.

Missing-data challenge checks are present.

Restore path checks are present.

Pruning remains blocked.

Rewarder remains payout planning only.

Storage remains bytes/artifacts only.

No direct wallet mutation was introduced.

No direct ledger mutation was introduced.

No paid unlock authority was introduced.

No payment truth was introduced.

No reward truth was introduced.

No balance truth was introduced.

No settlement truth was introduced.

No outside DA truth was introduced.

No outside chain truth was introduced.

No ROX/Solana runtime was introduced.

No public bridge was introduced.

No external settlement path was introduced.

No staking, liquidity, or exchange-facing logic was introduced.

Do not regress

Do not allow svc-rewarder to pay from DA fallback evidence.

Do not allow svc-rewarder to pay from missing-data challenges.

Do not allow svc-rewarder to mint or issue from archive restore material.

Do not allow svc-rewarder to treat carrier/archive evidence as direct ROC entitlement.

Do not allow svc-rewarder to create wallet or ledger receipts.

Do not allow svc-rewarder to bypass svc-wallet.

Do not allow svc-storage artifacts to unlock paid content.

Do not allow svc-storage artifacts to become payment truth.

Do not allow svc-storage artifacts to become reward truth.

Do not allow svc-storage artifacts to become balance truth.

Do not allow svc-storage artifacts to become pruning authority.

Do not allow b3 presence alone to imply paid access.

Do not allow archive restore evidence to mutate wallet or ledger state.

Do not allow missing-data challenge evidence to mutate wallet or ledger state.

Do not add bridge, external settlement, ROX, Solana, staking, liquidity, or exchange-facing logic.

Remaining risks

This pair adds boundary tests, not a full DA/archive restoration runtime.

The actual archive carrier workflow remains future work.

The actual missing-data challenge runtime remains future work.

The actual restore-from-archive path remains future work.

Pruning is still blocked.

Downstream crates still need their Phase 5 Round 2 passes so gateway, omnigate, index, policy, and CrabLink do not interpret DA fallback artifacts as unlock, finality, settlement, or pruning authority.

Next crate pair

svc-gateway + omnigate

Next pair objective

Carry Phase 5 Round 2 DA/archive/challenge fallback boundaries into the public gateway and hydration/coordinator layer.

Expected direction:

svc-gateway may expose/read/proxy DA fallback evidence only as non-authoritative status or artifact context.

svc-gateway must not treat DA fallback evidence as paid unlock authority.

svc-gateway must not treat archive restore evidence as pruning authority.

svc-gateway must not mutate wallet or ledger state from DA fallback material.

omnigate may hydrate DA fallback artifact/status context, but must not turn it into access truth, payment truth, settlement truth, finality truth, or pruning authority.

omnigate must continue to rely on backend wallet/ledger truth for paid unlocks.

Suggested next focused tests

svc-gateway:

quickchain_phase5_da_fallback_gateway_boundary

omnigate:

quickchain_phase5_da_fallback_omnigate_boundary

Suggested next command pattern

cargo fmt -p svc-gateway -p omnigate

cargo test -p svc-gateway --test quickchain_phase5_da_fallback_gateway_boundary

cargo test -p omnigate --test quickchain_phase5_da_fallback_omnigate_boundary

bash crates/svc-gateway/scripts/dev-quickchain-park.sh

bash crates/omnigate/scripts/dev-quickchain-park.sh


### END NOTE - JUNE 28 2026 - 00:40 CST




### BEGIN NOTE - JUNE 28 2026 - 20:00 CST

Your terminal output confirms both crates are green: `svc-rewarder` focused Phase 5 Round 3 test **5/5**, `svc-storage` focused Phase 5 Round 3 test **6/6**, exhaustive preflight passed with **21 rewarder QuickChain tests** and **24 storage QuickChain tests**, and both park gates passed. 

### BEGIN NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-rewarder

---

# CARRY-OVER NOTES — svc-rewarder QuickChain Phase 5 Round 3

**Date:** 2026-06-28
**Crate:** `svc-rewarder`
**Phase/Round:** QuickChain Phase 5 Round 3
**Status:** Complete / parked green
**Verdict:** `svc-rewarder` QuickChain work is complete for the current buildplan through Phase 5. The crate now has the final Phase 5 Round 3 selected external-posture boundary proving that external posture evidence may reference reward manifest commitments only as report/evidence metadata, but cannot become direct reward eligibility, payout execution, wallet mutation, ledger mutation, reward truth, settlement truth, bridge authority, market authority, liquidity authority, ROX/Solana runtime authority, or outside-program authority.

---

## 0) TL;DR

`svc-rewarder` now has a Phase 5 Round 3 boundary test for the selected external integration posture.

The selected posture remains:

```text
anchor-only
report-only
evidence-only
wallet/ledger truth canonical
```

`svc-rewarder` remains:

```text
deterministic payout planning only
not wallet mutation authority
not ledger mutation authority
not reward truth
not payout execution truth
not external settlement authority
```

External posture evidence can reference a reward manifest commitment, run key, inputs CID, checkpoint commitment, and payout count as evidence. It cannot turn that evidence into protocol ROC allocation, wallet mutation, direct payout execution, paid unlock, market authority, liquidity authority, bridge authority, or outside-chain truth.

---

## 1) Files added or changed

### Added

```text
crates/svc-rewarder/tests/quickchain_phase5_external_posture_reward_boundary.rs
```

### Repaired

```text
crates/svc-rewarder/tests/quickchain_phase5_external_posture_reward_boundary.rs
```

The repair corrected the test constant:

```text
POLICY_HASH = b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
```

The prior hash string was malformed, and `svc-rewarder` correctly rejected it with:

```text
BadRequest("policy_hash must be b3:<64 lowercase hex chars>")
```

After replacement with a valid `b3:<64 lowercase hex>` value, the focused test passed.

---

## 2) New focused test target

```text
cargo test -p svc-rewarder --test quickchain_phase5_external_posture_reward_boundary
```

Final result:

```text
running 5 tests
5 passed
0 failed
```

Test cases:

```text
rewarder_external_posture_report_is_anchor_only_evidence_and_not_reward_authority
rewarder_external_posture_report_rejects_authority_flags_and_unknown_fields
rewarder_compute_request_rejects_external_posture_authority_poison_fields
reward_manifest_and_settlement_batch_do_not_become_external_posture_payout_or_truth
rewarder_source_does_not_construct_external_posture_runtime_authority
```

---

## 3) What the new test proves

The new boundary test defines a local test-only external posture report:

```text
RewarderExternalPostureEvidenceReport
```

Schema:

```text
svc-rewarder.quickchain-external-posture-evidence.v1
```

The report can bind:

```text
chain_id
epoch_id
source
posture_id
chosen_posture
posture_semantics
checkpoint_commitment
reward_manifest_commitment
reward_run_key
reward_inputs_cid
payout_count
produced_at_ms
```

Required true flags:

```text
anchor_only_selected
report_only
evidence_only
wallet_ledger_truth_canonical
```

Required false flags:

```text
direct_reward_eligibility
rewarder_side_effect
wallet_side_effect
ledger_side_effect
payout_side_effect
reward_truth
paid_unlock_authority
settlement_truth
outside_da_truth
outside_chain_truth
outside_program_authority
bridge_authority
exchange_facing_authority
public_market
liquidity_enabled
bonded_economy_authority
```

The report rejects unknown authority fields through `serde(deny_unknown_fields)`.

---

## 4) Rewarder request boundary

The test confirms `ComputeEpochRequest` rejects external posture poison fields such as:

```text
external_posture
chosen_external_posture
direct_reward_eligibility
payout_from_external_posture
wallet_side_effect
ledger_side_effect
reward_truth
paid_unlock_authority
bridge_authority
outside_program_authority
exchange_facing_authority
public_market
liquidity_enabled
```

This keeps Phase 5 Round 3 posture out of the live reward compute request surface.

---

## 5) Reward manifest and wallet handoff boundary

The test confirms reward manifests and settlement batches do not expose external posture authority keys.

They remain normal reward-planning artifacts and wallet-handoff plans.

Forbidden concepts stay absent from manifest/settlement JSON:

```text
external_posture
chosen_external_posture
external_da_selected
external_l2_selected
hybrid_selected
direct_reward_eligibility
payout_from_external_posture
reward_truth
settlement_truth
outside_chain_truth
paid_unlock_authority
bridge_authority
outside_program_authority
exchange_facing_authority
public_market
liquidity_enabled
rox_runtime
solana_runtime
```

---

## 6) Source authority scan

The new source scanner confirms `svc-rewarder/src` does not construct Phase 5 Round 3 external posture runtime authority.

It blocks source-level drift toward:

```text
external_posture_payout
payout_from_external_posture
reward_from_external_posture
settle_from_external_posture
commit_from_external_posture
apply_external_posture
external_posture_wallet_receipt
external_posture_ledger_receipt
paid_unlock_from_external_posture
direct_reward_eligibility: true
rewarder_side_effect: true
wallet_side_effect: true
ledger_side_effect: true
payout_side_effect: true
reward_truth: true
settlement_truth: true
outside_da_truth: true
outside_chain_truth: true
outside_program_authority: true
bridge_authority: true
exchange_facing_authority: true
public_market: true
liquidity_enabled: true
bonded_economy_authority: true
external_da_selected: true
external_l2_selected: true
hybrid_selected: true
solana_runtime
rox_runtime
bridge_settlement
exchange_facing
liquidity_pool
```

---

## 7) Preflight and park status

`svc-rewarder` dynamic QuickChain preflight discovered:

```text
21 focused QuickChain tests
```

The new test was discovered automatically because the crate preflight scans:

```text
crates/svc-rewarder/tests/quickchain*.rs
```

No preflight script edit was required.

Park result:

```text
svc-rewarder quickchain exhaustive preflight gate passed: tests=21
svc-rewarder QuickChain parking gate passed
```

All-targets result:

```text
svc-rewarder all-targets test: passed
```

Clippy result:

```text
cargo clippy -p svc-rewarder --all-targets -- -D warnings
passed
```

---

## 8) Doctrine preserved

`svc-rewarder` remains aligned with the QuickChain doctrine:

```text
rewarder plans payouts
rewarder does not mutate wallet
rewarder does not mutate ledger
rewarder does not create balances
rewarder does not create receipts
rewarder does not create finality
rewarder does not create roots
rewarder does not execute external settlement
rewarder does not turn raw engagement into direct protocol ROC
```

The economic truth path remains:

```text
ron-proto DTOs
→ ron-ledger durable truth
→ svc-wallet mutation front-door
→ ron-accounting snapshots/reports
→ svc-rewarder payout planning only
```

---

## 9) Completion status

For `svc-rewarder`:

```text
QuickChain Phase 0: complete
QuickChain Phase 1: complete
QuickChain Phase 2: complete
QuickChain Phase 3: complete
QuickChain Phase 4: complete
QuickChain Phase 5 Round 1: complete
QuickChain Phase 5 Round 2: complete
QuickChain Phase 5 Round 3: complete

Current planned QuickChain work: complete / parked green
```

---

## 10) Future caution

Do not add any of the following to `svc-rewarder`:

```text
external posture payout execution
external posture reward eligibility
direct wallet mutation
direct ledger mutation
external settlement
public bridge
ROX runtime
Solana runtime
staking
liquidity
exchange-facing logic
outside-program authority
paid unlock authority
reward truth from external evidence
```

`svc-rewarder` can continue to produce deterministic payout plans and evidence artifacts, but only `svc-wallet` may mutate wallet state, and only `ron-ledger` remains durable economic truth.

---

### END NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-rewarder

### BEGIN NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-storage

---

# CARRY-OVER NOTES — svc-storage QuickChain Phase 5 Round 3

**Date:** 2026-06-28
**Crate:** `svc-storage`
**Phase/Round:** QuickChain Phase 5 Round 3
**Status:** Complete / parked green
**Verdict:** `svc-storage` QuickChain work is complete for the current buildplan through Phase 5. The crate now has the final Phase 5 Round 3 selected external-posture artifact boundary proving that external posture artifacts may be stored and retrieved only as opaque canonical `b3` bytes. Those bytes cannot become paid unlock authority, payment truth, wallet truth, ledger truth, reward truth, settlement truth, pruning authority, bridge authority, market authority, liquidity authority, ROX/Solana runtime authority, or outside-program authority.

---

## 0) TL;DR

`svc-storage` now has a Phase 5 Round 3 external posture artifact boundary.

The selected posture remains:

```text
anchor-only
artifact-reference-only
evidence-only
byte-identity-only
wallet/ledger truth canonical
```

`svc-storage` remains:

```text
bytes by canonical b3 only
not wallet truth
not ledger truth
not payment truth
not reward truth
not settlement truth
not paid unlock authority
not pruning authority
not bridge authority
not external settlement authority
```

External posture artifacts can be stored, retrieved, ranged, and verified by canonical `b3`. They cannot unlock paid content or mutate economic state.

---

## 1) Files added or changed

### Added

```text
crates/svc-storage/tests/quickchain_phase5_external_posture_artifact_boundary.rs
```

No production source changes were required.

No preflight script edit was required because `svc-storage` dynamically discovers `quickchain*.rs` tests.

---

## 2) New focused test target

```text
cargo test -p svc-storage --test quickchain_phase5_external_posture_artifact_boundary
```

Final result:

```text
running 6 tests
6 passed
0 failed
```

Test cases:

```text
storage_external_posture_artifact_report_is_anchor_only_byte_reference_only
storage_external_posture_artifact_report_rejects_authority_flags_and_unknown_fields
external_posture_artifact_bytes_store_by_b3_without_unlock_or_payment_authority
storage_accounting_export_rejects_external_posture_authority_fields
storage_usage_event_export_remains_metering_not_external_posture_authority
storage_source_does_not_construct_external_posture_runtime_or_unlock_authority
```

---

## 3) What the new test proves

The test defines a local test-only artifact report:

```text
StorageExternalPostureArtifactReport
```

Schema:

```text
svc-storage.quickchain-external-posture-artifact.v1
```

The report can bind:

```text
chain_id
artifact_cid
checkpoint_commitment
artifact_kind
chosen_posture
posture_semantics
source
produced_at_ms
```

Required true flags:

```text
anchor_only_selected
artifact_reference_only
evidence_only
byte_identity_only
wallet_ledger_truth_canonical
```

Required false flags:

```text
storage_side_effect
paid_unlock_authority
payment_truth
wallet_truth
ledger_truth
reward_truth
settlement_truth
outside_da_truth
outside_chain_truth
outside_program_authority
bridge_authority
exchange_facing_authority
public_market
liquidity_enabled
pruning_authority
bonded_economy_authority
```

The report rejects unknown authority fields through `serde(deny_unknown_fields)`.

---

## 4) b3 byte-storage boundary

The test stores external posture artifact bytes in `MemoryStorage`.

It verifies:

```text
CID is derived from the bytes
CID is canonical b3:<64 lowercase hex>
stored artifact is discoverable only by exact b3
HEAD returns exact length and matching ETag
GET full returns the original bytes
GET range returns bounded bytes only
```

This proves the storage path treats external posture artifacts as byte-addressed objects only.

Important boundary:

```text
b3 proves byte identity only
b3 does not prove paid access
b3 does not prove wallet truth
b3 does not prove ledger truth
b3 does not prove settlement truth
b3 does not prove pruning authority
```

---

## 5) Accounting export boundary

The test confirms `AccountingExportRequest` and nested `UsageEventDto` reject external posture authority poison fields.

Rejected top-level and nested fields include:

```text
external_posture
chosen_external_posture
paid_unlock_authority
payment_truth
wallet_truth
ledger_truth
reward_truth
settlement_truth
outside_da_truth
outside_chain_truth
outside_program_authority
bridge_authority
exchange_facing_authority
public_market
liquidity_enabled
pruning_authority
```

This preserves the accounting boundary:

```text
usage events are metering only
accounting export is not balance truth
accounting export is not paid unlock authority
accounting export is not settlement authority
```

---

## 6) Usage event export boundary

The test confirms storage usage/accounting export remains ordinary metering:

```text
metric_kind = bytes_stored
source_service = svc-storage
```

The serialized accounting export does not expose:

```text
external_posture
chosen_external_posture
paid_unlock_authority
payment_truth
wallet_truth
ledger_truth
reward_truth
settlement_truth
outside_chain_truth
bridge_authority
outside_program_authority
exchange_facing_authority
public_market
liquidity_enabled
pruning_authority
```

This keeps storage metering distinct from economic authority.

---

## 7) Source authority scan

The new source scanner confirms `svc-storage/src` does not construct external posture runtime or unlock authority.

It blocks source-level drift toward:

```text
external_posture_paid_unlock
paid_unlock_from_external_posture
unlock_from_external_posture
unlock_from_posture_artifact
external_posture_wallet_receipt
external_posture_payment_truth
external_posture_balance_truth
external_posture_settlement_truth
cache_external_posture_authority
cache_unlock_authority_from_posture
storage_unlock_authority_from_posture
outside_da_truth: true
outside_chain_truth: true
outside_program_authority: true
bridge_authority: true
exchange_facing_authority: true
public_market: true
liquidity_enabled: true
pruning_authority: true
bonded_economy_authority: true
solana_runtime
rox_runtime
bridge_settlement
exchange_facing
liquidity_pool
```

---

## 8) Preflight and park status

`svc-storage` dynamic QuickChain preflight discovered:

```text
24 focused QuickChain tests
```

The new test was discovered automatically because the crate preflight scans:

```text
crates/svc-storage/tests/quickchain*.rs
```

Park result:

```text
svc-storage quickchain exhaustive preflight gate passed: tests=24
svc-storage QuickChain parking gate passed
```

All-targets result:

```text
svc-storage all-targets test: passed
```

Clippy result:

```text
cargo clippy -p svc-storage --all-targets -- -D warnings
passed
```

---

## 9) Doctrine preserved

`svc-storage` remains aligned with QuickChain and CrabLink/Tauri doctrine:

```text
storage stores bytes by b3
cache is convenience only
cache is not paid-access authority
storage is not wallet truth
storage is not ledger truth
storage is not reward truth
storage is not settlement truth
storage is not finality truth
storage artifacts are not roots
storage artifacts are not validator authority
storage artifacts are not bridge authority
storage artifacts are not pruning authority
```

Paid storage remains gated by backend-derived wallet proof paths, not cache or artifact presence alone.

---

## 10) Completion status

For `svc-storage`:

```text
QuickChain Phase 0: complete
QuickChain Phase 1: complete
QuickChain Phase 2: complete
QuickChain Phase 3: complete
QuickChain Phase 4: complete
QuickChain Phase 5 Round 1: complete
QuickChain Phase 5 Round 2: complete
QuickChain Phase 5 Round 3: complete

Current planned QuickChain work: complete / parked green
```

---

## 11) Future caution

Do not add any of the following to `svc-storage`:

```text
external posture paid unlock
unlock from external posture artifact
cache-based paid unlock authority
payment truth from artifact bytes
wallet truth from artifact bytes
ledger truth from artifact bytes
reward truth from artifact bytes
settlement truth from artifact bytes
external settlement
public bridge
ROX runtime
Solana runtime
staking
liquidity
exchange-facing logic
outside-program authority
pruning authority before DA/challenge/archive fallback is proven and authorized
```

`svc-storage` can store and serve canonical `b3` artifacts, but backend wallet/ledger paths remain the economic truth.

---

## 12) Next crate-pair handoff

Next crate pair:

```text
svc-gateway + omnigate
```

Expected Phase 5 Round 3 posture there:

```text
svc-gateway:
  selected external posture may be exposed only as evidence/status metadata.
  it must not become paid unlock authority.
  it must not mutate wallet or ledger truth.
  it must not create bridge, settlement, market, liquidity, ROX/Solana, or outside-program authority.

omnigate:
  selected external posture may hydrate/read evidence only.
  it must not become content unlock authority.
  it must not bypass paid gates.
  it must not mutate wallet or ledger truth.
  it must not treat external evidence as finality, balance, receipt, or settlement truth.
```

---

### END NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-storage


### END NOTE - JUNE 28 2026 - 20:00 CST


### BEGIN NOTE - JUNE 29 2026 - 14:15 CST

Yes — the previous pair is **svc-storage + svc-index**, and the terminal output shows both focused tests passed, both all-target suites passed, both clippy runs passed, and both crate-local parking gates passed. `svc-storage` parked with **24 focused QuickChain tests**, and `svc-index` parked with **19 focused QuickChain tests**. 

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-storage

## Status

`svc-storage` is GREEN / PARKED for Internal ROC Beta Phase 1 Round 1 on the paid-content artifact boundary slice.

This crate pair was the fourth completed backend pair in the active Internal ROC Beta Phase 1 Round 1 flow:

```text
1. ron-proto + ron-ledger        GREEN / PARKED
2. svc-wallet + ron-accounting   GREEN / PARKED
3. svc-rewarder + ron-policy     GREEN / PARKED
4. svc-storage + svc-index       GREEN / PARKED
5. svc-gateway + omnigate        NEXT / final backend pair for Round 1
```

This does not mean all of Phase 1 is complete.

It means the `svc-storage` side of the `svc-storage + svc-index` crate pair is complete for this artifact/payment-truth boundary proof slice.

## Round purpose

Internal ROC Beta Phase 1 is proving repeatable paid post, paid comment, paid article, and generic content_view flows using the existing wallet/ledger path.

For `svc-storage`, the goal was to prove that paid-content bytes and descriptors can exist as b3-backed artifacts without storage becoming payment truth.

`svc-storage` may store bytes.

`svc-storage` may return b3-addressed content.

`svc-storage` may expose range reads.

`svc-storage` may emit metering/usage events.

`svc-storage` must not create receipt truth, balance truth, entitlement truth, paid unlock authority, finality truth, wallet authority, or ledger authority.

## File added

```text
crates/svc-storage/tests/internal_roc_beta_paid_content_artifact_boundary.rs
```

## Focused test run

```bash
cargo test -p svc-storage --test internal_roc_beta_paid_content_artifact_boundary
```

## Focused test result

```text
running 4 tests

storage_usage_events_are_metering_only_not_receipt_balance_or_unlock_truth ... ok
paid_content_storage_descriptor_rejects_authority_poison_fields ... ok
paid_post_comment_article_and_content_view_artifacts_store_by_b3_not_payment_truth ... ok
storage_source_does_not_construct_paid_authority_shortcuts_for_artifacts ... ok

test result: ok. 4 passed; 0 failed
```

## Parking / preflight result

The crate-local parking gate passed.

Terminal proof:

```text
== svc-storage quickchain exhaustive preflight gate passed: tests=24 ==
== svc-storage QuickChain parking gate passed ==
```

The park gate also ran:

```text
forbidden helper scan
cargo fmt --check
24 focused QuickChain tests
svc-storage all-targets test
svc-storage clippy
forbidden-scope marker
```

No checked-in Python helpers were found under `crates/svc-storage`.

Clippy completed cleanly.

## Focused proof details

The new test proves:

```text
paid_post artifacts store by canonical b3
paid_comment artifacts store by canonical b3
paid_article artifacts store by canonical b3
content_view artifacts store by canonical b3
stored bytes roundtrip exactly
HEAD metadata remains byte/content metadata
ETag remains content-hash material, not payment proof
bounded range reads work
paid descriptors are display/policy metadata only
usage events are metering only
storage source does not construct paid authority shortcuts
```

## Paid-content coverage

The focused test directly covered these beta labels:

```text
paid_post
paid_comment
paid_article
content_view
```

This gives Phase 1 Round 1 direct storage proof for the paid-content surfaces we are targeting.

## Authority poison rejected

The focused descriptor test rejects fields such as:

```text
wallet_mutation
ledger_mutation
balance_truth
receipt_truth
paid_unlock_authority
entitlement_truth
client_finality_claim
cache_unlock_authority
gateway_receipt_truth
omnigate_receipt_truth
receipt_id
receipt_hash
receipt_root
receipt_proof
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
bridge_txid
staking_position_id
liquidity_pool_id
external_settlement_id
raw_engagement_mints_roc
```

## Boundaries preserved

`svc-storage` remains bytes/artifacts by b3 only.

Allowed:

```text
store canonical b3-addressed bytes
retrieve full objects
retrieve bounded ranges
return byte/content metadata
accept paid-write admission evidence through existing backend path
emit usage/metering events
export metering to accounting as non-authority
label storage admission distinctly from economic finality
```

Forbidden and still not implemented:

```text
direct wallet mutation
direct ledger mutation
receipt creation
balance truth
paid unlock truth
entitlement truth
finality truth
cache-only unlock
b3-only paid unlock
manifest-only paid unlock
policy-only paid unlock
bridge runtime
staking runtime
liquidity runtime
external settlement
exchange-facing logic
raw engagement direct minting
```

## Existing QuickChain boundary stayed green

Existing tests confirmed:

```text
storage remains bytes by canonical b3 only
cache is not paid-access authority
wallet/ledger truth stays backend-owned
no roots
no checkpoints
no validators
no settlement authority
no anchors as runtime authority
no bridges
no staking
no liquidity
```

The parking marker explicitly preserved:

```text
svc-storage remains bytes by canonical b3 only; cache is not paid-access authority; wallet/ledger truth stays backend-owned
```

## Architecture result

This crate now has direct Internal ROC Beta proof that `svc-storage` can participate in paid post/comment/article/content_view flows without becoming economic authority.

Correct flow remains:

```text
backend wallet/ledger receipt/access truth
→ paid enforcement path
→ storage serves b3 bytes
→ accounting receives metering only
```

Incorrect flow remains blocked:

```text
stored bytes
→ paid unlock truth

b3 existence
→ receipt truth

cache hit
→ entitlement truth

usage event
→ balance truth
```

## Final svc-storage status

```text
Internal ROC Beta Phase 1 Round 1
svc-storage
GREEN / PARKED
```

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-storage

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-index

## Status

`svc-index` is GREEN / PARKED for Internal ROC Beta Phase 1 Round 1 on the paid-content pointer boundary slice.

This crate pair was the fourth completed backend pair in the active Internal ROC Beta Phase 1 Round 1 flow:

```text
1. ron-proto + ron-ledger        GREEN / PARKED
2. svc-wallet + ron-accounting   GREEN / PARKED
3. svc-rewarder + ron-policy     GREEN / PARKED
4. svc-storage + svc-index       GREEN / PARKED
5. svc-gateway + omnigate        NEXT / final backend pair for Round 1
```

This does not mean all of Phase 1 is complete.

It means the `svc-index` side of the `svc-storage + svc-index` crate pair is complete for this pointer/payment-truth boundary proof slice.

## Round purpose

Internal ROC Beta Phase 1 is proving repeatable paid post, paid comment, paid article, and generic content_view flows using the existing wallet/ledger path.

For `svc-index`, the goal was to prove that paid-content names, site pointers, asset pointers, manifest pointers, and provider responses remain lookup/navigation metadata only.

`svc-index` may store pointers.

`svc-index` may resolve names.

`svc-index` may return manifest references.

`svc-index` may return provider references.

`svc-index` must not create receipt truth, balance truth, entitlement truth, paid unlock authority, finality truth, wallet authority, or ledger authority.

## File added

```text
crates/svc-index/tests/internal_roc_beta_paid_content_pointer_boundary.rs
```

## Focused test run

```bash
cargo test -p svc-index --test internal_roc_beta_paid_content_pointer_boundary
```

## Focused test result

```text
running 5 tests

normalizers_preserve_content_and_navigation_identity_without_economic_authority ... ok
paid_post_comment_article_and_content_view_pointers_are_lookup_metadata_only ... ok
resolve_and_provider_responses_are_navigation_not_entitlement_truth ... ok
paid_content_pointer_inputs_reject_receipt_balance_unlock_and_finality_poison ... ok
index_source_does_not_construct_paid_authority_shortcuts ... ok

test result: ok. 5 passed; 0 failed
```

## Parking / preflight result

The crate-local parking gate passed.

Terminal proof:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=19 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

The park gate also ran:

```text
cargo fmt
19 focused QuickChain tests
existing svc-index regressions
all-targets tests
clippy
forbidden runtime scope marker
```

Clippy completed cleanly.

## Focused proof details

The new test proves:

```text
paid_post pointers roundtrip as lookup metadata
paid_comment pointers roundtrip as lookup metadata
paid_article pointers roundtrip as lookup metadata
content_view pointers roundtrip as lookup metadata
asset manifest pointers remain references only
site manifest pointers remain references only
legacy name pointers resolve to manifest CIDs only
owner wallet fields remain reference strings only
owner passport fields remain reference strings only
resolve responses are navigation metadata only
provider responses are navigation metadata only
normalizers preserve identity but do not create economic proof
index source does not construct paid authority shortcuts
```

## Paid-content coverage

The focused test directly covered these beta labels:

```text
paid_post
paid_comment
paid_article
content_view
```

It also covered these pointer forms:

```text
asset manifest pointer
site manifest pointer
legacy name pointer
resolve response
provider response
crab:// navigation route metadata
```

## Authority poison rejected

The focused test rejects authority-shaped fields in pointer inputs and resolve shapes, including:

```text
wallet_mutation
ledger_mutation
balance_truth
receipt_truth
paid_unlock_authority
entitlement_truth
client_finality_claim
cache_unlock_authority
gateway_receipt_truth
omnigate_receipt_truth
receipt_id
receipt_hash
receipt_root
receipt_proof
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
bridge_txid
bridge_proof
staking_position_id
liquidity_pool_id
external_settlement_id
raw_engagement_mints_roc
```

## Boundaries preserved

`svc-index` remains lookup/pointer/navigation infrastructure only.

Allowed:

```text
store asset manifest pointers
store site manifest pointers
resolve names to manifest references
return provider references
normalize b3 CIDs
normalize asset kinds
normalize site names
carry owner wallet account as reference string
carry owner passport subject as reference string
support crab:// navigation metadata
```

Forbidden and still not implemented:

```text
direct wallet mutation
direct ledger mutation
receipt creation
balance truth
paid unlock truth
entitlement truth
finality truth
index-only unlock
pointer-only unlock
manifest-only unlock
cache-only unlock
bridge runtime
staking runtime
liquidity runtime
external settlement
exchange-facing logic
raw engagement direct minting
```

## Existing QuickChain boundary stayed green

Existing tests confirmed:

```text
svc-index is lookup/pointer-only
owner wallet metadata is allowed only as reference metadata
stored pointer wire shapes have no paid or chain authority
routes do not expose paid unlock or settlement authority
source has no direct value-plane or QuickChain runtime dependencies
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
```

The parking marker explicitly preserved:

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

## Architecture result

This crate now has direct Internal ROC Beta proof that `svc-index` can participate in paid post/comment/article/content_view flows without becoming economic authority.

Correct flow remains:

```text
creator/content manifest
→ svc-index pointer/reference graph
→ gateway/omnigate lookup
→ backend wallet/ledger receipt/access truth
→ paid render
```

Incorrect flow remains blocked:

```text
index pointer
→ paid unlock truth

manifest reference
→ receipt truth

owner wallet reference string
→ spend authority

resolve response
→ entitlement truth
```

## Final svc-index status

```text
Internal ROC Beta Phase 1 Round 1
svc-index
GREEN / PARKED
```

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-index


### END NOTE - JUNE 29 2026 - 14:15 CST


### BEGIN NOTE - JULY 1 2026 - 20:50 CST


```

Your aggregate gate passed all focused `svc-storage` and `svc-index` stabilization/preflight checks, including paid-write verifier, settlement, wallet mode, accounting export, paid storage loop, b3 integrity, paid cache, no-direct-mutation, value-loop boundary, index pointer authority, route boundary, value-loop boundary, HTTP contract, integration, and clippy for both crates. The final park output explicitly says the storage b3/artifact/admission boundary and index pointer/lookup boundary are locked, backend receipt/access truth is preserved, and no fake receipt/balance/finality, paid unlock truth, cache-only unlock, direct wallet/ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement was introduced. 

## Pair 5 — `svc-storage + svc-index`

### What we changed

For `svc-storage`, we added/locked:

```text id="7vgbcc"
crates/svc-storage/docs/internal-roc-stabilization-b3-artifact-non-authority.md
crates/svc-storage/tests/internal_roc_stabilization_b3_artifact_non_authority_boundary.rs
crates/svc-storage/scripts/dev-internal-roc-stabilization-b3-artifact-preflight.sh
```

For `svc-index`, we added/locked:

```text id="j0oqob"
crates/svc-index/docs/internal-roc-stabilization-pointer-non-authority.md
crates/svc-index/tests/internal_roc_stabilization_pointer_non_authority_boundary.rs
crates/svc-index/scripts/dev-internal-roc-stabilization-pointer-preflight.sh
```

And the aggregate pair gate:

```text id="0s4064"
scripts/dev-internal-roc-stabilization-storage-index-park.sh
```

We also repaired two older `svc-storage` economics regressions so they use the correct legacy paid-action economics fixture instead of the canonical Phase 5 `configs/roc-economics.toml`.

## What we accomplished

`svc-storage` is now locked as **b3 byte/artifact storage plus paid-admission participant only**.

```text id="8q0e94"
- stores canonical b3-addressed bytes
- serves object metadata, full reads, and range reads
- derives b3 from bytes
- verifies paid-write admission evidence
- supports wallet receipt mode
- supports wallet capture/release settlement through svc-wallet
- exports usage/metering to accounting
- preserves paid estimate behavior
- does not create receipt truth
- does not create balance truth
- does not create finality truth
- does not unlock paid content from cache, b3, ETag, usage events, or storage admission alone
```

`svc-index` is now locked as **pointer/lookup/navigation metadata only**.

```text id="kcjuvm"
- stores asset CID → manifest CID pointers
- stores site/name → manifest CID pointers
- resolves provider and manifest lookup metadata
- normalizes b3 CIDs, site names, asset kinds, wallet/passport owner references
- owner_wallet_account remains reference metadata only
- owner_passport_subject remains reference metadata only
- index/cache/resolve/provider responses are not entitlement truth
- pointers are not receipts
- manifests are not balances
- provider lookup is not settlement
```

## Boundary we proved

Correct path:

```text id="wl3bne"
backend wallet/ledger receipt/access truth
→ gateway/omnigate enforcement
→ svc-index pointer lookup
→ svc-storage b3 artifact read/write
→ CrabLink display/render
```

Forbidden path:

```text id="dtk3gx"
b3 exists
→ paid unlock

ETag exists
→ receipt proof

storage cache hit
→ entitlement

usage event
→ balance truth

index pointer
→ paid access proof

manifest pointer
→ receipt truth

owner wallet reference
→ spend authority

resolve response
→ finality
```

## Final record note

```text id="2x7efp"
Pair 5 completed the b3 artifact storage and pointer lookup non-authority stabilization layer. svc-storage is now parked as canonical b3 byte/artifact storage, paid-write admission verification, wallet capture/release handoff, and accounting metering infrastructure only. svc-index is now parked as asset/site manifest pointer, provider lookup, owner-reference metadata, and crab:// navigation support only. The pair blocks fake receipts, fake balances, fake finality, cache-only unlocks, b3-only unlocks, ETag/payment confusion, usage-event balance truth, pointer-based entitlement truth, owner-wallet spend authority, direct wallet/ledger mutation, and all bridge/ROX/Solana/staking/liquidity/external-settlement drift.
```

At this point, the completed parked stabilization set is:

```text id="pygtvu"
1. CrabLink Tauri — COMPLETE / GREEN / PARKED
2. svc-gateway + omnigate — COMPLETE / GREEN / PARKED
3. ron-proto + ron-ledger — COMPLETE / GREEN / PARKED
4. svc-wallet + ron-accounting — COMPLETE / GREEN / PARKED
5. svc-rewarder + ron-policy — COMPLETE / GREEN / PARKED
6. svc-storage + svc-index — COMPLETE / GREEN / PARKED
```

The next logical step is the **final aggregate product-beta readiness park/smoke/docs sweep**, not another crate-pair stabilization pass.


### END NOTE - JULY 1 2026 - 20:50 CST



### BEGIN NOTE - JULY 11 2026 - 13:15 CST

Here is the complete `svc-storage` crate changelog and carryover record for this session.

# `svc-storage` — Phase 10 Moderation Enforcement Changelog Notes

## 1. Crate role

`svc-storage` is the RustyOnions byte-storage and object-serving boundary.

Its responsibilities include:

```text
storing bytes under canonical b3 identifiers
reading full objects
reading byte ranges
returning object metadata
serving legacy HTTP object routes
serving OAP/1 object-fetch responses
verifying stored bytes against their requested b3 identity
enforcing storage-side admission and serving policy
remaining non-authoritative for wallets, ledgers, balances, and rewards
```

During this session, `svc-storage` became the first concrete enforcement point for the shared moderation decisions defined by `ron-policy`.

The central rule implemented was:

```text
A refused or quarantined object must be rejected before storage is accessed.
```

This applies to all currently active object-read surfaces:

```text
OAP/1 OBJ_GET
GET /o/:cid
HEAD /o/:cid
```

The crate does not maintain its own moderation precedence. It consumes the canonical `ron_policy::ModerationPolicy` and enforces the resulting decision.

---

## 2. Phase coverage completed in this crate

The `svc-storage` work spanned several Phase 10 subphases.

### Phase 10B

Integrated moderation into the local OAP object-serving model.

### Phase 10C1

Added bounded moderation-policy file loading and immutable router injection.

### Phase 10D1

Extended the same policy enforcement to legacy HTTP GET and HEAD object reads.

Final status:

```text
Phase 10B: complete
Phase 10C1: complete
Phase 10D1 storage portion: complete
Focused tests: green
Strict Clippy: green
Workspace check: green
```

---

## 3. Primary files added or changed

The main files changed in `svc-storage` were:

```text
crates/svc-storage/src/oap_object.rs
crates/svc-storage/src/moderation_runtime.rs
crates/svc-storage/src/http/server.rs
crates/svc-storage/src/http/routes/oap_object_get.rs
crates/svc-storage/src/http/routes/get_object.rs
crates/svc-storage/src/http/routes/head_object.rs
crates/svc-storage/src/http/routes/legacy_moderation.rs
crates/svc-storage/src/http/routes/mod.rs
crates/svc-storage/src/lib.rs

crates/svc-storage/tests/oap_object_fetch.rs
crates/svc-storage/tests/oap_http_transport.rs
crates/svc-storage/tests/moderation_runtime.rs
crates/svc-storage/tests/legacy_http_moderation.rs
```

Existing regression tests such as `http_blackbox.rs` were also rerun to prove that normal object operations remained functional.

The final crate bundle contains the moderation loader, the shared router extension, the OAP handler, legacy moderation gate, and focused integration tests. 

---

# Part I — OAP moderation enforcement

## 4. `LocalOapObjectService` gained shared moderation state

The local OAP object-serving service was extended to hold:

```rust
Arc<ModerationPolicy>
```

The policy is immutable from the perspective of an individual service instance.

The service gained two construction helpers:

```rust
with_moderation_policy(policy)
with_shared_moderation_policy(Arc<ModerationPolicy>)
```

These support:

```text
owned policy injection in focused tests
shared policy injection from the HTTP router
one immutable snapshot shared across requests
no per-request policy-file parsing
no duplicate moderation state machine
```

The ordinary/default service construction path uses:

```rust
Arc::new(ModerationPolicy::default())
```

An empty default policy permits serving through the canonical `NoRule` decision.

This preserves existing behavior when no moderation policy has been configured.

---

## 5. New OAP moderation errors

The OAP object service gained dedicated moderation error variants.

### `ModerationDenied`

Represents a valid object identity that was refused by the shared policy.

Conceptual shape:

```rust
ModerationDenied {
    reason: ModerationReasonCode,
}
```

This preserves the stable reason code returned by `ron-policy`.

Possible reasons include:

```text
global_deny
owner_tombstone
local_block
quarantined
```

### `ModerationIdentity`

Represents an unexpected inability to convert or reconcile the object identity with the canonical moderation identifier.

This is treated as an internal identity/configuration inconsistency rather than an ordinary policy refusal.

It is distinct from:

```text
invalid client request
missing object
storage not ready
corrupt stored bytes
ordinary policy denial
```

---

## 6. OAP enforcement order

The OAP serving sequence was deliberately ordered so moderation runs before storage.

The effective flow is:

```text
1. Accept one bounded OAP request frame.
2. Decode and validate the OAP request.
3. Apply the existing request-serving policy.
4. Convert the requested object into canonical B3 moderation identity.
5. Evaluate the shared ModerationPolicy.
6. Refuse immediately unless the decision permits serving.
7. Only then query storage.
8. Stream bounded response chunks.
9. Verify the complete returned bytes against the requested BLAKE3 digest.
10. Return the encoded OAP response.
```

The critical new invariant is:

```text
moderation evaluation occurs before exists, head, get_full, or get_range
```

This matters for more than efficiency.

A blocked object should not reveal through backend activity whether:

```text
the bytes exist
the object is cached
the object has a certain size
the storage backend is warm
the storage backend is slow
```

The policy refusal becomes the first meaningful object-specific result.

---

## 7. Interaction with existing request policy

`svc-storage` already had an ordinary object-serving policy boundary.

The moderation check was added after request validation and existing request-policy admission, but before storage access.

The two layers remain distinct:

```text
request policy
    decides whether this class of request is allowed

moderation policy
    decides whether this exact b3 object may be served
```

An ordinary request-policy allowance does not bypass moderation.

Likewise, moderation does not replace:

```text
frame limits
chunk limits
request decoding
storage readiness
not-found handling
full-digest verification
```

---

## 8. OAP HTTP adapter gained policy injection

The OAP HTTP handler now extracts the shared policy from Axum extensions:

```rust
Extension(moderation): Extension<Arc<ModerationPolicy>>
```

It then constructs the local OAP service and injects that shared policy before executing the request.

Conceptually:

```rust
let service = LocalOapObjectService::privacy_aware_local(store)?
    .with_shared_moderation_policy(moderation);
```

This ensures that:

```text
the router owns the policy snapshot
all OAP requests see the same immutable policy
the OAP service does not read environment variables
the OAP service does not open policy files
the handler does not duplicate moderation rules
```

The OAP route remains bounded to the existing one-mebibyte request-frame limit and returns an encoded OAP response only after successful policy and byte verification. 

---

## 9. OAP HTTP status mapping

The HTTP adapter maps moderation outcomes as follows:

```text
ModerationDenied        -> 403 Forbidden
PolicyDenied            -> 403 Forbidden
ModerationIdentity      -> 500 Internal Server Error
NotFound                -> 404 Not Found
NotReady                -> 503 Service Unavailable
Frame or chunk too large -> 413 Payload Too Large
Invalid request/codec   -> 400 Bad Request
```

This distinction is intentional.

A normal moderation refusal is an expected policy result:

```text
403
```

An impossible or inconsistent moderation identity conversion indicates a server-side bug or configuration mismatch:

```text
500
```

No fake success body is returned for refused objects.

---

## 10. OAP tests added or expanded

### `oap_object_fetch.rs`

The focused local OAP suite reached ten tests.

The moderation-related cases prove:

```text
deny policy runs before storage lookup
global deny refuses before storage
owner tombstone refuses before storage
local block refuses before storage
quarantine refuses before storage
local allow permits normal verified serving
local allow cannot override local block
```

The same suite continues proving:

```text
not-found and not-ready remain distinct
stored bytes under the wrong b3 are rejected
corrupt response bytes are rejected by full digest
oversized request frames reject
oversized data chunks reject
valid OBJ_GET round-trips over the real OAP codec
```

Final result:

```text
10 passed
0 failed
```

### `oap_http_transport.rs`

The live-router OAP suite reached five tests.

It proves:

```text
valid object fetch succeeds
missing object returns not found
wrong stored bytes reject
oversized frame rejects
an injected moderation policy returns 403
```

Final result:

```text
5 passed
0 failed
```

---

# Part II — Bounded policy loading

## 11. Added `moderation_runtime.rs`

A new runtime-loading module was added:

```text
crates/svc-storage/src/moderation_runtime.rs
```

This module is responsible only for:

```text
reading one policy file
enforcing a hard file-size bound
strictly deserializing ModerationPolicy
returning an immutable shared policy
returning aggregate entry counts
returning structured load errors
```

It does not:

```text
watch files
reload policy automatically
modify live policy
set node readiness
choose an environment variable
log policy contents
serve HTTP
```

Those responsibilities remain with the embedding runtime, currently `macronode`.

---

## 12. Policy-file size limit

The loader enforces:

```text
MAX_MODERATION_POLICY_BYTES = 256 KiB
```

This bound is checked before accepting the file as policy input.

The purpose is to prevent:

```text
accidental giant configuration files
unbounded memory use
large-file denial-of-service during startup
silent acceptance of unrelated data
expensive parsing of oversized policy documents
```

The current loader is intentionally designed for bounded local policy snapshots, not massive distributed deny databases.

Future scaling can use another validated representation, but it should not silently remove this bound.

---

## 13. Structured load result

A successful load returns a structure equivalent to:

```rust
LoadedModerationPolicy {
    policy: Arc<ModerationPolicy>,
    counts: ModerationPolicyCounts,
}
```

### Shared policy

The loaded policy is immediately wrapped in:

```rust
Arc<ModerationPolicy>
```

This supports immutable sharing across all router handlers.

### Aggregate counts

The load result includes:

```text
global_deny
local_block
local_allow
owner_tombstone
quarantine
```

It also provides a total count.

These counts are used by `macronode` status and logging without exposing exact object identifiers.

---

## 14. Structured loader errors

The loader distinguishes these failure classes:

```text
Metadata
Read
Empty
TooLarge
Json
```

### `Metadata`

The file metadata could not be inspected.

### `Read`

The file could not be read.

### `Empty`

The configured file contains no usable bytes.

### `TooLarge`

The file exceeds the 256 KiB limit.

### `Json`

The JSON or canonical policy contents are invalid.

The JSON category also covers strict `ron-policy` failures such as:

```text
unknown fields
invalid b3 identifiers
wrong field types
noncanonical uppercase hashes
```

The loader does not silently fall back to an empty policy after one of these errors.

---

## 15. Loader behavior versus runtime fail-closed behavior

An important architectural distinction was preserved.

`svc-storage::moderation_runtime` does this:

```text
valid file   -> return loaded policy
invalid file -> return explicit error
```

It does not independently decide node readiness.

The embedding `macronode` runtime later consumes that error and chooses:

```text
configured policy failed to load
    ->
storage readiness stays false
    ->
storage server does not claim successful startup
```

So the bounded loader enables fail-closed behavior, while the runtime host enforces it.

No configured file is different from an invalid configured file:

```text
no configured policy
    -> empty default policy is allowed

configured but invalid policy
    -> explicit failure
```

This prevents a malformed safety configuration from silently disabling moderation.

---

## 16. Loader tests

The `moderation_runtime.rs` integration suite contains four focused tests.

It proves:

```text
valid policy loads
aggregate counts are correct
loaded policy produces the expected decision
empty policy files reject
oversized policy files reject
invalid JSON or invalid canonical policy rejects
```

Final result:

```text
4 passed
0 failed
```

The valid-policy test confirms that a loaded `local_block` entry is actually usable by `ModerationPolicy::evaluate`, not merely counted.

The loader tests and current legacy moderation test are present in the final bundle. 

---

# Part III — Router injection

## 17. Added a router constructor with explicit moderation

The storage HTTP router now has two construction paths.

### Default path

```rust
build_router()
```

This constructs the router using:

```rust
Arc::new(ModerationPolicy::default())
```

It is the correct path when no policy is configured.

### Explicit-policy path

```rust
build_router_with_moderation(Arc<ModerationPolicy>)
```

This installs the provided shared policy into the router.

The router applies:

```rust
.layer(Extension(moderation))
```

so every moderated object-read handler receives the same immutable snapshot.

---

## 18. Why router-level injection was chosen

Router extension injection avoids several unsafe or confusing alternatives.

It prevents:

```text
each request reopening the policy file
each handler reading environment variables
different routes constructing different policies
global mutable policy state
locks held during request handling
policy parsing inside the object-service hot path
```

It also makes tests straightforward:

```rust
build_router_with_moderation(Arc::new(test_policy))
```

This permits realistic live-router tests without modifying process-global environment state.

---

## 19. Immutable snapshot posture

The current router holds one immutable policy snapshot for its lifetime.

This means:

```text
all requests during that runtime use the same policy
policy behavior is deterministic
there is no mid-request policy race
there is no lock-based policy mutation
```

Dynamic reload was deliberately deferred.

A later reload design should replace the whole validated snapshot atomically rather than mutate its internal sets while handlers are using it.

---

# Part IV — Legacy HTTP moderation

## 20. Added `legacy_moderation.rs`

A shared helper module was added for the legacy object-read routes:

```text
src/http/routes/legacy_moderation.rs
```

Its central helper:

```rust
require_serve(...)
```

performs:

```text
canonical B3 identity conversion
shared ModerationPolicy evaluation
403 response construction for refusal
500 response construction for identity inconsistency
```

Both legacy GET and HEAD call the same helper.

This avoids duplicating moderation logic between the two handlers.

---

## 21. Legacy GET enforcement

The `GET /o/:cid` handler was updated to extract:

```rust
Extension<Arc<ModerationPolicy>>
```

The relevant order is now:

```text
1. Parse and validate the requested CID.
2. Apply the shared moderation gate.
3. Refuse immediately when policy does not permit serving.
4. Only then call storage head/read methods.
5. Preserve conditional and range behavior for allowed objects.
```

Moderation therefore runs before:

```text
store.head
store.get_full
store.get_range
```

Allowed GET behavior remains unchanged.

That includes:

```text
normal full-object reads
range responses
ETag behavior
content-length handling
not-found behavior
```

---

## 22. Legacy HEAD enforcement

The `HEAD /o/:cid` handler was updated in the same manner.

The order is:

```text
1. Validate CID.
2. Evaluate moderation.
3. Return 403 when refused.
4. Only then call store.head.
```

This is especially important for HEAD.

Without the gate, a blocked object could still leak:

```text
whether the object exists
its size
its ETag
metadata timing
```

The new rule ensures that a refused HEAD request cannot query storage metadata.

---

## 23. GET and HEAD now share the same policy as OAP

At the end of Phase 10D1, all active object-read routes use the same shared policy snapshot:

```text
OAP OBJ_GET
GET /o/:cid
HEAD /o/:cid
```

The route-specific protocols remain different, but the moderation answer is not.

The serving model is now:

```text
one object
one canonical b3 identity
one shared moderation snapshot
one fixed precedence
all read routes enforce it before storage
```

---

## 24. Legacy pre-storage proof test

A new focused test was added:

```text
tests/legacy_http_moderation.rs
```

It defines a `TrackingStorage` implementation where every storage operation increments a counter.

The test:

```text
creates one local-block policy
injects it into the real router
sends GET /o/<blocked-cid>
expects 403
asserts storage call count is zero
sends HEAD /o/<blocked-cid>
expects 403
asserts storage call count is still zero
```

This is stronger than checking only the HTTP result.

It proves the policy decision occurred before every storage method:

```text
put
exists
head
get_full
get_range
```

The focused result was:

```text
1 passed
0 failed
```

The final code bundle shows the `TrackingStorage` implementation and the zero-call assertions for blocked GET and HEAD. 

---

# Part V — Module export repair

## 25. Compiler failure discovered during Phase 10D1

After adding `legacy_moderation.rs`, the first focused compile failed with:

```text
cannot find legacy_moderation in super
```

The route module appeared to be exported correctly from:

```text
src/http/routes/mod.rs
```

but the crate had a second, inline module declaration tree inside:

```text
src/lib.rs
```

That inline tree manually declared modules under:

```rust
pub mod http {
    pub mod routes {
        ...
    }
}
```

Because the crate was using that inline declaration tree, adding the module only to `src/http/routes/mod.rs` was insufficient.

---

## 26. Narrow module export fix

The immediate repair added:

```rust
pub mod legacy_moderation;
```

to the inline `http::routes` declaration inside `src/lib.rs`.

After that repair:

```text
cargo check -p svc-storage
```

passed, and:

```text
cargo test -p svc-storage --test legacy_http_moderation
```

passed.

This was intentionally a narrow compiler fix.

---

## 27. Known module-structure technical debt

`svc-storage` currently has duplicate-looking module fanout surfaces:

```text
src/http/mod.rs
src/http/routes/mod.rs
```

and an inline nested module declaration in:

```text
src/lib.rs
```

The inline tree can bypass or duplicate the normal filesystem module roots.

This creates risks such as:

```text
a module added to routes/mod.rs but not lib.rs
confusing compiler resolution
duplicate maintenance
future route-export drift
```

We deliberately did not refactor the entire module tree during Phase 10D1.

That broader cleanup should occur only in a focused slice with regression checks.

A likely eventual cleanup is:

```rust
pub mod http;
```

in `lib.rs`, allowing `src/http/mod.rs` and `src/http/routes/mod.rs` to become the single normal module fanout.

That refactor is not yet complete and should not be mixed into moderation behavior changes without tests.

---

# Part VI — Regression preservation

## 28. Existing HTTP behavior remained intact

The general storage black-box test was rerun:

```text
http_blackbox
```

It proved that normal storage HTTP behavior still works after router policy injection.

Final result:

```text
1 passed
0 failed
```

The covered flow includes:

```text
put
head
get
range
```

This confirms the empty/default moderation policy does not accidentally block ordinary storage usage.

---

## 29. Allowed objects remain serveable

The tests prove two different allowed postures.

### No matching rule

An object not present in any moderation set continues through the normal storage path.

### Explicit local allow

An object present only in `local_allow` is also served normally.

The implementation therefore does not confuse “moderation enabled” with “deny everything.”

The gate is decision-based:

```text
Serve       -> proceed
Refuse      -> 403
Quarantine  -> 403
```

---

## 30. Stronger states remain authoritative

A local allow cannot bypass:

```text
global deny
owner tombstone
local block
quarantine
```

This is primarily guaranteed by `ron-policy`, but the `svc-storage` tests prove the serving layer respects the returned decision.

`svc-storage` does not reinterpret:

```text
LocalAllow
```

as unconditional access.

It relies on:

```rust
decision.permits_serve()
```

after canonical precedence has already been applied.

---

# Part VII — Security and privacy properties

## 31. Refusal before object existence disclosure

Because moderation occurs before storage lookup, a blocked request does not directly reveal whether the object is present.

The externally visible result is:

```text
403 Forbidden
```

rather than:

```text
404 when absent
200 when present
different metadata depending on backend state
```

This creates a cleaner and less revealing policy boundary.

---

## 32. No exact-ID exposure added to status

`svc-storage` provides aggregate count support through the loader result, but it does not expose a route that dumps the policy.

The runtime integration reports only counts such as:

```text
local_block = 1
quarantine = 0
```

It does not report:

```text
the policy file path
the exact blocked b3 identifiers
the contents of local allowlists
the contents of global deny lists
```

---

## 33. No residential IP or provider details added

The moderation additions do not add:

```text
peer IP addresses
provider IP addresses
residential addresses
node routing descriptors
DHT provider data
```

to storage responses.

The OAP route continues using byte/object DTOs without economic or provider identity claims.

---

## 34. No economic authority added

The moderation work does not allow `svc-storage` to:

```text
issue ROC
award ROC
deny confirmed ROC directly
modify a wallet
modify the ledger
claim a payout
create a reward receipt
burn tokens
mint tokens
```

Later accounting may treat successful or refused service evidence differently, but that authority remains outside `svc-storage`.

The crate still owns:

```text
bytes
byte integrity
serve admission
read behavior
storage-local evidence inputs
```

It does not own durable economic truth.

---

# Part VIII — What was not changed

## 35. Write routes are not yet governed by this moderation gate

This session applied moderation to object reads:

```text
OAP OBJ_GET
legacy GET
legacy HEAD
```

It did not add the same exact-object moderation gate to:

```text
PUT
POST
paid object writes
paid estimates
settlement-related write admission
```

That is not necessarily a bug: serving refusal and write admission are separate concerns.

Future work must deliberately decide whether denied or tombstoned objects should also be rejected at ingest time.

That should not be assumed implicitly.

---

## 36. Physical bytes are not deleted

A moderation refusal currently means:

```text
do not serve this object
```

It does not mean:

```text
the object has been deleted
```

Blocked, denied, tombstoned, or quarantined bytes may still physically exist in the storage backend.

This distinction must remain explicit:

```text
declarative refusal != physical pruning
```

---

## 37. The `Storage` trait still lacks delete capability

The current shared storage trait exposes operations equivalent to:

```rust
put
exists
head
get_full
get_range
```

It does not yet expose:

```rust
delete
remove
prune
```

Therefore physical pruning cannot be honestly implemented merely by adding a CLI command.

A later pruning slice must:

```text
add a delete-capable trait operation
implement it for applicable storage backends
define idempotent not-found behavior
test deletion
test concurrent reads
test cache cleanup
test metadata cleanup
test failure recovery
```

---

## 38. Provider-record withdrawal is not implemented

Removing local bytes is not sufficient if the node has advertised itself as a provider.

A complete prune operation will eventually need to coordinate:

```text
storage deletion
index cleanup
cache cleanup
provider-record withdrawal
operator/audit result
```

The current DHT/provider machinery is not yet fully active in `macronode`.

`svc-storage` must not claim provider withdrawal until the DHT crate and host integration implement and test it.

---

## 39. No dynamic policy reload

The current storage router uses one immutable snapshot.

Changing the underlying file does not automatically update a running router.

Future reload work should specify:

```text
validation before activation
atomic snapshot replacement
failure behavior
status generation or epoch
concurrent request behavior
rollback behavior
```

No partial mutable reload should be introduced.

---

## 40. No signed-policy verification

`svc-storage` trusts the already-constructed `ModerationPolicy` supplied by the runtime loader.

It does not determine whether:

```text
a global list was signed
a signer is trusted
a policy epoch is current
a tombstone has valid ownership proof
```

Those validations belong upstream.

Only validated policy state should be injected into the router.

---

# Part IX — Validation completed

## 41. Focused tests confirmed green

The following relevant test targets passed during this session:

```text
cargo test -p svc-storage --test moderation_runtime
    4 passed

cargo test -p svc-storage --test oap_object_fetch
    10 passed

cargo test -p svc-storage --test oap_http_transport
    5 passed

cargo test -p svc-storage --test legacy_http_moderation
    1 passed

cargo test -p svc-storage --test http_blackbox
    1 passed
```

---

## 42. Compile and lint gates confirmed green

The following gates passed:

```text
cargo check -p svc-storage

cargo clippy \
  -p svc-storage \
  -p macronode \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo check --workspace
```

The combined Clippy run was green after all Phase 10D1 behavior and test corrections.

The workspace check was also green.

---

# Part X — Current invariants

## 43. Moderation-before-storage invariant

For every currently moderated object-read path:

```text
validate identity
    ->
evaluate shared moderation policy
    ->
refuse unless Serve
    ->
access storage only after permission
```

This ordering must not regress.

---

## 44. One shared policy snapshot

Within one router runtime:

```text
OAP GET
legacy GET
legacy HEAD
```

all use the same:

```rust
Arc<ModerationPolicy>
```

No route should construct a different moderation policy independently.

---

## 45. Empty-default compatibility

When no policy is configured:

```text
build_router()
```

uses an empty policy.

Normal allowed object serving remains functional.

This is different from a configured policy failing to load, which must be surfaced as an error by the host runtime.

---

## 46. Full BLAKE3 verification remains mandatory

Moderation does not replace byte verification.

For an allowed OAP request:

```text
the complete served bytes must still hash to the requested b3 identifier
```

Stored bytes under the wrong identifier remain a hard failure.

---

## 47. Refusal does not imply pruning

Status and future operator output must distinguish:

```text
refused from serving
physically deleted
provider record withdrawn
```

Only the first exists today.

---

## 48. Storage remains economically non-authoritative

Even though policy refusals may later affect reward eligibility, `svc-storage` must not calculate or finalize ROC itself.

The durable reward path remains elsewhere.

---

# Part XI — Recommended future work

## 49. Near-term Phase 10 work

The next storage-adjacent Phase 10 slices should likely include:

```text
low-cardinality moderation refusal counters
reason-code metrics without CID labels
operator mutation workflow integration
signed policy snapshot validation upstream
delete-capable storage trait design
physical prune implementation
cache and metadata cleanup
provider withdrawal coordination
prune result tests
failure and retry tests
```

---

## 50. Refusal observability requirements

Future counters should use bounded labels such as:

```text
route = oap | legacy_get | legacy_head
reason = global_deny | owner_tombstone | local_block | quarantined
```

They should not use:

```text
cid as a metric label
policy path as a metric label
user identity as a metric label
raw IP as a metric label
```

That avoids high-cardinality and privacy problems.

---

## 51. Physical prune requirements

A truthful physical prune should eventually prove:

```text
the exact canonical b3 target was selected
the target was refused before deletion
local bytes were removed
cache copies were removed or invalidated
metadata/index references were handled
provider advertising was withdrawn when active
repeated prune is idempotent
unrelated objects remain untouched
partial failure is reported honestly
no reward or ledger mutation occurs
```

---

# Part XII — Final crate status

At the end of this session, `svc-storage` has moved from OAP-only object policy enforcement to one shared moderation boundary across every active object-read route.

The completed behavior is:

```text
shared immutable ModerationPolicy injection
bounded 256 KiB policy-file loading
strict invalid-policy rejection
aggregate policy counts
OAP moderation before storage lookup
legacy GET moderation before storage lookup
legacy HEAD moderation before storage lookup
403 for expected refusal
500 for internal moderation identity inconsistency
normal allowed reads preserved
full BLAKE3 verification preserved
focused pre-storage proof tests
strict Clippy green
workspace check green
```

The central completed invariant is:

```text
A refused or quarantined object is never looked up or served through
OAP OBJ_GET, legacy GET, or legacy HEAD.
```

The central remaining limitation is:

```text
The object may still physically exist because pruning and provider
withdrawal have not yet been implemented.
```

`svc-storage` now correctly enforces declarative moderation at the byte-serving boundary without claiming deletion, provider withdrawal, reward authority, wallet mutation, ledger mutation, or economic finality.

The final crate-specific notes in this sequence are for `macronode`, including runtime policy loading, fail-closed readiness, truthful status, live route tests, and the updated Phase 10D1 status contract.


### END NOTE - JULY 11 2026 - 13:15 CST


### BEGIN NOTE - JULY 11 2026 - 22:50 CST

Here is the complete crate-specific changelog record for the `svc-storage` work completed during BUILD_PLAN_Z Phase 10.

# `svc-storage` — BUILD_PLAN_Z Phase 10 Changelog Notes

## 1. Crate role

`svc-storage` is the RustyOnions byte-storage and object-serving boundary.

Its responsibilities include:

```text
accepting and storing object bytes
serving content-addressed objects
handling GET, HEAD, range, and OAP object reads
reporting storage readiness
performing BLAKE3 integrity verification
returning storage-specific errors
providing the storage handle embedded by macronode
```

During BUILD_PLAN_Z Phase 10, `svc-storage` gained two major capabilities:

```text
1. canonical moderation enforcement before object reads
2. exact local byte pruning through the shared Storage trait
```

The resulting rule is:

```text
No moderated object read may reach storage unless the canonical
ron-policy decision permits serving.
```

The crate does not maintain its own moderation state machine or precedence model. It consumes `ron_policy::ModerationPolicy` and enforces the resulting decision at the byte-access boundary. 

---

## 2. Phase coverage completed in this crate

The `svc-storage` changes spanned several BUILD_PLAN_Z Phase 10 subphases.

### Phase 10B

Integrated canonical moderation into the OAP object-serving path.

### Phase 10C1

Added bounded policy-file loading and immutable policy injection into the storage router.

### Phase 10D1

Extended the same policy enforcement to legacy HTTP object reads:

```text
GET /o/:cid
HEAD /o/:cid
```

### Phase 10F1

Added the exact local storage-prune primitive used by macronode’s real prune coordinator.

Final status:

```text
Phase 10B: complete
Phase 10C1: complete
Phase 10D1 storage portion: complete
Phase 10F1: complete

OAP moderation enforcement: green
bounded loader: green
legacy GET/HEAD enforcement: green
exact local pruning: green
strict Clippy checkpoint: green
workspace compilation checkpoint: green
```

The BUILD_PLAN_Z Phase 10 acceptance criteria specifically required denied objects not to be served and pruning to remove local bytes. Those storage-side requirements are now implemented. 

---

## 3. Primary files added or changed

The principal files changed in `svc-storage` were:

```text
crates/svc-storage/src/oap_object.rs
crates/svc-storage/src/moderation_runtime.rs
crates/svc-storage/src/http/server.rs
crates/svc-storage/src/http/routes/oap_object_get.rs
crates/svc-storage/src/http/routes/get_object.rs
crates/svc-storage/src/http/routes/head_object.rs
crates/svc-storage/src/http/routes/legacy_moderation.rs
crates/svc-storage/src/http/routes/mod.rs
crates/svc-storage/src/storage/mod.rs
crates/svc-storage/src/lib.rs

crates/svc-storage/tests/oap_object_fetch.rs
crates/svc-storage/tests/oap_http_transport.rs
crates/svc-storage/tests/moderation_runtime.rs
crates/svc-storage/tests/legacy_http_moderation.rs
```

Existing storage and HTTP regression tests were also rerun during the integration checkpoints.

The crate bundle confirms the addition of the moderation runtime module, shared router policy injection, OAP moderation path, legacy moderation helper, and focused moderation tests. 

---

# Part I — OAP Serve-Time Moderation

## 4. `LocalOapObjectService` gained shared moderation state

The local OAP object-serving service was extended to carry:

```rust
Arc<ModerationPolicy>
```

This policy represents one immutable moderation snapshot used throughout the service instance.

Construction paths were added for:

```text
owned policy injection for focused tests
shared Arc policy injection from the runtime/router
default empty policy for unconfigured operation
```

Conceptually, the constructors support behavior equivalent to:

```rust
with_moderation_policy(policy)
with_shared_moderation_policy(Arc<ModerationPolicy>)
```

The ordinary unconfigured path uses:

```rust
Arc::new(ModerationPolicy::default())
```

An empty policy produces the canonical `no_rule` decision and preserves the existing serve behavior.

### Why an `Arc` snapshot was used

The shared immutable policy ensures:

```text
no policy-file read on each request
no per-request JSON parsing
no mutable policy race during a request
one consistent snapshot across all routes
cheap cloning into Axum handlers
no duplicate policy state machine
```

Hot reload was deliberately not introduced here. Runtime activation and replacement remain a macronode concern.

---

## 5. Canonical moderation identity

Before looking up bytes, the OAP service reconciles the requested object with the canonical:

```text
ron_policy::B3Id
```

The accepted identity is:

```text
b3:<64 lowercase hexadecimal characters>
```

The storage service does not invent a looser moderation identifier.

This guarantees that the same exact identity is used by:

```text
ron-policy
svc-storage
macronode
crabnode
pruning
provider withdrawal
index-cache invalidation
```

Near-identical hashes remain separate objects.

---

## 6. OAP enforcement order

The OAP object-read path now follows this order:

```text
validate OAP request
  ->
apply ordinary OAP request policy
  ->
parse or reconcile canonical moderation identity
  ->
evaluate the shared ModerationPolicy
  ->
refuse unless the decision permits serving
  ->
only then call storage
  ->
stream bounded object bytes
  ->
verify the complete BLAKE3 digest
```

The critical security property is:

```text
moderation happens before storage access
```

For a denied object, the service must not first:

```text
check whether the object exists
read metadata
read its size
read its bytes
read a range
calculate an ETag
begin streaming
```

This avoids turning storage behavior into an existence or metadata oracle for blocked content.

---

## 7. New OAP moderation errors

The OAP service gained distinct moderation-related errors.

### Moderation denial

A valid object identity can be refused by policy.

The denial retains the canonical reason from `ron-policy`, such as:

```text
global_deny
owner_tombstone
local_block
quarantined
```

This is an expected policy outcome and maps to:

```text
403 Forbidden
```

### Moderation identity inconsistency

A separate error represents an internal inability to reconcile an otherwise accepted request with the canonical moderation identity.

This is not treated as:

```text
ordinary not found
ordinary policy denial
bad stored bytes
storage not ready
```

It maps to an internal server error rather than silently bypassing moderation.

### Existing errors remain distinct

The new moderation errors did not collapse the pre-existing taxonomy for:

```text
invalid request
not found
not ready
wrong object identity
corrupt bytes
oversized frame
oversized chunk
ordinary OAP policy rejection
```

This preserves useful diagnostics and protocol behavior.

---

## 8. OAP refusal before storage

Focused tests use controlled storage implementations to prove that denied objects are rejected before byte access.

The OAP moderation coverage proves:

```text
global deny refuses before storage
owner tombstone refuses before storage
local block refuses before storage
quarantine refuses before storage
local allow permits serving
local allow cannot bypass stronger refusal state
near-identical B3 IDs are not falsely matched
normal allowed object fetch still works
wrong bytes remain rejected
full BLAKE3 verification remains mandatory
frame and chunk limits remain enforced
```

Focused result recorded during the session:

```text
oap_object_fetch:
  10 passed
  0 failed
```

The moderation work therefore strengthened the existing OAP path without weakening the Phase 9 integrity and size-limit guarantees. 

---

# Part II — Bounded Moderation Policy Loading

## 9. New `moderation_runtime` module

A new module was added:

```text
crates/svc-storage/src/moderation_runtime.rs
```

This module provides a strict, bounded loader for one moderation-policy JSON snapshot.

It owns:

```text
MAX_MODERATION_POLICY_BYTES
ModerationPolicyCounts
LoadedModerationPolicy
ModerationPolicyLoadError
load_moderation_policy_file
```

It does not own:

```text
environment-variable selection
signed-policy verification
trusted signer configuration
epoch persistence
runtime status
hot reload
policy distribution
operator mutation commands
```

Those belong to macronode, `ron-policy`, and operator tooling.

---

## 10. Policy-file size limit

The moderation policy file is bounded to:

```text
256 KiB
```

This prevents a configured policy path from causing unbounded memory use or unexpectedly large startup parsing work.

The loader checks the size:

```text
before reading, using filesystem metadata
after reading, using the actual byte length
```

Checking both avoids a replacement or growth race between metadata inspection and file reading.

The loader does not stream or accept an arbitrarily large JSON policy.

---

## 11. Structured loader errors

The loader provides explicit failure categories:

```text
Metadata
Read
Empty
TooLarge
Json
```

### `Metadata`

The file could not be inspected.

Examples:

```text
missing path
permission failure during metadata access
filesystem error
```

### `Read`

Metadata succeeded, but reading the file failed.

### `Empty`

The file contains no meaningful data or only ASCII whitespace.

An explicitly configured empty policy file is treated as an error rather than silently meaning “no policy.”

### `TooLarge`

The file exceeds the 256 KiB cap either before or after reading.

### `Json`

The file cannot be decoded into the strict canonical `ModerationPolicy`.

This includes:

```text
malformed JSON
unknown fields
invalid exact B3 identifiers
wrong field types
other canonical policy-deserialization failures
```

The loader does not normalize or repair invalid policies.

---

## 12. Successful load result

A successful load returns:

```text
Arc<ModerationPolicy>
ModerationPolicyCounts
```

The shared `Arc` is ready for direct injection into the storage router.

The counts include:

```text
global_deny
local_block
local_allow
owner_tombstone
quarantine
total
```

The counts are derived from the same immutable policy snapshot that is injected into serving routes.

This prevents status information from being calculated from a different policy object than the one actually used for enforcement.

The loader and its count structure are visible in the final crate source. 

---

## 13. Loader tests

The focused loader suite proves behavior including:

```text
valid policy loads
aggregate counts match the loaded sets
loaded policy produces the expected decision
empty policy file rejects
oversized policy file rejects
invalid JSON or invalid canonical policy rejects
```

Focused result:

```text
moderation_runtime:
  4 passed
  0 failed
```

---

# Part III — Router Injection

## 14. Two router construction paths

The storage HTTP server now exposes:

```rust
build_router()
```

and:

```rust
build_router_with_moderation(
    Arc<ModerationPolicy>
)
```

### Default router

`build_router()` preserves unconfigured behavior by constructing an empty canonical policy.

### Explicit moderation router

`build_router_with_moderation(...)` injects the exact shared policy provided by the runtime.

This allows macronode to:

```text
load and verify policy once
compose signed global and local policy once
inject the final immutable snapshot once
start the storage listener only with that snapshot
```

`svc-storage` does not need to know whether the snapshot came from:

```text
a local operator file
a signed global policy
a composition of both
a focused test fixture
```

It simply enforces the supplied canonical policy.

---

## 15. Axum `Extension` injection

The shared policy is inserted into the router through Axum extension state.

Moderated handlers extract:

```text
Arc<ModerationPolicy>
```

from the request’s router state.

Benefits include:

```text
one policy snapshot for OAP, GET, and HEAD
cheap Arc cloning
no global mutable singleton
no filesystem work in handlers
no lock held across request awaits
straightforward focused tests
```

The OAP transport integration suite confirmed that the router-provided policy reaches the live HTTP OAP handler.

Focused result:

```text
oap_http_transport:
  5 passed
  0 failed
```

---

# Part IV — Legacy GET and HEAD Enforcement

## 16. Why additional work was needed

After the OAP path was moderated, two legacy object-read routes still existed:

```text
GET /o/:cid
HEAD /o/:cid
```

Leaving those routes unmoderated would have created a direct bypass:

```text
OAP request:
  denied

legacy GET:
  still serves the object
```

Phase 10D1 closed that bypass.

---

## 17. New `legacy_moderation` helper

A new route helper was added:

```text
crates/svc-storage/src/http/routes/legacy_moderation.rs
```

It centralizes the shared behavior for legacy object reads.

The helper performs:

```text
canonical B3 parsing
shared policy evaluation
stable refusal mapping
permission result returned to GET/HEAD handlers
```

This avoids independently duplicating the policy check inside both handlers.

---

## 18. Legacy GET order

The GET object handler now follows:

```text
extract CID
  ->
validate canonical B3 identity
  ->
evaluate shared moderation policy
  ->
return 403 if refused
  ->
only then inspect or read storage
  ->
preserve existing range/full-read behavior
```

A blocked GET does not call:

```text
head
get_full
get_range
exists
```

---

## 19. Legacy HEAD order

The HEAD handler now follows the same policy gate before metadata access:

```text
extract CID
  ->
validate canonical B3 identity
  ->
evaluate shared moderation policy
  ->
return 403 if refused
  ->
only then read object metadata
```

This matters because an unmoderated HEAD response could disclose:

```text
whether the object exists
object length
ETag
range posture
storage metadata
```

The handler now refuses before those values are calculated.

---

## 20. Dedicated zero-storage-call test

A focused integration test introduced a tracking storage implementation that increments a counter whenever any storage method is called.

The test proves:

```text
blocked GET:
  HTTP 403
  storage calls = 0

blocked HEAD:
  HTTP 403
  storage calls = 0
```

This is stronger than merely checking the returned status.

It proves the moderation check occurs before the physical storage boundary. The test implementation and zero-call assertions are present in the crate bundle. 

Focused result:

```text
legacy_http_moderation:
  1 passed
  0 failed
```

---

## 21. Full object-read coverage

At the end of Phase 10D1, these read surfaces share the same moderation snapshot:

```text
OAP OBJ_GET
GET /o/:cid
HEAD /o/:cid
```

The truthful runtime posture became:

```text
all object-read policy enforcement active
```

This does not claim that every storage operation is moderated.

It specifically does not imply:

```text
write admission policy is complete
persistence approval is complete
pin/unpin policy is complete
repair policy is complete
physical pruning happens automatically on every deny
network-wide deletion exists
```

Those remain separate behaviors and later phases.

---

# Part V — Exact Local Byte Pruning

## 22. New `PruneOutcome`

Phase 10F1 added a canonical result type to the storage layer:

```rust
pub enum PruneOutcome {
    Removed { bytes: u64 },
    NotFound,
}
```

The outcomes are intentionally truthful.

### `Removed { bytes }`

The exact object existed and was removed from the local storage backend.

The result reports the number of local object bytes actually removed.

### `NotFound`

The exact object was already absent.

This is a successful idempotent no-op, not an error and not fake deletion.

The storage layer does not claim removal when no object existed.

---

## 23. `Storage::prune`

The shared storage trait gained an exact object-prune operation equivalent to:

```rust
async fn prune(
    &self,
    object: &B3Id,
) -> Result<PruneOutcome>
```

Using `B3Id` rather than an arbitrary string ensures that a prune request has already passed the canonical exact-object identity rules.

This method is the local-byte component consumed by macronode’s higher-level prune coordinator.

The storage trait remains unaware of:

```text
DHT provider records
svc-index provider cache
network propagation
operator HTTP authorization
wallet state
ledger state
reward state
```

It removes local bytes only.

---

## 24. `MemoryStorage` exact deletion

The active in-memory storage implementation gained a real exact-key deletion path.

The implementation:

```text
acquires one write lock
removes exactly object.as_str()
returns Removed with the removed byte length
returns NotFound when the exact key is absent
releases the lock before returning
```

The one-write-lock approach avoids a check-then-delete pattern such as:

```text
read lock to check existence
drop read lock
write lock to remove
```

Such a two-step pattern could race and report inaccurate results.

The final behavior is atomic with respect to the in-memory map:

```text
remove and determine outcome from the same operation
```

---

## 25. Prune exactness

The local prune operation does not use:

```text
prefix matching
substring matching
hash abbreviation
case normalization
directory sweeping
policy-set iteration
manifest lookup
alias expansion
```

It removes only the exact canonical B3 key.

Neighboring objects remain present.

For example:

```text
target:
  b3:aaaa...aaaa

neighbor:
  b3:aaaa...aaab
```

Pruning the target does not remove the neighbor.

---

## 26. Idempotency

Repeated pruning behaves as:

```text
first call:
  Removed { bytes: N }

second call:
  NotFound

third call:
  NotFound
```

The storage layer does not turn repeated no-op calls into:

```text
false Removed result
generic internal failure
fake changed=true
```

This behavior is required so the higher-level macronode coordinator can report:

```text
pruned
already_absent
partial
failed
```

truthfully.

---

## 27. Byte-count reporting

The byte count comes from the removed `Bytes` value itself.

This means:

```text
Removed { bytes }
```

represents local object payload bytes removed from the storage map.

It does not represent:

```text
filesystem allocation blocks
index-cache memory
DHT record size
manifest size
replication savings
network-wide reclaimed storage
```

The result is deliberately narrow.

---

## 28. Prune boundary

`svc-storage` pruning performs only:

```text
exact local object-byte deletion
```

It does not:

```text
withdraw a DHT provider
invalidate svc-index provider cache
remove resolve-cache entries
remove manifest pointers
notify other nodes
propagate a tombstone
delete remote replicas
change moderation policy
issue a reward penalty
mutate a wallet
mutate the ledger
alter confirmed ROC
```

Those independent steps belong to other crates and are coordinated by macronode.

The macronode prune service explicitly consumes `Storage::prune` as one independent result rather than treating it as complete network deletion. 

---

## 29. Runtime handle reuse

The embedded macronode storage worker registers the exact `DynStorage` instance used by the live storage router with the prune coordinator.

This is important because pruning a separate or newly constructed storage instance would produce false success while the live serving router retained the bytes.

The runtime relationship is:

```text
one MemoryStorage instance
  ->
embedded svc-storage router
  ->
registered macronode prune handle
```

Therefore, the bytes removed by `crabnode prune` are the same bytes previously available through the live storage service.

The Phase 10 live prune exit test later proved:

```text
seeded object returns 200 before prune
crabnode prune removes it
same storage endpoint returns 404 afterward
repeated prune reports already_absent
```

That end-to-end test lives in macronode, but its physical deletion behavior depends on this `svc-storage` implementation.

---

# Part VI — Error and Status Behavior

## 30. HTTP refusal mapping

Policy-denied object reads map to:

```text
403 Forbidden
```

This distinguishes moderation refusal from:

```text
400 invalid object/request
404 object not found
416 invalid range
500 internal moderation identity error
503 storage not ready
```

The refusal does not pretend the object is absent, and the not-found response does not pretend policy denied it.

---

## 31. No policy details leaked through storage responses

The storage layer uses stable reason behavior internally but does not expose:

```text
entire policy files
all blocked object IDs
policy paths
trusted signer keys
accepted epochs
operator configuration
```

Higher-level status reporting exposes aggregate policy posture through macronode.

`svc-storage` remains focused on whether the current exact read is permitted.

---

## 32. No implicit deletion on deny

A policy denial does not automatically invoke `Storage::prune`.

The two operations are distinct:

```text
deny:
  prevent serving

prune:
  physically remove local bytes
```

This distinction is important because operators may need:

```text
temporary quarantine
review
appeal
evidence retention
audit handling
delayed physical removal
```

Only an explicit prune operation removes bytes.

---

# Part VII — Regression Repairs During Integration

## 33. Duplicate route-module declarations

The crate contained both:

```text
src/http/routes/mod.rs
```

and a manually declared inline route-module tree in:

```text
src/lib.rs
```

When `legacy_moderation` was initially added only to `routes/mod.rs`, compilation failed because the library’s active inline module tree did not export it.

Failure shape:

```text
cannot find legacy_moderation in super
```

The narrow fix was to add:

```rust
pub mod legacy_moderation;
```

to the inline route declarations as well.

This was a module-export mismatch, not a moderation-algorithm defect. 

---

## 34. Existing test storage implementors

Adding a new shared storage operation required ensuring trait implementations and focused test doubles remained compatible.

The real runtime `MemoryStorage` received the exact deletion implementation.

Test-only tracking or mock storage types remained focused on their intended behavior and were adjusted where necessary during compile repair.

The first compile failure was fixed before expanding scope, following the QuickChain-style implementation posture.

---

## 35. Existing read behavior preserved

The moderation work was inserted before storage access but did not replace the existing successful read behavior.

Allowed objects still preserve:

```text
full-object GET
HEAD metadata
range reads
ETag behavior
OAP chunking
full digest verification
not-found behavior
readiness behavior
```

The empty default policy preserves ordinary operation where moderation is not configured.

---

# Part VIII — Tests and Verification

## 36. Focused OAP object tests

Command family:

```bash
cargo test -p svc-storage \
  --test oap_object_fetch \
  -- --nocapture
```

Recorded focused result:

```text
10 passed
0 failed
```

Coverage includes moderation precedence and existing B3 integrity behavior.

---

## 37. Moderation loader tests

Command family:

```bash
cargo test -p svc-storage \
  --test moderation_runtime \
  -- --nocapture
```

Recorded focused result:

```text
4 passed
0 failed
```

Coverage includes valid load, counts, empty input, oversized input, and malformed policy rejection.

---

## 38. Live OAP HTTP transport tests

Command family:

```bash
cargo test -p svc-storage \
  --test oap_http_transport \
  -- --nocapture
```

Recorded result:

```text
5 passed
0 failed
```

This proved that router-injected policy reaches the live OAP handler.

---

## 39. Legacy HTTP moderation test

Command family:

```bash
cargo test -p svc-storage \
  --test legacy_http_moderation \
  -- --nocapture
```

Recorded result:

```text
1 passed
0 failed
```

The test proves both GET and HEAD return 403 with zero storage calls.

---

## 40. Exact prune tests

Focused storage tests proved:

```text
exact target is removed
removed byte count is returned
missing target reports NotFound
repeated prune is idempotent
neighboring object remains
```

The resulting primitive was then exercised through macronode coordinator tests, handler tests, CLI tests, and the live CLI-to-storage test.

---

## 41. Quality gates

Relevant crate and integration checkpoints included:

```bash
cargo fmt -p svc-storage

cargo test -p svc-storage --test oap_object_fetch

cargo test -p svc-storage --test moderation_runtime

cargo test -p svc-storage --test oap_http_transport

cargo test -p svc-storage --test legacy_http_moderation

cargo check -p svc-storage

cargo clippy \
  -p svc-storage \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo check --workspace
```

Focused behavior was made green before downstream macronode work continued.

---

# Part IX — Downstream Integration

## 42. `ron-policy`

`svc-storage` consumes the canonical:

```text
B3Id
ModerationPolicy
ModerationDecision
ModerationReasonCode
```

It does not recreate:

```text
global/local precedence
reason strings
exact identifier parsing
signed-global/local composition
```

---

## 43. `macronode`

Macronode owns:

```text
policy path selection
signed snapshot verification
trusted signer configuration
global/local policy composition
rollback epoch persistence
fail-closed startup behavior
runtime status
registration of the live storage handle
multi-surface prune coordination
admin HTTP endpoint
```

It then injects the final immutable policy into `svc-storage`.

For pruning, macronode calls:

```text
Storage::prune
```

and reports its result separately from:

```text
DHT provider withdrawal
svc-index provider-cache invalidation
```

---

## 44. `crabnode`

The `crabnode prune` command validates the canonical B3 ID and calls macronode.

It does not contact `svc-storage` directly.

The control chain is:

```text
crabnode
  ->
macronode protected moderation endpoint
  ->
PruneCoordinator
  ->
registered live DynStorage
  ->
Storage::prune
```

This keeps authentication, operator semantics, and multi-step reporting outside the byte-storage crate.

---

# Part X — Authority and Security Boundaries

## 45. What `svc-storage` now owns

After Phase 10, `svc-storage` owns:

```text
moderation enforcement before object byte reads
one immutable policy snapshot per router
bounded local policy-file decoding helper
policy-entry counts derived from the loaded snapshot
OAP refusal mapping
legacy GET refusal mapping
legacy HEAD refusal mapping
zero-storage-call guarantee for denied reads
exact local object-byte pruning
truthful Removed/NotFound storage outcomes
removed payload-byte reporting
```

---

## 46. What `svc-storage` does not own

It still does not own:

```text
canonical moderation precedence
signed policy verification
trusted signer selection
global policy distribution
local operator policy editing
policy hot reload
runtime accepted-epoch persistence
DHT provider withdrawal
provider advertisement
svc-index cache invalidation
resolve-cache deletion
manifest-pointer deletion
network-wide content deletion
moderation quorum
reward eligibility
reward planning
wallet mutation
ledger mutation
confirmed ROC
ROX minting or burning
bridge settlement
staking
liquidity
exchange behavior
```

---

## 47. No economic authority introduced

The moderation and prune work does not convert `svc-storage` into an economic authority.

It does not:

```text
award ROC for a successful read
penalize a balance for denied content
finalize a service reward
create a wallet receipt
create a ledger receipt
treat stored bytes as payment proof
treat a policy decision as payout truth
```

Later evidence and reward phases may consume storage-related facts, but only the canonical accounting, policy, wallet, ledger, and quorum paths can create confirmed economic outcomes.

---

## 48. No network-wide deletion claim

`Storage::prune` means:

```text
this exact object was removed from this exact local storage instance
```

It does not mean:

```text
the object disappeared from every service node
all caches were cleared
all providers withdrew
the object can never be republished
the global network accepted a tombstone
```

This narrow wording is preserved through the macronode prune report and CLI output.

---

# Part XI — Future Work

## 49. Phase 11 persistence state machine

BUILD_PLAN_Z Phase 11 will extend `svc-storage` with a persistence-policy state machine such as:

```text
ephemeral_unvetted
pending_review
verified_persistent
operator_blocked
global_denied
owner_tombstoned
quarantined
pinned_by_operator
```

Phase 10 does not implement this state machine.

Current Phase 10 behavior answers:

```text
may this exact object be read?
can this exact local object be physically pruned?
```

Phase 11 will answer:

```text
may this object persist, and in which storage tier/state?
```

The BUILD_PLAN_Z Phase 11 requirements explicitly place this future state machine in `svc-storage`. 

---

## 50. Persistent backend pruning

The current embedded service-node runtime uses the in-memory storage implementation exercised by the live prune test.

Future persistent backends must implement the same exact and truthful contract:

```text
remove only the exact canonical object
return actual removed byte count
return NotFound when absent
preserve neighbors
avoid path traversal
avoid prefix deletion
avoid manifest or pointer deletion unless separately authorized
```

No backend may return `Removed` merely because a delete request was accepted.

---

## 51. Repair and replication coordination

Later phases will add provider repair and replication behavior.

Pruned, globally denied, tombstoned, blocked, or quarantined objects must not be automatically restored by a repair worker without canonical policy approval.

That coordination is not yet implemented in `svc-storage`.

---

# Part XII — Final Crate State

## 52. Final completed behavior

At the end of BUILD_PLAN_Z Phase 10, `svc-storage` can now answer and enforce:

```text
Is this exact B3 object allowed to be read?
Can a denied read be stopped before any storage access?
Can OAP, legacy GET, and legacy HEAD share one policy snapshot?
Can a configured policy be loaded with strict size and JSON limits?
Can one exact local object be physically removed?
Can repeated prune remain truthful and idempotent?
```

The answer to each of these is now yes.

---

## 53. Final status label

Use this status in future notes:

```text
svc-storage BUILD_PLAN_Z Phase 10 work:
COMPLETE / GREEN / PARKED

OAP serve-time moderation:
COMPLETE

Bounded policy loading:
COMPLETE

Legacy GET/HEAD moderation:
COMPLETE

Exact local MemoryStorage pruning:
COMPLETE

Persistent storage state machine:
NOT STARTED — PHASE 11

Network-wide deletion:
NOT IMPLEMENTED / NOT CLAIMED
```

---

## 54. One-line summary

```text
svc-storage now enforces the canonical exact-B3 moderation decision before every active object-read path and provides a truthful, idempotent primitive for deleting one exact object from the live local storage instance—without acquiring policy, network, wallet, ledger, or reward authority.
```

The next crate-specific changelog should cover `svc-dht`, including canonical `crab://node/<node-id>` identity, local provider-store behavior, exact configured-node withdrawal, HTTP/router integration, idempotency, and the limits around network advertisement.


### END NOTE - JULY 11 2026 - 22:50 CST


### BEGIN NOTE - JULY 12 2026 - 14:15 CST

Here are the session-specific changelog notes for **`svc-storage`**.

# `svc-storage` — Session Changelog Notes

## Scope

This session extended `svc-storage` with the canonical local storage and persistence-review foundation required by Phase 11 of `BUILD_PLAN_Z`.

The work added:

* an amnesia-first in-memory object store;
* a shared persistence state machine;
* a bounded process-local persistence catalog;
* policy-gated approval and pinning;
* moderation-state projection;
* truthful metadata-only persistence eligibility;
* focused behavioral tests.

The crate now owns local storage residency and persistence-candidate state, while relying on `ron-policy` for the rules governing approval and pinning.

---

## Added canonical persistence states

A shared persistence state model was added for locally tracked storage candidates.

The supported states are:

```text
ephemeral_unvetted
pending_review
verified_persistent
operator_blocked
global_denied
owner_tombstoned
quarantined
pinned_by_operator
```

These states provide one reusable lifecycle for macronode and future storage callers.

Other crates no longer need to invent their own labels or persistence state transitions.

---

## Defined durable-storage eligibility

The persistence model now distinguishes between:

* being known to the local storage system;
* being eligible for durable storage;
* actually having durable bytes written.

Only these states are durable-storage eligible:

```text
verified_persistent
pinned_by_operator
```

The following states are explicitly non-durable:

```text
ephemeral_unvetted
pending_review
operator_blocked
global_denied
owner_tombstoned
quarantined
```

This session did not add actual durable-byte persistence.

The new model only records eligibility and review posture, preventing metadata state from being misrepresented as completed disk storage.

---

## Added amnesia-first `MemoryStorage`

A bounded process-local memory store was added for ephemeral object residency.

The store uses:

* process-local memory;
* FIFO eviction;
* bounded object count;
* bounded total byte usage;
* exact byte accounting;
* deterministic replacement behavior.

Default capacity is:

```text
4,096 objects
256 MiB total bytes
```

Objects stored in this layer report:

```text
StorageResidency::EphemeralMemory
```

This preserves the project’s amnesia-first design: objects are accepted into bounded memory without silently becoming durable files.

---

## Added bounded-storage enforcement

`MemoryStorage` now rejects objects that cannot fit within the configured maximum byte capacity.

The implementation also evicts older entries when necessary to keep both limits satisfied:

* maximum number of objects;
* maximum total bytes.

This prevents unbounded in-memory growth and gives macronode a predictable storage budget.

---

## Added exact byte accounting

Memory storage tracks the current total bytes held by the store.

The accounting is updated correctly when:

* a new object is inserted;
* an older object is evicted;
* an existing object is replaced;
* an object with different byte length replaces a prior value;
* an oversized object is rejected.

This prevents stale or inflated usage reporting after replacement and eviction.

---

## Added deterministic FIFO replacement behavior

The bounded memory layer maintains insertion order for eviction.

When capacity is exceeded, the oldest eligible object is removed first.

Replacing an existing object also updates its position consistently so future eviction order reflects the replacement operation rather than leaving stale ordering metadata.

Focused tests cover the resulting deterministic behavior.

---

## Added `PersistenceCatalog`

A process-local persistence catalog was added to track candidate metadata independently from ephemeral object bytes.

The catalog uses:

* a `BTreeMap` for deterministic ordering;
* `parking_lot::RwLock` for shared runtime access;
* a bounded pending-review limit;
* canonical B3 object identifiers;
* exact asset categories;
* transactional mutations.

The catalog can be shared by the macronode HTTP runtime and separate `crabnode` CLI processes through the active macronode service.

---

## Added bounded review capacity

The persistence catalog limits the number of candidates that can occupy the review workflow.

The configured maximum is:

```text
1,024 review candidates
```

This prevents an unlimited number of untrusted candidates from accumulating in process memory.

Invalid pending-list limits are rejected rather than silently expanded or treated as unbounded.

---

## Added candidate registration

The catalog now supports registering a canonical object and its exact `AssetKind`.

A newly registered candidate begins in:

```text
ephemeral_unvetted
```

Registration is idempotent.

Registering the same object again:

* does not duplicate the candidate;
* does not reset a prior decision;
* does not replace its original asset category;
* does not claim durable eligibility.

This protects existing review state from being erased by repeated registration.

---

## Added candidate status lookup

The catalog now exposes exact status lookup by canonical B3 object ID.

Status includes the candidate’s:

* object identifier;
* asset kind;
* persistence state;
* durable-storage eligibility.

Missing candidates fail explicitly instead of returning fabricated default state.

---

## Added deterministic pending-review listing

The catalog can list candidates currently awaiting review.

The list is:

* deterministic;
* bounded by the requested limit;
* ordered consistently through the underlying `BTreeMap`;
* limited to undecided review candidates;
* exclusive of approved, rejected, denied, tombstoned, quarantined, and pinned objects.

Invalid or unsafe list limits fail closed.

---

## Added review submission

A registered `ephemeral_unvetted` candidate can be submitted into:

```text
pending_review
```

Submission is idempotent and cannot reset an already decided candidate.

This creates an explicit distinction between:

* a known ephemeral object;
* an object actively awaiting persistence review.

---

## Added policy-gated approval

The catalog now supports persistence approval through the canonical `ron-policy::PersistencePolicy`.

Approval evaluates:

* exact asset category;
* required review threshold;
* effective moderation policy;
* current candidate state.

A successful approval transitions a valid candidate into:

```text
verified_persistent
```

The candidate then becomes eligible for durable storage, but the mutation does not claim that durable bytes have been written.

---

## Added transactional approval behavior

Approval mutations are applied transactionally.

When policy evaluation fails:

* the candidate is not partially advanced;
* pending-review state is not corrupted;
* durable eligibility is not incorrectly enabled;
* no intermediate approval state is committed.

This was covered by a focused regression test proving that failed approval leaves the prior state intact.

---

## Added moderation refusal projection

The persistence state machine now projects canonical moderation outcomes into matching non-durable states.

Examples include:

```text
global deny      → global_denied
owner tombstone  → owner_tombstoned
local block      → operator_blocked
quarantine       → quarantined
```

These projected states remain non-durable regardless of whether:

* the asset category is otherwise allowed;
* the review threshold is satisfied;
* operator pinning is enabled.

This ensures persistence approval cannot override moderation.

---

## Preserved canonical moderation precedence

When multiple moderation rules exist, the strongest canonical result controls persistence state.

The storage crate does not implement an alternative precedence system.

It consumes the effective moderation result produced through the shared policy layer and projects that result into the corresponding persistence state.

This preserves the system-wide ordering:

```text
global deny
> owner tombstone
> local block
> quarantine
> local allow
> no rule
```

---

## Added operator rejection

A candidate in the review workflow can be explicitly rejected by the operator.

Rejection moves the candidate into:

```text
operator_blocked
```

The operation is idempotent.

Repeated rejection does not fabricate additional state changes and does not make the object durable-storage eligible.

---

## Added policy-gated operator pinning

The catalog now supports pinning an approved candidate.

Pinning requires:

* a candidate already in `verified_persistent`;
* an allowed asset category;
* sufficient review posture;
* no effective moderation refusal;
* explicit operator-pinning permission from `PersistencePolicy`.

A successful pin transitions the candidate to:

```text
pinned_by_operator
```

Pinning cannot skip the approval state.

A candidate cannot move directly from `ephemeral_unvetted` or `pending_review` into `pinned_by_operator`.

---

## Added unpin behavior

An operator-pinned candidate can be unpinned.

Unpinning transitions:

```text
pinned_by_operator → verified_persistent
```

The object remains durable-storage eligible because unpinning removes the stronger operator-retention posture without revoking its completed persistence approval.

Unpin is idempotent.

Repeated unpin requests return a truthful no-change result.

---

## Added truthful mutation results

Catalog operations return mutation results that distinguish between:

* an actual state change;
* an idempotent no-op;
* a rejected or invalid transition.

This enables macronode HTTP and CLI surfaces to report real behavior rather than returning generic success strings.

The mutation result includes the resulting candidate state and whether the operation changed it.

---

## Preserved non-economic authority

`svc-storage` gained no authority to:

* create ROC;
* transfer ROC;
* mutate wallets;
* write ledger receipts;
* mint or burn tokens;
* issue rewards;
* claim settlement or finality.

The new persistence workflow is exclusively local storage metadata and state management.

Economic authority remains outside this crate.

---

## Preserved separation from real durable storage

The session established the eligibility and pinning model but did not add a durable file-writing backend.

Therefore:

```text
verified_persistent
```

means the candidate has passed the required local policy and review checks.

```text
pinned_by_operator
```

means the candidate has also received an explicit operator pin decision.

Neither state alone proves that bytes were written to disk.

This distinction is intentionally retained in all API responses and tests.

---

## Focused persistence-state tests

The `persistence_state` integration suite completed with eleven passing tests:

```text
new_object_is_ephemeral_and_not_durable_storage_eligible

persistence_state_and_action_labels_are_stable

persistence_record_json_is_strict_and_defaults_to_ephemeral

only_verified_and_pinned_states_are_durable_storage_eligible

approval_and_pinning_require_canonical_policy_and_remain_idempotent

approval_and_pinning_cannot_skip_required_review_state

category_review_and_pin_refusals_leave_state_unchanged

operator_reject_requires_review_and_is_idempotent

local_block_projects_pending_review_to_operator_blocked

moderation_refusals_project_to_non_durable_storage_states

canonical_moderation_precedence_projects_global_deny
```

These tests prove:

* safe initial state;
* stable labels and serialization;
* exact durable-eligibility rules;
* policy-gated approval and pinning;
* valid transition ordering;
* refusal-state immutability;
* rejection behavior;
* moderation projection;
* canonical precedence.

---

## Focused persistence-catalog tests

The `persistence_catalog` suite completed with eight passing tests:

```text
registration_is_ephemeral_idempotent_and_does_not_reset_category

pending_list_is_deterministic_bounded_and_excludes_decided_objects

approval_uses_canonical_policy_and_remains_idempotent

failed_approval_does_not_partially_commit_pending_review

canonical_moderation_refusal_overrides_approval

rejection_moves_review_candidate_to_operator_blocked

pin_requires_approval_and_unpin_preserves_verified_eligibility

missing_objects_and_invalid_list_limits_fail_closed
```

These tests prove:

* idempotent registration;
* stable asset-category ownership;
* bounded deterministic listing;
* canonical policy usage;
* transactional approval;
* moderation supremacy;
* truthful rejection;
* pin sequencing;
* unpin eligibility behavior;
* fail-closed error handling.

---

## Validation completed

The final persistence-state test run passed:

```text
svc-storage persistence_state
11 passed
0 failed
```

The final catalog test run passed:

```text
svc-storage persistence_catalog
8 passed
0 failed
```

Strict Clippy passed:

```text
cargo clippy \
  -p svc-storage \
  --all-targets \
  --no-deps \
  -- -D warnings
```

The updated crate also passed the workspace compile gate:

```text
cargo check --workspace
```

---

## Result

`svc-storage` now provides the shared local foundation for Phase 11 storage behavior:

* bounded amnesia-first object residency;
* canonical persistence states;
* exact durable-eligibility semantics;
* bounded candidate review;
* transactional policy-gated approval;
* moderation refusal projection;
* explicit operator rejection;
* policy-gated pinning;
* truthful unpinning and idempotency.

The crate is now ready to support future durable storage implementation without confusing eligibility metadata with completed disk persistence.

The `svc-storage` portion of Phase 11 is complete, green, and parked.

The next crate changelog should cover `macronode`.


### END NOTE - JULY 12 2026 - 14:15 CST

### BEGIN NOTE - JULY 12 2026 - 18:30 CST

## svc-storage — Session Changelog

* Added Phase 23 hostile-content and cache-chaos coverage.
* Enforced full BLAKE3 verification before serving stored or provider-fetched bytes.
* Verified global deny and owner tombstone override local allow.
* Confirmed cache eviction returns truthful not-found behavior.
* Rejected oversized objects without evicting valid cached content.
* Completed focused tests, strict Clippy, and acceptance checks.


### END NOTE - JULY 12 2026 - 18:30 CST