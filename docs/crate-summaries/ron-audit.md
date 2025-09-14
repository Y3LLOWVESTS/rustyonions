---

crate: ron-audit
path: crates/ron-audit
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Tamper-evident audit logging: append signed, hash-chained records (Ed25519 + BLAKE3) with an optional filesystem sink.

## 2) Primary Responsibilities

* Build an **append-only chain** of audit records: `prev_hash → body → hash`, then **sign** the new `hash` with Ed25519.
* Provide a minimal API to **append** semantic events with a timestamp and small JSON payload, and (optionally) **persist** each record to disk.

## 3) Non-Goals

* Not a general logging/metrics framework; no query/index layer.
* No key storage/rotation policy (belongs in `ron-kms`/ops).
* No transport/remote sinks in-core (HTTP/Kafka/S3, etc. should be separate).
* No end-user PII redaction or schema validation (callers must filter/sanitize).

## 4) Public API Surface

* Re-exports: none.
* Key types / functions:

  * `struct Auditor` (in-memory signer + chain head):

    * `fn new() -> Self` — generates a fresh Ed25519 key; `prev_hash = [0;32]`.
    * `fn with_dir(self, dir: impl Into<PathBuf>) -> Self` *(feature = "fs")* — enable per-record persistence to `dir`.
    * `fn verifying_key(&self) -> &VerifyingKey`.
    * `fn append(&mut self, kind: &'static str, data: serde_json::Value) -> Result<AuditRecord>`.
  * `struct AuditBody { ts: i64, kind: &'static str, data: serde_json::Value }`.
  * `struct AuditRecord { prev_hash: [u8;32], hash: [u8;32], sig: Vec<u8>, body: AuditBody }`.
* Events / HTTP / CLI: none in this crate.

## 5) Dependencies & Coupling

* Internal crates → none at runtime; **loose** coupling to the wider system (any service can use it). Replaceable: **yes** (API is small).
* External crates (top):

  * `ed25519-dalek` — signatures; mature; constant-time ops; keep pin current.
  * `blake3` — fast hash for chaining; mature.
  * `serde`/`rmp-serde` — deterministic encoding for chaining; low risk.
  * `time` — UTC timestamps.
  * `anyhow` — error aggregation (could be replaced with a custom error).
  * `serde_json` — event payload type.
* Runtime services: OS filesystem *(only with `fs` feature)*; otherwise memory-only.

## 6) Config & Feature Flags

* Features:

  * `fs` (off by default): enable persistence (`with_dir`, per-record `.bin` files); writes `rmp-serde` bytes.
* Env/config structs: none here (callers choose directory path when enabling `fs`).

## 7) Observability

* The crate itself does not emit metrics/logs.
* Recommended: callers increment counters like `audit_append_total{kind}` / `audit_write_fail_total` and export `audit_chain_head` (short b3 hash) as an info metric.

## 8) Concurrency Model

* `append(&mut self, …)` mutates internal `prev_hash`; API is **single-writer** by design.
* For multi-threaded apps, wrap `Auditor` in `Arc<Mutex<…>>` or provide a dedicated single-threaded task receiving messages via mpsc.
* No built-in backpressure or batching; each append is immediate.

## 9) Persistence & Data Model

* **Chain model:** `hash_i = BLAKE3(prev_hash_{i-1} || rmp(body_i))`; `sig_i = Ed25519(hash_i)`.
* **On-disk (fs feature):** each record saved to `<dir>/<ts>-<kind>.bin` (rmp-encoded `AuditRecord`); no directory partitioning or index; no fsync/atomic rename semantics today.
* **Retention:** not provided; callers must rotate/prune/export.

## 10) Errors & Security

* Error taxonomy: `anyhow::Error` for append/write failures (I/O errors, serialization issues). All are **terminal** for the attempted append.
* Security posture:

  * **Integrity:** hash chain detects record reordering/removal/insertion.
  * **Authenticity:** Ed25519 signatures with process-held key.
  * **Confidentiality:** none—payload is cleartext; do not store secrets/PII unless policy allows.
  * **Key handling:** signing key kept in memory (not zeroized on drop); no rotation API; no KMS integration yet.
  * **PQ-readiness:** not PQ-safe; consider Dilithium family later (or store PQ co-signatures).

## 11) Performance Notes

* Per-append work: one BLAKE3 over small buffers + one Ed25519 sign; both are fast.
* Hot path bottleneck under `fs`: synchronous `std::fs::write` (alloc + copy + write). For high rates, use a buffered writer task, batched files, or WAL-style segments.
* Body serialization uses `rmp-serde` once per record; keep `data` small.

## 12) Tests

* Present: chain continuity (`hash_n == prev_hash_{n+1}`).
* Recommended:

  * **Tamper tests:** modify any byte in `body`/`prev_hash` → verification must fail (requires adding a `verify_chain` API).
  * **Signature checks:** verify `sig` with `verifying_key`.
  * **FS roundtrip:** with `fs` feature: write two records, reload files, verify continuity & sigs.
  * **Clock sanity:** ensure `ts` monotonic non-decreasing (document expectation; wall-clock jumps are possible).
  * **Property tests:** randomized small JSON bodies; ensure stable hash across runs.

## 13) Improvement Opportunities

* **Verification API:** add `fn verify_chain(records: impl Iterator<Item=AuditRecord>, vk: &VerifyingKey) -> Result<()>` and a streaming verifier.
* **Key management:** `Auditor::from_signing_key(sk)`; `rotate_keys(new_sk)` that emits a signed **checkpoint** record containing the new verifying key.
* **Zeroization:** add `zeroize` and implement `Drop` for `Auditor` to wipe the signing key (or keep in a KMS).
* **Atomic writes:** write to temp file then `rename` + optional `fsync`; batch to segment files (`YYYY/MM/DD/HH/*.bin`) and write periodic **checkpoint** files (chain head + VK).
* **Remote sinks:** trait `AuditSink` (fs, stdout, http) implemented out-of-crate; provide `AuditWriter` task with bounded channel and backpressure metrics.
* **Schema/versioning:** add `version: u8` in `AuditRecord` to allow future field evolution.
* **Compression:** optional zstd for large JSON payloads (feature-gated).
* **Privacy guard:** opt-in redaction hooks; “amnesia mode” that drops `data` and stores only minimal fields for sensitive events.
* **Docs:** include recommended event kinds (`auth-fail`, `admin-op`, `key-rotated`, `quota-breach`) and example dashboards.

## 14) Change Log (recent)

* 2025-09-14 — Initial draft: Ed25519-signed, BLAKE3-chained records; optional `fs` persistence; basic chaining test.

## 15) Readiness Score (0–5 each)

* API clarity: **4** (small, obvious; lacks verify/rotation)
* Test coverage: **2** (needs verification & fs tests)
* Observability: **2** (none built-in; easy to add counters/hooks)
* Config hygiene: **5** (single optional feature)
* Security posture: **3** (good integrity/auth; key lifecycle & zeroization missing; no PQ)
* Performance confidence: **4** (crypto fast; fs could bottleneck without buffering)
* Coupling (lower is better): **1** (standalone; only standard crypto/serde deps)

