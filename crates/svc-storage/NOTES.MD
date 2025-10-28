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